//! 块解析（MixedContentElements 产生式）—— 对应 `parse_block` / `parse_block_impl`。
//!
//! `parse_block_impl` 是递归下降主循环：扫描文本段 → 遇标签 → 分派指令/处理结束标签。

use super::grammar_element_helpers::newline_count;
use super::grammar_helpers::{
    number_literal_only, parse_legacy_number_format, tok_desc, BlockStop,
};
use super::{
    Parser, END_TAG_NAMES, MIXED_CONTENT_PATTERNS, NOPARAM_DIRECTIVES, ROOT_MIXED_PATTERNS,
    SELF_CLOSE_DIRECTIVES,
};
use crate::core::{Element, ElementKind};
use crate::error::{Result, TemplateError};
use crate::parser::lexer::{
    ExprCtx, TagOpen, TagSyntax, TextStop, Tok, DIRECTIVE_NAMES, PARAM_DIRECTIVES,
};
use crate::span::Span;

impl<'a> Parser<'a> {
    pub(crate) fn parse_block(
        &mut self,
        end_tags: &[&str],
        dir_terms: &[&str],
    ) -> Result<(Vec<Element>, BlockStop)> {
        self.parse_block_impl(end_tags, dir_terms, false)
    }

    /// `<#sep>` 就地元素专用：Java Sep() 的 MixedContentElements（FTL.jj 2974-2995）——
    /// `<END_SEP>` 可选，除显式 `</#sep>` 外，任意父块结束标签 / `else` 类终止指令
    /// 都自动收尾（`<#list xs>${x}<#sep>, </#list>`、`<#sep>, </#items>`、
    /// `<#sep>, <#else>Empty</#list>` 三种形态，list3/list-bis 用例）
    pub(crate) fn parse_sep_block(&mut self) -> Result<(Vec<Element>, BlockStop)> {
        self.parse_block_impl(&["sep"], &[], true)
    }

    /// Java ParseException 对 EOF 的统一消息（ParseException.getOrRenderDescription
    /// :384-392，jar 实测基线）：`Unexpected end of file reached. You have an unclosed
    /// {descs}. Check if the FreeMarker end-tags are present, and aren't malformed.
    /// (Note that FreeMarker end-tags must have # or @ after the / character.)`
    /// descs 为空时省略 " You have an unclosed..." 段（Java :387-391 同款分支）。
    /// 位置取输入末尾：JavaCC EOF token 的 beginColumn = 最后字符所在列（实测列=模板
    /// 长度），而 lexer.line_col() 停在不消费字符的 EOF 处（最后字符+1 列）→ 减 1。
    pub(crate) fn eof_unclosed(&self, descs: &[&str]) -> TemplateError {
        let (el, ec) = self.lexer.line_col();
        let details = if descs.is_empty() {
            "Unexpected end of file reached.".to_string()
        } else {
            format!(
                "Unexpected end of file reached. You have an unclosed {}. Check if the FreeMarker end-tags are present, and aren't malformed. (Note that FreeMarker end-tags must have # or @ after the / character.)",
                descs.join(" and ")
            )
        };
        self.err(el, ec.saturating_sub(1).max(1), details)
    }

    /// 结束标签名 → Java getEndTokenDescIfIsEndToken 描述（END_xxx token → desc，
    /// ParseException.java :499-577；END_MACRO/END_FUNCTION 共享 "#macro or #function"、
    /// END_ASSIGN/END_GLOBAL/END_LOCAL 共享 "#assign or #local or #global"），
    /// 去重保序（Java LinkedHashSet :499-500）
    /// Java UNKNOWN_DIRECTIVE 的 tip 段（FTL.jj :1147-1167）：相近指令名提示或
    /// Help 链接+版本号；无 null 情形（Java 仅在 dn 是内置名但标签畸形时用另一消息）
    pub(crate) fn unknown_directive_tip(name: &str) -> &'static str {
        match name {
            "set" | "var" => {
                "Use #assign or #local or #global, depending on the intented scope \
                 (#assign is template-scope). (If you have seen this directive in use \
                 elsewhere, this was a planned directive, so maybe you need to upgrade \
                 FreeMarker.)"
            }
            "else_if" | "elif" => "Use #elseif.",
            "no_escape" => "Use #noescape instead.",
            "method" => "Use #function instead.",
            "head" | "template" | "fm" => "You may meant #ftl.",
            "try" | "atempt" => "You may meant #attempt.",
            "for" | "each" | "iterate" | "iterator" => {
                "You may meant #list (http://freemarker.org/docs/ref_directive_list.html)."
            }
            "prefix" => {
                "You may meant #import. (If you have seen this directive in use elsewhere, \
                 this was a planned directive, so maybe you need to upgrade FreeMarker.)"
            }
            "item" | "row" | "rows" => "You may meant #items.",
            "separator" | "separate" | "separ" => "You may meant #sep.",
            _ => {
                "Help (latest version): http://freemarker.org/docs/ref_directive_alphaidx.html; \
                 you're using FreeMarker 2.3.34."
            }
        }
    }

    /// Java UNKNOWN_DIRECTIVE（FTL.jj :1128-1172）错误：`Unknown directive: #{name}. {tip}`，
    /// 位置 = beginColumn + 1（`#` 处）
    pub(crate) fn unknown_directive_err(&self, line: u32, col: u32, name: &str) -> TemplateError {
        self.err(
            line,
            col + 1,
            format!(
                "Unknown directive: #{name}. {}",
                Self::unknown_directive_tip(name)
            ),
        )
    }

    /// Java FTL.jj :1135-1143：内置指令名但标签畸形——`</#name>` 缺 `>`（END_xxx
    /// 不匹配，回退 UNKNOWN_DIRECTIVE）或指令无结束标签形态（`</#else>` 等）。
    /// 消息：`#{name} is an existing directive, but the tag is malformed.  (See ...)`；
    /// 位置 = beginColumn + 1（`#` 处，jar 实测 parse_expected_close/parse_bad_close）
    pub(crate) fn malformed_directive_err(&self, line: u32, col: u32, name: &str) -> TemplateError {
        self.err(
            line,
            col + 1,
            format!(
                "#{name} is an existing directive, but the tag is malformed.  \
                 (See FreeMarker Manual / Directive Reference.)"
            ),
        )
    }

    pub(crate) fn end_tag_descs(end_tags: &[&str]) -> Vec<&'static str> {
        let mut out = Vec::new();
        for t in end_tags {
            let d: &'static str = match *t {
                "foreach" => "#foreach",
                "list" => "#list",
                "sep" => "#sep",
                "items" => "#items",
                "switch" => "#switch",
                "if" => "#if",
                "compress" => "#compress",
                "macro" | "function" => "#macro or #function",
                "transform" => "#transform",
                "escape" => "#escape",
                "noescape" => "#noescape",
                "assign" | "global" | "local" => "#assign or #local or #global",
                "attempt" => "#attempt",
                _ => continue,
            };
            if !out.contains(&d) {
                out.push(d);
            }
        }
        out
    }

    pub(crate) fn parse_block_impl(
        &mut self,
        end_tags: &[&str],
        dir_terms: &[&str],
        auto_close: bool,
    ) -> Result<(Vec<Element>, BlockStop)> {
        let mut els: Vec<Element> = Vec::new();
        loop {
            // 嵌套就地元素（#sep）已把外层块的结束标签/终止指令上抛（pending_stop）
            if let Some(stop) = self.pending_stop.take() {
                return Ok((els, stop));
            }
            let (text, stop, line, col) = self.next_text_chunk()?;
            if !text.is_empty() {
                els.push(self.text_element(text, line, col));
            }
            match stop {
                TextStop::Eof => {
                    if auto_close || end_tags.is_empty() {
                        return Ok((els, BlockStop::Eof));
                    }
                    // Java getOrRenderDescription EOF 分支（含 end tag 描述；
                    // 实测 parse_unclosed_tag/parse_macro_no_end 基线）
                    return Err(self.eof_unclosed(&Self::end_tag_descs(end_tags)));
                }
                TextStop::Interp => {
                    els.push(self.parse_interpolation()?);
                }
                TextStop::Tag => {
                    // tag_pos 取标签起始位置（文本段扫描后的当前位置）
                    let (tl, tc) = self.lexer.line_col();
                    self.tag_pos = (tl, tc);
                    let open = self.lexer.read_tag_open();
                    match open {
                        TagOpen::TerseComment { square } => {
                            let (content, l, c) = self.lexer.scan_comment(square)?;
                            els.push(Element::new(
                                ElementKind::Comment { text: content },
                                Span::new(l, c),
                            ));
                        }
                        TagOpen::EndCall { .. } => {
                            // UNIFIED_CALL_END：`</@name>` 或 `</@ns.name>`（点链名，
                            // FTL.jj 1102；`</@>` 无名）。名字按 Java `<ID>` token
                            // 语义（FTL.jj 1392：ID_START_CHAR (ID_START_CHAR|ASCII_DIGIT)*
                            // —— **含数字**，如 `</@m2>`；指令名 token 才限制 [a-zA-Z_]+）
                            let mut name = self.lexer.scan_ident();
                            while self.lexer.peek() == Some('.') {
                                self.lexer.bump();
                                let part = self.lexer.scan_ident();
                                if part.is_empty() {
                                    break;
                                }
                                name.push('.');
                                name.push_str(&part);
                            }
                            // 结束标签的 `>`（UNIFIED_CALL_END 的 CLOSE_TAG1）
                            self.expect_tag_end_raw()?;
                            return Ok((els, BlockStop::EndCall(name)));
                        }
                        TagOpen::EndDir { square } => {
                            self.enter_tag(square);
                            let name = self.lexer.read_name().unwrap_or_default();
                            let lname = name.to_ascii_lowercase();
                            // Java 词法层语义：未知名 `</#foo>` 落 UNKNOWN_DIRECTIVE
                            // （jar 实测 parse_unknown_closing）；无 END token 的指令
                            // `</#else>` 同样落 UNKNOWN_DIRECTIVE → 内置名畸形标签
                            // （jar 实测 parse_bad_close）；END 集合指令 `</#if` 缺 `>`
                            // 时 END_xxx 不匹配 → 畸形标签（FTL.jj :1135-1143，jar 实测
                            // parse_expected_close）。位置 = `#` 处 = 标签起始列 + 1
                            if !DIRECTIVE_NAMES.contains(&lname.as_str()) {
                                return Err(self.unknown_directive_err(line, col, &lname));
                            }
                            if !END_TAG_NAMES.contains(&lname.as_str()) {
                                return Err(self.malformed_directive_err(line, col, &lname));
                            }
                            if self.expect_tag_end_raw().is_err() {
                                return Err(self.malformed_directive_err(line, col, &lname));
                            }
                            if end_tags.iter().any(|e| *e == lname) {
                                return Ok((els, BlockStop::EndTag(lname)));
                            }
                            if auto_close {
                                // Java Sep()：`[LOOKAHEAD(1) end = <END_SEP>]` 可选 ——
                                // 父块结束标签自动收尾，停止信息上抛给外层块
                                return Ok((els, BlockStop::EndTag(lname)));
                            }
                            // JavaCC 嵌套错误格式（ParseException :423-479，jar 实测
                            // parse_nested_comment 基线）：expectedEndTokenDescs 非空时
                            // 附 "only this/these can be closed" + 嵌套提示段，再加
                            // MixedContentElements 全量 expected 列表。根级
                            // （end_tags 空）时 expected 序列无 END token → 普通格式
                            // （无 close 段），列表为根级变体（<EOF> 置首）
                            let patterns = if end_tags.is_empty() {
                                ROOT_MIXED_PATTERNS
                            } else {
                                MIXED_CONTENT_PATTERNS
                            };
                            let mut msg = format!(
                                "Encountered \"</#{name}>\", but was expecting one of these patterns:\n    {}",
                                patterns.join("\n    ")
                            );
                            if !end_tags.is_empty() {
                                let descs = Self::end_tag_descs(end_tags);
                                let close_desc = if descs.len() > 1 {
                                    format!(
                                        "these can be closed: {}",
                                        descs
                                            .iter()
                                            .map(|d| format!("\"{d}\""))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    )
                                } else {
                                    // descs 可能为空（end_tags 名不在
                                    // getEndTokenDescIfIsEndToken 映射，如 "nested"）
                                    // → 兜底用 end_tags[0]
                                    format!(
                                        "this can be closed: \"{}\"",
                                        descs.first().copied().unwrap_or(end_tags[0])
                                    )
                                };
                                msg = format!(
                                    "Encountered \"</#{name}>\", but at this place only {close_desc}. \
                                     This usually because of wrong nesting of FreeMarker directives, \
                                     like a missed or malformed end-tag somewhere. (Note that FreeMarker \
                                     end-tags must have # or @ after the / character.)\n\
                                     Was expecting one of these patterns:\n    {}",
                                    MIXED_CONTENT_PATTERNS.join("\n    ")
                                );
                            }
                            return Err(self.err(line, col, msg));
                        }
                        TagOpen::Call { square } => {
                            self.enter_tag(square);
                            els.push(self.parse_call()?);
                        }
                        TagOpen::Dir { square } => {
                            self.enter_tag(square);
                            let name = self.lexer.read_name().unwrap_or_default();
                            let lname = name.to_ascii_lowercase();
                            // Java 词法层（FTL.jj token 结构，:921-1111）：指令 token
                            // 分四类——BLANK 家族（需参数，名字后必须空白：if/list/
                            // assign 等）、CLOSE_TAG1（无参 `>`/`]` 闭合）、CLOSE_TAG2
                            // （无参，`/` 自闭合可）、双 token（nested/recurse/return/
                            // stop 的 BLANK + SIMPLE 两版）。不匹配 token 结构 → 落
                            // UNKNOWN_DIRECTIVE → 内置名畸形标签（FTL.jj :1135-1143，
                            // jar 实测 parse_malformed_assign）；未知名报 Unknown
                            // directive。位置 = beginColumn + 1（`#` 处）
                            if !DIRECTIVE_NAMES.contains(&lname.as_str()) {
                                return Err(self.unknown_directive_err(line, col, &lname));
                            }
                            let after = self.lexer.peek();
                            let whitespace = matches!(
                                after,
                                Some(c) if c == ' ' || c == '\t' || c == '\r' || c == '\n'
                            );
                            let direct_close = matches!(after, Some('>') | Some(']'));
                            let self_close = after == Some('/');
                            let malformed = if whitespace {
                                // 无参家族带参数 → token 不匹配（`<#compress x>`）；
                                // else 特例：`<#else x>` 走 ELSE_IF（BLANK）
                                !PARAM_DIRECTIVES.contains(&lname.as_str()) && lname != "else"
                            } else if direct_close {
                                // BLANK-only 指令无空白直接闭合 → 不匹配
                                !NOPARAM_DIRECTIVES.contains(&lname.as_str())
                            } else if self_close {
                                // CLOSE_TAG1 家族不含 `/`（`<#compress/>` 畸形）
                                !SELF_CLOSE_DIRECTIVES.contains(&lname.as_str())
                            } else {
                                // EOF 或其他字符 → token 不完整 → 畸形
                                true
                            };
                            if malformed {
                                return Err(self.malformed_directive_err(line, col, &lname));
                            }
                            if dir_terms.iter().any(|t| *t == lname) {
                                return Ok((els, BlockStop::Dir(lname)));
                            }
                            if auto_close
                                && matches!(
                                    lname.as_str(),
                                    "else" | "elseif" | "case" | "default" | "recover"
                                )
                            {
                                // Java MixedContentElements：else/elseif/case/default/recover
                                // 不是元素产生式（FreemarkerDirective 之外）→ 终止本块
                                return Ok((els, BlockStop::Dir(lname)));
                            }
                            els.push(self.dispatch_directive(
                                &lname,
                                self.tag_pos.0,
                                self.tag_pos.1,
                            )?);
                            // 嵌套就地元素（#sep）可能把外层块停止信号放进 pending_stop，
                            // 下一轮循环顶部取出返回
                        }
                    }
                }
            }
        }
    }

    /// 扫描一段模板文本；处理标签语法不一致（`[` vs 已确立的 Angle 语法 → 文本）
    pub(crate) fn next_text_chunk(&mut self) -> Result<(String, TextStop, u32, u32)> {
        let (line, col) = self.lexer.line_col();
        let mut acc = String::new();
        loop {
            let (text, stop) = self.lexer.scan_text_chunk()?;
            acc.push_str(&text);
            if stop != TextStop::Tag {
                return Ok((acc, stop, line, col));
            }
            // 已确立语法与标签开头不一致 → 按文本输出（Java UNKNOWN_DIRECTIVE 的
            // STATIC_TEXT_NON_WS 分支；docs/03 §2.3 规则 1/3）
            let mismatch = matches!(
                (self.lexer.peek(), self.lexer.tag_syntax),
                (Some('['), Some(TagSyntax::Angle)) | (Some('<'), Some(TagSyntax::Square))
            );
            if mismatch {
                acc.push(self.lexer.bump().unwrap());
                continue;
            }
            return Ok((acc, TextStop::Tag, line, col));
        }
    }

    pub(crate) fn text_element(&self, text: String, line: u32, col: u32) -> Element {
        Element::new(
            ElementKind::Text {
                orig_end_line: line + newline_count(&text),
                text,
                strip_before: false,
                strip_after: false,
            },
            Span::new(line, col),
        )
    }

    /// `${expr}` / `#{expr[ ; mNMN]}` 插值（StringOutput / NumericalOutput；契约上
    /// 两者坍缩为 Interpolation，旧式 `#{}` 携带小数位格式信息）
    pub(crate) fn parse_interpolation(&mut self) -> Result<Element> {
        let (line, col) = self.lexer.line_col();
        // 消费 `${` 或 `#{`（scan_text_chunk 已保证）
        let c = self.lexer.bump().unwrap();
        debug_assert!(c == '$' || c == '#');
        if self.lexer.bump() != Some('{') {
            return Err(self.err(line, col, "Expected \"{\" after the interpolation opening."));
        }
        let legacy = c == '#';
        let prev_ctx = self.ctx;
        self.ctx = ExprCtx::Interp;
        let e = self.expression();
        let e = match e {
            Ok(e) => e,
            Err(err) => {
                self.ctx = prev_ctx;
                return Err(err);
            }
        };
        // 旧式 `#{...}`（Java NumericalOutput，FTL.jj 2627-2703）：`; fmt` 格式串 +
        // numberLiteralOnly 字面量校验
        let mut legacy_fmt: Option<(u32, u32)> = None;
        let r = (|| {
            if legacy {
                number_literal_only(self, &e)?;
                if self.peek_tok()?.0 == Tok::Semicolon {
                    self.next_tok()?;
                    let (t, l, c2) = self.next_tok()?;
                    let Tok::Ident(fmt) = t else {
                        // JavaCC：`[ <SEMICOLON> fmt = <ID> ]` —— 非 ID → 词法错误，
                        // 消息逐字（JavaCC 引号原始 token 图像，ProbeP2 实测
                        // `Encountered "12", but was expecting pattern: <ID>`）
                        let found = match &t {
                            Tok::Number(raw) => format!("\"{raw}\""),
                            other => tok_desc(other),
                        };
                        return Err(self.err(
                            l,
                            c2,
                            format!("Encountered {found}, but was expecting pattern: <ID>"),
                        ));
                    };
                    legacy_fmt = Some(parse_legacy_number_format(self, &fmt, l, c2)?);
                } else {
                    // 无格式串：Java NumericalOutput(exp, autoEscOF) → (0, 50)
                    legacy_fmt = Some((0, 50));
                }
            }
            let (t, l, c2) = self.next_tok()?;
            if t != Tok::InterpEnd {
                if t == Tok::Eof {
                    // Java：EOF 时表达式状态期望 CLOSING_CURLY_BRACKET（FTL.jj），
                    // desc = "\"{\""；实测 parse_unclosed_interpolation 基线
                    return Err(self.eof_unclosed(&["\"{\""]));
                }
                return Err(self.err(
                    l,
                    c2,
                    format!(
                        "Expected \"}}\" to close the interpolation, but found {}.",
                        tok_desc(&t)
                    ),
                ));
            }
            Ok(())
        })();
        self.ctx = prev_ctx;
        r?;
        Ok(Element::new(
            ElementKind::Interpolation {
                expr: e,
                legacy_min_frac: legacy_fmt.map(|f| f.0),
                legacy_max_frac: legacy_fmt.map(|f| f.1),
            },
            Span::new(line, col),
        ))
    }
}
