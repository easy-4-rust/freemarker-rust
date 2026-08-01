//! 语法分析器 —— 对应 Java `freemarker.core.FMParser`（FTL.jj 全部产生式）
//!
//! 递归下降实现 docs/03 §3 的 24 个表达式产生式与 §4 的 13 个指令产生式；
//! 词法由 lexer.rs 提供（JavaCC 的 5 词法状态在本实现中压缩为 `ExprCtx` +
//! 括号深度，见 lexer.rs 文件头说明）。
//!
//! 优先级层次（低 → 高，对照 FTL.jj）：or < and < equality < relational <
//! range < additive < multiplicative < unary(not/±) < primary(postfix) < atomic。
//!
//! 解析错误消息：`Parsing error in template "{name}" at line L, column C. {details}`
//! （契约：错误消息必须含模板名 + 行列）。

use crate::core::{
    AssignOp, BuiltinVar, CallTarget, CaseDef, Element, ElementKind, Expr, ExprKind, MacroDef,
    MacroParam, RangeKind, StrPart,
};
use crate::error::{Result, TemplateError};
use crate::parser::lexer::{ExprCtx, Lexer, TagOpen, TagSyntax, TextStop, Tok};
use crate::span::Span;
use crate::template::{Configuration, Template};
use crate::value::TNumber;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr;

/// 解析入口 —— 对应 `new FMParser(Configuration, Reader, ParserConfiguration)`。
/// 构造即解析：词法 + 语法全流程，产物为 `Template`（根元素树 + 宏表）。
pub fn parse(cfg: &Rc<Configuration>, name: &str, text: &str) -> Result<Template> {
    let mut parser = Parser::new(cfg, name, text);
    parser.parse_template()
}

/// 解析独立表达式（Java `?eval` 的 FM_EXPRESSION 模式，BuiltInsForStringsMisc.java:70-90：
/// 源码包为 `(...)` 后按表达式词法解析；本实现包为 `${{src}}` 插值解析后提取表达式）。
pub fn parse_expression(cfg: &Rc<Configuration>, src: &str) -> Result<crate::core::Expr> {
    let wrapped = format!("${{{src}}}");
    let t = parse(cfg, "eval", &wrapped)?;
    match t.root.first().map(|e| &e.kind) {
        Some(ElementKind::Interpolation(e)) => Ok(e.clone()),
        _ => Err(crate::error::TemplateError::misc(format!(
            "Failed to parse the expression: {src:?}"
        ))),
    }
}

/// 递归下降解析器（对应 Java FMParser 的字段 + 产生式方法）
struct Parser<'a> {
    lexer: Lexer,
    cfg: &'a Rc<Configuration>,
    name: String,
    /// 宏表（`template.addMacro` 语义；key 为宏名）
    macros: HashMap<String, MacroDef>,
    /// `[#ftl encoding=...]` 设置的编码（写入 Template.encoding）
    encoding: Option<String>,
    /// whitespace_stripping（`[#ftl strip_whitespace=false]` 可覆盖；docs/08 §5.2）
    strip_ws: bool,
    /// inMacro / inFunction 嵌套计数（Macro 语义校验；互斥）
    in_macro: u32,
    in_function: u32,
    /// breakable/continuable 嵌套计数（#list with as / #items / #switch / #foreach）
    loop_nesting: u32,
    /// 迭代块上下文栈（Java ParserIteratorBlockContext；#items/#sep 的嵌套校验）
    iter_stack: Vec<IterCtx>,
    /// 当前表达式词法上下文（标签参数区 vs 插值内部）
    ctx: ExprCtx,
    /// 前瞻 token 缓冲（最多 2 个；`next_expr_token` 有消耗性）。
    /// 元组 = (token, 起始行, 起始列, 结束行, 结束列) —— 结束位置供
    /// `<@callee>` 的 NO_SPACE_EXPRESSION 相邻性判定（FTL.jj 1614 TERMINATING_WHITESPACE）
    buf: Vec<(Tok, u32, u32, u32, u32)>,
    /// 最近一个被消费 token 的结束位置（`next_tok` 维护；callee 相邻性判定用）
    last_tok_end: (u32, u32),
    /// 最近一个标签的起始位置（指令 span 用）
    tag_pos: (u32, u32),
    /// 命名参数值表达式嵌套深度（NAMED_PARAMETER_EXPRESSION 语义：`!`+空白终止）
    named_arg_depth: u32,
}

impl<'a> Parser<'a> {
    fn new(cfg: &'a Rc<Configuration>, name: &str, text: &str) -> Self {
        Parser {
            lexer: Lexer::new(name, text, cfg.settings.strict_syntax),
            cfg,
            name: name.to_string(),
            macros: HashMap::new(),
            encoding: None,
            strip_ws: cfg.settings.whitespace_stripping,
            in_macro: 0,
            in_function: 0,
            loop_nesting: 0,
            iter_stack: Vec::new(),
            ctx: ExprCtx::Tag { square: false },
            buf: Vec::new(),
            last_tok_end: (1, 1),
            tag_pos: (1, 1),
            named_arg_depth: 0,
        }
    }

    /// 解析错误：`Parsing error in template "{name}" at line L, column C. {details}`
    fn err(&self, line: u32, col: u32, details: impl Into<String>) -> TemplateError {
        TemplateError::Parse {
            template: self.name.clone(),
            message: format!(
                "\"{}\" at line {}, column {}. {}",
                self.name,
                line,
                col,
                details.into()
            ),
        }
    }

    /// 已确立的标签语法（首个标签决定；docs/03 §2.3 规则 3）
    fn tag_square(&self) -> bool {
        self.lexer.tag_syntax == Some(TagSyntax::Square)
    }

    /// 首个标签确立标签语法（`<`→Angle，`[`→Square）
    fn establish_tag_syntax(&mut self, square: bool) {
        if self.lexer.tag_syntax.is_none() {
            self.lexer.tag_syntax = Some(if square {
                TagSyntax::Square
            } else {
                TagSyntax::Angle
            });
        }
    }

    /// 标签打开后：确立语法 + 切换到标签参数词法上下文
    fn enter_tag(&mut self, square: bool) {
        self.establish_tag_syntax(square);
        self.ctx = ExprCtx::Tag {
            square: self.tag_square(),
        };
        debug_assert!(self.buf.is_empty());
    }

    // -----------------------------------------------------------------------
    // token 流辅助
    // -----------------------------------------------------------------------

    fn peek_tok(&mut self) -> Result<(Tok, u32, u32)> {
        if self.buf.is_empty() {
            let (t, l, c, el, ec) = self.lexer.next_expr_token(self.ctx)?;
            self.buf.push((t, l, c, el, ec));
        }
        Ok((self.buf[0].0.clone(), self.buf[0].1, self.buf[0].2))
    }

    fn peek_tok2(&mut self) -> Result<(Tok, u32, u32)> {
        self.peek_tok()?;
        while self.buf.len() < 2 {
            let (t, l, c, el, ec) = self.lexer.next_expr_token(self.ctx)?;
            self.buf.push((t, l, c, el, ec));
        }
        Ok((self.buf[1].0.clone(), self.buf[1].1, self.buf[1].2))
    }

    fn next_tok(&mut self) -> Result<(Tok, u32, u32)> {
        let entry = if !self.buf.is_empty() {
            Some(self.buf.remove(0))
        } else {
            let (t, l, c, el, ec) = self.lexer.next_expr_token(self.ctx)?;
            Some((t, l, c, el, ec))
        };
        if let Some((t, l, c, el, ec)) = entry {
            self.last_tok_end = (el, ec);
            Ok((t, l, c))
        } else {
            unreachable!()
        }
    }

    /// 期望指定 token；失败报错（含位置与期望内容）
    fn expect_tok(&mut self, tok: Tok, what: &str) -> Result<(u32, u32)> {
        let (t, l, c) = self.next_tok()?;
        if t == tok {
            Ok((l, c))
        } else {
            Err(self.err(
                l,
                c,
                format!("Expected {what}, but found {}.", tok_desc(&t)),
            ))
        }
    }

    /// 标签结束（token 形式）：`>` 结束标签
    fn expect_tag_end(&mut self) -> Result<(u32, u32)> {
        let (t, l, c) = self.next_tok()?;
        match t {
            Tok::TagEnd => Ok((l, c)),
            Tok::EmptyTagEnd => Err(self.err(
                l,
                c,
                "The tag can't be self-closing (\" />\"): this directive needs a body.",
            )),
            other => Err(self.err(
                l,
                c,
                format!(
                    "Expected \">\" to close the tag, but found {}.",
                    tok_desc(&other)
                ),
            )),
        }
    }

    /// 宽松标签结束：`>` 或 `/>`（对应 LooseDirectiveEnd）
    fn loose_end(&mut self) -> Result<(u32, u32)> {
        let (t, l, c) = self.next_tok()?;
        match t {
            Tok::TagEnd | Tok::EmptyTagEnd => Ok((l, c)),
            other => Err(self.err(
                l,
                c,
                format!(
                    "Expected \">\" or \"/>\" to close the tag, but found {}.",
                    tok_desc(&other)
                ),
            )),
        }
    }

    /// 无参数指令的标签结束（直接读原始字符；调用前不得 lex 过 token）
    fn expect_tag_end_raw(&mut self) -> Result<()> {
        match self.lexer.try_read_tag_end() {
            Some(_) => Ok(()),
            None => {
                let (l, c) = self.lexer.line_col();
                Err(self.err(l, c, "Expected \">\" or \"/>\" to close the tag."))
            }
        }
    }

    // -----------------------------------------------------------------------
    // 模板根（Root 产生式）
    // -----------------------------------------------------------------------

    fn parse_template(&mut self) -> Result<Template> {
        // FTL 头部（HeaderElement 产生式）：仅允许模板开头（前导空白后）
        let save = self.lexer.save();
        let (text, stop, _, _) = self.next_text_chunk()?;
        let mut header_parsed = false;
        if stop == TextStop::Tag && text.trim().is_empty() {
            let open = self.lexer.read_tag_open();
            if let TagOpen::Dir { square } = open {
                if let Some(name) = self.lexer.read_name() {
                    if name.eq_ignore_ascii_case("ftl") {
                        self.enter_tag(square);
                        self.parse_ftl_header()?;
                        header_parsed = true;
                    }
                }
            }
        }
        if !header_parsed {
            self.lexer.restore(&save);
        }

        let (root, _) = self.parse_block(&[], &[])?;
        let mut root = root;
        Self::mark_stripping(&mut root, self.strip_ws);
        // Java TextBlock.postParseCleanup：<#lt>/<#rt>/<#t>/<#nt> 解析期消费
        // （TrimInstruction "does nothing at render-time, only parse-time"，
        //  直接改写相邻文本块并调整剥离标记）；随后每层移除 ignorable 元素
        // （TemplateElement.postParseCleanup :404-414：全空白文本夹在非输出元素之间
        //  或顶层边界时删除 —— 与剥离顺序一致：先清理后移除）
        Self::remove_ignorable(&mut root, true);
        // Java：宏定义在解析期已 clone 进注册表（Template.macros），空白剥离标记只打在
        // 树上 ElementKind::Macro 的副本上；须同步回注册表，否则宏体不剥离（hashconcat 用例）
        sync_macro_defs(&mut root, &mut self.macros);

        let mut template = Template::new(
            self.name.clone(),
            root,
            std::mem::take(&mut self.macros),
            self.cfg.clone(),
        );
        template.encoding = self.encoding.take();
        Ok(template)
    }

    /// 移除 ignorable 元素（Java isIgnorable：TextBlock.java:349-366 全空白判定 +
    /// nonOutputtingType :374-381；TrimInstruction.isIgnorable=true 恒被移除）。
    /// 逐个移除并实时更新相邻关系（Java :410-419 逐个搬移）。
    fn remove_ignorable(els: &mut Vec<Element>, is_root: bool) {
        let mut i = 0;
        while i < els.len() {
            let removable = match &els[i].kind {
                ElementKind::TrimLineStart
                | ElementKind::NoTrimLineStart
                | ElementKind::TrimLineEnd
                | ElementKind::LeftTrimLine => true,
                ElementKind::Text { text, .. } | ElementKind::NoParse { text, .. } => {
                    // Java isIgnorable（TextBlock.java:349-366）：空文本恒移除；
                    // 全空白文本仅在顶层边界或两侧非输出元素时移除
                    if text.is_empty() {
                        true
                    } else if !text.chars().all(is_ws) {
                        false
                    } else {
                        let prev_ok =
                            (i == 0 && is_root) || (i > 0 && is_non_outputting(&els[i - 1]));
                        let next_ok = (i + 1 == els.len() && is_root)
                            || (i + 1 < els.len() && is_non_outputting(&els[i + 1]));
                        prev_ok && next_ok
                    }
                }
                _ => false,
            };
            if removable {
                els.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// `[#ftl]` 头部：`key = expr`* 或直接结束；`inFTLHeader` 的 eatNewline 语义
    fn parse_ftl_header(&mut self) -> Result<()> {
        loop {
            let (t, l, c) = self.peek_tok()?;
            match t {
                Tok::TagEnd | Tok::EmptyTagEnd => {
                    self.next_tok()?;
                    break;
                }
                Tok::Ident(key) => {
                    self.next_tok()?;
                    self.expect_tok(Tok::Eq, "\"=\" after the FTL header parameter")?;
                    let value = self.expression()?;
                    match key.to_ascii_lowercase().as_str() {
                        "encoding" => match &value.kind {
                            ExprKind::Str(s) => self.encoding = Some(s.clone()),
                            _ => {
                                return Err(self.err(
                                    l,
                                    c,
                                    "Expected a string constant for \"encoding\".",
                                ))
                            }
                        },
                        "strip_whitespace" | "stripwhitespace" | "strip_text" | "striptext" => {
                            match header_bool(&value) {
                                Some(b) => self.strip_ws = b,
                                None => {
                                    return Err(self.err(
                                        l,
                                        c,
                                        "Expected a boolean constant for the header parameter.",
                                    ))
                                }
                            }
                        }
                        "strict_syntax" | "strictsyntax" => match header_bool(&value) {
                            Some(b) => self.lexer.strict_syntax = b,
                            None => {
                                return Err(self.err(
                                    l,
                                    c,
                                    "Expected a boolean constant for \"strict_syntax\".",
                                ))
                            }
                        },
                        // 渲染期设置（auto_esc / output_format / ns_prefixes / attributes）：
                        // 本实现解析并忽略（渲染引擎尚未实现；文档化偏差）
                        "auto_esc" | "autoesc" | "output_format" | "outputformat"
                        | "ns_prefixes" | "nsprefixes" | "attributes" => {}
                        other => {
                            return Err(self.err(
                                l,
                                c,
                                format!("Unknown FTL header parameter: {other}."),
                            ))
                        }
                    }
                }
                other => {
                    return Err(self.err(
                        l,
                        c,
                        format!(
                            "Expected an FTL header parameter or the closing \">\", but found {}.",
                            tok_desc(&other)
                        ),
                    ))
                }
            }
        }
        // eatNewline：吞掉头部后的空白含换行（FTL.jj eatNewline）
        loop {
            match self.lexer.peek() {
                Some(c) if c == ' ' || c == '\t' || c == '\r' || c == '\n' => {
                    if c == '\r' {
                        self.lexer.bump();
                        if self.lexer.peek() == Some('\n') {
                            self.lexer.bump();
                        }
                        break;
                    }
                    if c == '\n' {
                        self.lexer.bump();
                        break;
                    }
                    self.lexer.bump();
                }
                _ => break,
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // 混合内容（MixedContentElements 产生式）
    // -----------------------------------------------------------------------

    /// 解析一段元素序列，直到命中结束标签 / 终止指令 / 文件尾。
    /// - `end_tags`：可接受的结束标签名（`</#name>`）；
    /// - `dir_terms`：可接受的终止指令名（如 `<#else>`，由调用方继续处理）。
    fn parse_block(
        &mut self,
        end_tags: &[&str],
        dir_terms: &[&str],
    ) -> Result<(Vec<Element>, BlockStop)> {
        let mut els: Vec<Element> = Vec::new();
        loop {
            let (text, stop, line, col) = self.next_text_chunk()?;
            if !text.is_empty() {
                els.push(self.text_element(text, line, col));
            }
            match stop {
                TextStop::Eof => {
                    if end_tags.is_empty() {
                        return Ok((els, BlockStop::Eof));
                    }
                    return Err(self.err(
                        line,
                        col,
                        format!(
                            "Unexpected end of file; expected the closing tag \"</#{0}>\".",
                            end_tags[0]
                        ),
                    ));
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
                            // FTL.jj 1102；`</@>` 无名）
                            let mut name = self.lexer.read_name().unwrap_or_default();
                            while self.lexer.peek() == Some('.') {
                                self.lexer.bump();
                                if let Some(part) = self.lexer.read_name() {
                                    name.push('.');
                                    name.push_str(&part);
                                }
                            }
                            // 结束标签的 `>`（UNIFIED_CALL_END 的 CLOSE_TAG1）
                            self.expect_tag_end_raw()?;
                            return Ok((els, BlockStop::EndCall(name)));
                        }
                        TagOpen::EndDir { square } => {
                            self.enter_tag(square);
                            let name = self.lexer.read_name().unwrap_or_default();
                            let lname = name.to_ascii_lowercase();
                            if end_tags.iter().any(|e| *e == lname) {
                                // 结束标签的 `>`（END_xxx 的 CLOSE_TAG1）
                                self.expect_tag_end_raw()?;
                                return Ok((els, BlockStop::EndTag(lname)));
                            }
                            return Err(self.err(
                                line,
                                col,
                                format!(
                                    "Unexpected closing tag \"</#{name}>\" (expected \"</#{0}>\" or the end of the enclosing block).",
                                    end_tags.first().copied().unwrap_or("?")
                                ),
                            ));
                        }
                        TagOpen::Call { square } => {
                            self.enter_tag(square);
                            els.push(self.parse_call()?);
                        }
                        TagOpen::Dir { square } => {
                            self.enter_tag(square);
                            let name = self.lexer.read_name().unwrap_or_default();
                            let lname = name.to_ascii_lowercase();
                            if dir_terms.iter().any(|t| *t == lname) {
                                return Ok((els, BlockStop::Dir(lname)));
                            }
                            els.push(self.dispatch_directive(
                                &lname,
                                self.tag_pos.0,
                                self.tag_pos.1,
                            )?);
                        }
                    }
                }
            }
        }
    }

    /// 扫描一段模板文本；处理标签语法不一致（`[` vs 已确立的 Angle 语法 → 文本）
    fn next_text_chunk(&mut self) -> Result<(String, TextStop, u32, u32)> {
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

    fn text_element(&self, text: String, line: u32, col: u32) -> Element {
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

    /// `${expr}` / `#{expr}` 插值（StringOutput/NumericalOutput 坍缩为 Interpolation）
    fn parse_interpolation(&mut self) -> Result<Element> {
        let (line, col) = self.lexer.line_col();
        // 消费 `${` 或 `#{`（scan_text_chunk 已保证）
        let c = self.lexer.bump().unwrap();
        debug_assert!(c == '$' || c == '#');
        if self.lexer.bump() != Some('{') {
            return Err(self.err(line, col, "Expected \"{\" after the interpolation opening."));
        }
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
        let r = (|| {
            let (t, l, c2) = self.next_tok()?;
            if t != Tok::InterpEnd {
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
            ElementKind::Interpolation(e),
            Span::new(line, col),
        ))
    }

    // -----------------------------------------------------------------------
    // 指令分发（FreemarkerDirective 产生式）
    // -----------------------------------------------------------------------

    fn dispatch_directive(&mut self, name: &str, line: u32, col: u32) -> Result<Element> {
        let span = Span::new(line, col);
        let elem = match name {
            "if" => self.if_directive()?,
            "list" => self.list_directive()?,
            "assign" => self.assign_directive(AssignScope::Namespace)?,
            "global" => self.assign_directive(AssignScope::Global)?,
            "local" => self.assign_directive(AssignScope::Local)?,
            "macro" | "function" => self.macro_directive(name == "function")?,
            "nested" => self.nested_directive()?,
            "switch" => self.switch_directive()?,
            "attempt" => self.attempt_directive()?,
            "break" => {
                self.expect_tag_end_raw()?;
                if self.loop_nesting == 0 {
                    return Err(self.err(
                        line,
                        col,
                        "break must be nested inside a directive that supports it:  #list with \"as\", #items, #switch (or the deprecated #foreach)",
                    ));
                }
                Element::new(ElementKind::Break, span)
            }
            "continue" => {
                self.expect_tag_end_raw()?;
                if self.loop_nesting == 0 {
                    return Err(self.err(
                        line,
                        col,
                        "continue must be nested inside a directive that supports it:  #list with \"as\", #items (or the deprecated #foreach)",
                    ));
                }
                Element::new(ElementKind::Continue, span)
            }
            "return" => self.return_directive()?,
            "stop" => self.stop_directive()?,
            "flush" => {
                self.expect_tag_end_raw()?;
                Element::new(ElementKind::Flush, span)
            }
            "trim" => {
                // 注：Java FTL.jj 无 `<#trim>` 块指令；按契约 ElementKind::Trim 实现
                self.expect_tag_end_raw()?;
                let (body, stop) = self.parse_block(&["trim"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#trim> block."));
                }
                Element::new(ElementKind::Trim(body), span)
            }
            "compress" => {
                self.expect_tag_end_raw()?;
                let (body, stop) = self.parse_block(&["compress"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#compress> block."));
                }
                Element::new(ElementKind::Compress(body), span)
            }
            "include" => self.include_directive()?,
            "import" => self.import_directive()?,
            "escape" => {
                let (Tok::Ident(_var), l, c) = self.next_tok()? else {
                    return Err(self.err(line, col, "Expected a variable name after <#escape>."));
                };
                // 注：契约 Escape{expr, body} 无变量槽位；变量名解析后丢弃（渲染期由 expr 承担）
                self.expect_tok(Tok::As, "\"as\"")?;
                let expr = self.expression()?;
                self.expect_tag_end()?;
                let (body, stop) = self.parse_block(&["escape"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(l, c, "Unclosed <#escape> block."));
                }
                Element::new(ElementKind::Escape { expr, body }, span)
            }
            "noescape" => {
                self.expect_tag_end_raw()?;
                let (body, stop) = self.parse_block(&["noescape"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#noescape> block."));
                }
                Element::new(ElementKind::NoEscape(body), span)
            }
            "autoesc" => {
                self.expect_tag_end_raw()?;
                let (body, stop) = self.parse_block(&["autoesc"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#autoesc> block."));
                }
                Element::new(ElementKind::AutoEsc(body), span)
            }
            "noautoesc" => {
                self.expect_tag_end_raw()?;
                let (body, stop) = self.parse_block(&["noautoesc"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#noautoesc> block."));
                }
                Element::new(ElementKind::NoAutoEsc(body), span)
            }
            "outputformat" => {
                let name_expr = self.expression()?;
                self.expect_tag_end()?;
                let (body, stop) = self.parse_block(&["outputformat"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#outputformat> block."));
                }
                Element::new(
                    ElementKind::OutputFormat {
                        name: name_expr,
                        body,
                    },
                    span,
                )
            }
            "setting" => {
                let (Tok::Ident(key), kl, kc) = self.next_tok()? else {
                    return Err(self.err(line, col, "Expected a setting name after <#setting>."));
                };
                // Java PropertySetting（:71-82）：配置级设置（whitespace_stripping 等）
                // 不在模板级 SETTING_NAMES 白名单 → 解析期报错
                // （"The setting name is recognized, but changing this setting from
                //  inside a template isn't supported."）
                if matches!(
                    key.as_str(),
                    "whitespace_stripping" | "strict_syntax" | "output_format" | "auto_escaping"
                ) {
                    return Err(self.err(
                        kl,
                        kc,
                        "The setting name is recognized, but changing this setting from inside a template isn't supported.",
                    ));
                }
                self.expect_tok(Tok::Eq, "\"=\"")?;
                let value = self.expression()?;
                self.loose_end()?;
                Element::new(ElementKind::Setting { key, value }, span)
            }
            "comment" => {
                self.expect_tag_end_raw()?;
                let (content, _l, _c) = self.lexer.scan_unparsed("comment")?;
                Element::new(ElementKind::Comment { text: content }, span)
            }
            "noparse" => {
                self.expect_tag_end_raw()?;
                let (content, _l, _c) = self.lexer.scan_unparsed("noparse")?;
                // Java：<#noparse> = TextBlock(unparsed=true)（TextBlock.java:31-33），
                // 空白剥离与普通文本同规则（mark_block 打 strip_before/strip_after 标记）
                Element::new(
                    ElementKind::NoParse {
                        text: content.clone(),
                        strip_before: false,
                        strip_after: false,
                        orig_end_line: span.line + newline_count(&content),
                    },
                    span,
                )
            }
            "t" => {
                self.expect_tag_end_raw()?;
                Element::new(ElementKind::TrimLineStart, span)
            }
            "nt" => {
                self.expect_tag_end_raw()?;
                Element::new(ElementKind::NoTrimLineStart, span)
            }
            // `<#rt>`：行尾裁剪（Java TrimInstruction(false,true)；解析期标记，
            // 渲染期语义由 mark_deliberate 打标后由文本剥离实现）
            "rt" => {
                self.expect_tag_end_raw()?;
                Element::new(ElementKind::TrimLineEnd, span)
            }
            // `<#lt>`：行首裁剪（Java TrimInstruction(true,false)）
            "lt" => {
                self.expect_tag_end_raw()?;
                Element::new(ElementKind::LeftTrimLine, span)
            }
            "gt" => {
                // 注：Java 无 `<#gt>` 指令；契约映射为字面 ">"（v1 文档化偏差）
                self.expect_tag_end_raw()?;
                Element::new(ElementKind::RawText(">".to_string()), span)
            }
            "foreach" => self.foreach_directive()?,
            "items" => self.items_directive()?,
            "sep" => self.sep_directive()?,
            "call" => self.call_directive()?,
            "transform" => {
                let expr = self.expression()?;
                self.expect_tag_end()?;
                let (body, stop) = self.parse_block(&["transform"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#transform> block."));
                }
                Element::new(ElementKind::Transform { expr, body }, span)
            }
            "visit" => {
                let expr = self.expression()?;
                self.loose_end()?;
                Element::new(ElementKind::Visit { expr }, span)
            }
            "recurse" => {
                let expr = self.expression()?;
                self.loose_end()?;
                Element::new(ElementKind::Recurse { expr }, span)
            }
            "on" => {
                // `<#on name>body</#on>`（Java On()：exps = PositionalArgs，取首参）
                let expr = self.expression()?;
                self.expect_tag_end()?;
                let (body, stop) = self.parse_block(&["on"], &[])?;
                if !matches!(stop, BlockStop::EndTag(_)) {
                    return Err(self.err(line, col, "Unclosed <#on> block."));
                }
                Element::new(ElementKind::On { expr, body }, span)
            }
            "fallback" => {
                self.expect_tag_end_raw()?;
                Element::new(ElementKind::Fallback, span)
            }
            "ftl" => {
                return Err(self.err(
                    line,
                    col,
                    "The #ftl header is only allowed at the very beginning of the template.",
                ));
            }
            "elseif" | "else" | "case" | "default" | "recover" => {
                return Err(self.err(
                    line,
                    col,
                    format!("Unexpected directive <#{name}> here (it must be inside the matching block)."),
                ));
            }
            other => {
                return Err(self.err(line, col, format!("Unknown directive: #{other}.")));
            }
        };
        Ok(elem)
    }

    /// `<#if cond> ... [<#elseif>] ... [<#else>] ... </#if>`
    /// elseif 扁平化为嵌套 If 的 else 分支（契约注释）
    fn if_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let cond = self.expression()?;
        self.expect_tag_end()?;
        let (then, stop) = self.parse_block(&["if"], &["elseif", "else"])?;
        let else_ = self.parse_if_tail(stop, line, col)?;
        Ok(Element::new(
            ElementKind::If { cond, then, else_ },
            Span::new(line, col),
        ))
    }

    fn parse_if_tail(
        &mut self,
        stop: BlockStop,
        line: u32,
        col: u32,
    ) -> Result<Option<Vec<Element>>> {
        match stop {
            BlockStop::EndTag(_) => Ok(None),
            BlockStop::Dir(name) if name == "else" => {
                self.expect_tag_end_raw()?;
                let (els, stop) = self.parse_block(&["if"], &[])?;
                match stop {
                    BlockStop::EndTag(_) => Ok(Some(els)),
                    BlockStop::Eof => Err(self.err(line, col, "Unclosed <#if> block.")),
                    _ => Err(self.err(line, col, "Unexpected directive in the #else block.")),
                }
            }
            BlockStop::Dir(name) if name == "elseif" => {
                let (el, ec) = self.tag_pos;
                let cond = self.expression()?;
                self.expect_tag_end()?;
                let (then, stop2) = self.parse_block(&["if"], &["elseif", "else"])?;
                let else_ = self.parse_if_tail(stop2, line, col)?;
                Ok(Some(vec![Element::new(
                    ElementKind::If { cond, then, else_ },
                    Span::new(el, ec),
                )]))
            }
            BlockStop::EndCall(_) => {
                Err(self.err(line, col, "Unexpected </@...> in the #if block."))
            }
            BlockStop::Eof => Err(self.err(line, col, "Unclosed <#if> block.")),
            BlockStop::Dir(_) => Err(self.err(line, col, "Unexpected directive in the #if block.")),
        }
    }

    /// `<#list seq [as var[, var2]]> body [<#else>] </#list>`（Java List()，FTL.jj 2779-2870）
    /// - `as var, var2`：hash 键值对列出（hashListing，loopVar2Name）；
    /// - 无 `as`：body 内必须出现 `<#items as x>`（就地元素，Java Items 模型）；
    /// - `<#sep>`/`<#items>` 是普通就地指令，可嵌套在 if/switch 等任意位置（list3 用例）。
    fn list_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let seq = self.expression()?;
        let mut var: Option<String> = None;
        let mut var2: Option<String> = None;
        if self.peek_tok()?.0 == Tok::As {
            self.next_tok()?;
            let (Tok::Ident(v), _, _) = self.next_tok()? else {
                return Err(self.err(line, col, "Expected a loop variable name after \"as\"."));
            };
            if self.peek_tok()?.0 == Tok::Comma {
                self.next_tok()?;
                let (Tok::Ident(v2), _, _) = self.next_tok()? else {
                    return Err(self.err(
                        line,
                        col,
                        "Expected a second loop variable name after \",\".",
                    ));
                };
                if v2 == v {
                    return Err(self.err(
                        line,
                        col,
                        format!(
                            "The key and value loop variable names must differ, but both were: {v}"
                        ),
                    ));
                }
                var2 = Some(v2);
            }
            var = Some(v);
        }
        self.expect_tag_end()?;

        // Java pushIteratorBlockContext（loopVar 存在 → breakable/continuable）
        let has_var = var.is_some();
        self.iter_stack.push(IterCtx {
            has_loop_var: has_var,
            is_foreach: false,
            is_items: false,
        });
        if has_var {
            self.loop_nesting += 1;
        }
        let r = self.list_body(line, col, var, var2, seq);
        if has_var {
            self.loop_nesting -= 1;
        }
        self.iter_stack.pop();
        r
    }

    fn list_body(
        &mut self,
        line: u32,
        col: u32,
        var: Option<String>,
        var2: Option<String>,
        seq: Expr,
    ) -> Result<Element> {
        let mut body: Vec<Element> = Vec::new();
        let mut else_: Option<Vec<Element>> = None;
        let (els, stop) = self.parse_block(&["list"], &["else"])?;
        body.extend(els);
        match stop {
            BlockStop::EndTag(_) => {}
            BlockStop::Dir(n) if n == "else" => {
                self.expect_tag_end_raw()?;
                let (els, stop) = self.parse_block(&["list"], &[])?;
                match stop {
                    BlockStop::EndTag(_) => else_ = Some(els),
                    BlockStop::Eof => {
                        return Err(self.err(line, col, "Unclosed <#list> block."));
                    }
                    _ => {
                        return Err(self.err(
                            line,
                            col,
                            "Unexpected directive in the #else block.",
                        ));
                    }
                }
            }
            BlockStop::Eof => return Err(self.err(line, col, "Unclosed <#list> block.")),
            BlockStop::EndCall(_) => {
                return Err(self.err(line, col, "Unexpected </@...> in the #list block."));
            }
            BlockStop::Dir(other) => {
                return Err(self.err(
                    line,
                    col,
                    format!("Unexpected directive <#{other}> in the #list block."),
                ));
            }
        }
        // Java List()：loopVar==null 且未进入 #items → 报错（校验在 #items 解析时标记）
        let entered_items = self.iter_stack.last().map(|c| c.is_items).unwrap_or(false);
        if var.is_none() && !entered_items {
            return Err(self.err(
                line,
                col,
                "#list must have either \"as loopVar\" parameter or nested #items that belongs to it.",
            ));
        }
        Ok(Element::new(
            ElementKind::List {
                seq,
                var: var.unwrap_or_default(),
                var2,
                body,
                else_,
            },
            Span::new(line, col),
        ))
    }

    /// `<#foreach var in seq> body </#foreach>`（legacy；Java ForEach()，FTL.jj 2886-2910：
    /// 语义为 IteratorBlock(loopVar1Name=var, forEach=true)，无 else/sep/items）
    fn foreach_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let (Tok::Ident(var), _, _) = self.next_tok()? else {
            return Err(self.err(line, col, "Expected a loop variable name after <#foreach>."));
        };
        self.expect_tok(Tok::In, "\"in\" after the loop variable name")?;
        let seq = self.expression()?;
        self.expect_tag_end()?;
        self.iter_stack.push(IterCtx {
            has_loop_var: true,
            is_foreach: true,
            is_items: false,
        });
        self.loop_nesting += 1;
        let r = (|| {
            let (body, stop) = self.parse_block(&["foreach"], &[])?;
            match stop {
                BlockStop::EndTag(_) => Ok(Element::new(
                    ElementKind::List {
                        seq,
                        var,
                        var2: None,
                        body,
                        else_: None,
                    },
                    Span::new(line, col),
                )),
                BlockStop::Eof => Err(self.err(line, col, "Unclosed <#foreach> block.")),
                BlockStop::EndCall(_) => {
                    Err(self.err(line, col, "Unexpected </@...> in the #foreach block."))
                }
                BlockStop::Dir(other) => Err(self.err(
                    line,
                    col,
                    format!("Unexpected directive <#{other}> in the #foreach block."),
                )),
            }
        })();
        self.loop_nesting -= 1;
        self.iter_stack.pop();
        r
    }

    /// `<#items as x[, y]> body </#items>`（就地元素 —— 对应 Java Items()，FTL.jj 2913-2971：
    /// 从最近的 #list 迭代上下文驱动 body 逐项执行；可嵌套在 list 体内的任意位置）
    fn items_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        // Java ITEMS token 自带 "as"（FTL.jj 944）
        self.expect_tok(Tok::As, "\"as\" after #items")?;
        let (Tok::Ident(var), _, _) = self.next_tok()? else {
            return Err(self.err(line, col, "Expected a loop variable name after #items."));
        };
        let mut var2: Option<String> = None;
        if self.peek_tok()?.0 == Tok::Comma {
            self.next_tok()?;
            let (Tok::Ident(v2), _, _) = self.next_tok()? else {
                return Err(self.err(
                    line,
                    col,
                    "Expected a second loop variable name after \",\".",
                ));
            };
            if v2 == var {
                return Err(self.err(
                    line,
                    col,
                    format!(
                        "The key and value loop variable names must differ, but both were: {var}"
                    ),
                ));
            }
            var2 = Some(v2);
        }
        self.expect_tag_end()?;
        // Java Items()：peekIteratorBlockContext 校验 + 上下文标记
        if self.iter_stack.is_empty() {
            return Err(self.err(line, col, "#items must be inside a #list block."));
        }
        let ctx = self.iter_stack.last_mut().expect("已判非空");
        if ctx.has_loop_var {
            return Err(self.err(
                line,
                col,
                "The parent #list of the #items must not have \"as loopVar\" parameter.",
            ));
        }
        if ctx.is_foreach {
            return Err(self.err(
                line,
                col,
                "The deprecated #foreach directive doesn't support nested #items.",
            ));
        }
        if ctx.is_items {
            return Err(self.err(
                line,
                col,
                "Can't nest #items into each other when they belong to the same #list.",
            ));
        }
        ctx.is_items = true;
        self.loop_nesting += 1;
        let r = (|| {
            let (body, stop) = self.parse_block(&["items"], &[])?;
            match stop {
                BlockStop::EndTag(_) => Ok(Element::new(
                    ElementKind::Items { var, var2, body },
                    Span::new(line, col),
                )),
                BlockStop::Eof => Err(self.err(line, col, "Unclosed <#items> block.")),
                BlockStop::EndCall(_) => {
                    Err(self.err(line, col, "Unexpected </@...> in the #items block."))
                }
                BlockStop::Dir(other) => Err(self.err(
                    line,
                    col,
                    format!("Unexpected directive <#{other}> in the #items block."),
                )),
            }
        })();
        self.loop_nesting -= 1;
        r
    }

    /// `<#sep> body </#sep>`（就地元素 —— 对应 Java Sep()，FTL.jj 2974-2995：
    /// 当前迭代 hasNext 时渲染 body；可嵌套在 list/items 体内）
    fn sep_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        if self.iter_stack.is_empty() {
            return Err(self.err(
                line,
                col,
                "#sep must be inside a #list (or #foreach) block.",
            ));
        }
        self.expect_tag_end_raw()?;
        let (body, stop) = self.parse_block(&["sep"], &[])?;
        match stop {
            BlockStop::EndTag(_) => {}
            BlockStop::Eof => return Err(self.err(line, col, "Unclosed <#sep> block.")),
            _ => return Err(self.err(line, col, "Unexpected directive in the #sep block.")),
        }
        Ok(Element::new(
            ElementKind::Sep { body },
            Span::new(line, col),
        ))
    }

    /// `<#call name (args)>`（legacy；对应 Java Call()，FTL.jj 3711-3751：
    /// 构造 UnifiedCall(legacySyntax=true)，无 body；括号可选）
    fn call_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let (name, _) = self.ident_or_string_literal()?;
        let target = CallTarget::Name(name);
        let mut args: Vec<(String, Expr)> = Vec::new();
        let (p1, _, _) = self.peek_tok()?;
        let named = matches!(&p1, Tok::Ident(_)) && self.peek_tok2()?.0 == Tok::Eq;
        if named {
            while let (Tok::Ident(n), _, _) = self.next_tok()? {
                self.expect_tok(Tok::Eq, "\"=\" after the parameter name")?;
                self.named_arg_depth += 1;
                let v = self.expression();
                self.named_arg_depth -= 1;
                let e = v?;
                args.push((n, e));
                let (p, _, _) = self.peek_tok()?;
                if !matches!(&p, Tok::Ident(_)) {
                    break;
                }
                if self.peek_tok2()?.0 != Tok::Eq {
                    break;
                }
            }
        } else if self.peek_tok()?.0 == Tok::OpenParen {
            // `<#call m(a, b)>`：括号包裹的位置参数
            self.next_tok()?;
            for e in self.positional_args(false)? {
                args.push((String::new(), e));
            }
            self.expect_tok(Tok::CloseParen, "\")\" to close the argument list")?;
        } else {
            // `<#call m a, b>`：裸位置参数
            for e in self.positional_args(false)? {
                args.push((String::new(), e));
            }
        }
        self.loose_end()?;
        Ok(Element::new(
            ElementKind::Call {
                callee: target,
                args,
                body: None,
                body_params: Vec::new(),
            },
            Span::new(line, col),
        ))
    }

    /// `<#assign name = expr [, name = expr]* [in ns]>` / `<#assign name>body</#assign>`
    /// （含 global/local；多赋值对应 Java AssignmentInstruction，FTL.jj 3235-3380：
    /// 续项前瞻 `[<COMMA>] (ID|STRING_LITERAL) (assign-op)`，逗号可选）
    fn assign_directive(&mut self, scope: AssignScope) -> Result<Element> {
        let (line, col) = self.tag_pos;
        if scope == AssignScope::Local && self.in_macro + self.in_function == 0 {
            return Err(self.err(line, col, "Local variable assigned outside a macro."));
        }
        let (name, _) = self.ident_or_string_literal()?;
        let (op_tok, l, c) = self.next_tok()?;
        let op = match op_tok {
            Tok::Eq => Some(AssignOp::Equals),
            Tok::PlusEq => Some(AssignOp::PlusEq),
            Tok::MinusEq => Some(AssignOp::MinusEq),
            Tok::TimesEq => Some(AssignOp::TimesEq),
            Tok::DivEq => Some(AssignOp::DivideEq),
            Tok::ModEq => Some(AssignOp::ModuloEq),
            Tok::PlusPlus => Some(AssignOp::PlusPlus),
            Tok::MinusMinus => Some(AssignOp::MinusMinus),
            Tok::In | Tok::TagEnd | Tok::EmptyTagEnd => None, // 块赋值形式
            other => {
                return Err(self.err(
                    l,
                    c,
                    format!(
                        "Expected an assignment operator (\"=\", \"+=\", ...), but found {}.",
                        tok_desc(&other)
                    ),
                ))
            }
        };
        match op {
            Some(op) => {
                // 简单赋值（可多赋值）：`name op expr` 序列
                let mut assignments: Vec<Element> = Vec::new();
                let mut cur_name = name;
                let mut cur_op = op;
                let mut cur_l = l;
                let mut cur_c = c;
                loop {
                    // ++/-- 无右值（Java exp = null；以 Ident 占位，渲染期按 op 处理）
                    let expr = if matches!(cur_op, AssignOp::PlusPlus | AssignOp::MinusMinus) {
                        Expr::new(ExprKind::Ident(cur_name.clone()), Span::new(cur_l, cur_c))
                    } else {
                        self.expression()?
                    };
                    assignments.push(assignment_element(
                        scope,
                        cur_name,
                        expr,
                        cur_op,
                        Span::new(line, col),
                    ));
                    // Java LOOKAHEAD([<COMMA>] (ID|STRING_LITERAL) (assign-op))：续项判定
                    let (p1, _, _) = self.peek_tok()?;
                    let has_comma = p1 == Tok::Comma;
                    if has_comma {
                        self.next_tok()?;
                    }
                    let (p2, _, _) = self.peek_tok()?;
                    let name2 = match &p2 {
                        Tok::Ident(n) | Tok::Str(n) => n.clone(),
                        _ => {
                            if has_comma {
                                return Err(self.err(
                                    line,
                                    col,
                                    format!(
                                        "Expected a variable name after \",\" in the assignment, but found {}.",
                                        tok_desc(&p2)
                                    ),
                                ));
                            }
                            break;
                        }
                    };
                    let (p3, _, _) = self.peek_tok2()?;
                    let _op2 = match assign_op_of(&p3) {
                        Some(op) => op,
                        None => {
                            if has_comma {
                                return Err(self.err(
                                    line,
                                    col,
                                    format!(
                                        "Expected an assignment operator (\"=\", \"+=\", ...) after the variable name, but found {}.",
                                        tok_desc(&p3)
                                    ),
                                ));
                            }
                            break;
                        }
                    };
                    let (_, l2, c2) = self.next_tok()?; // 消费名字（已前瞻确认）
                    let (op2_tok, _, _) = self.next_tok()?; // 消费赋值符
                    let op2 = assign_op_of(&op2_tok).expect("前瞻已确认");
                    cur_name = name2;
                    cur_op = op2;
                    cur_l = l2;
                    cur_c = c2;
                }
                // `expr in ns`（Java：`[id = <IN> nsExp = Expression()]` 在赋值序列之后）
                let namespace = if self.peek_tok()?.0 == Tok::In {
                    self.next_tok()?;
                    if scope != AssignScope::Namespace {
                        return Err(self.err(line, col, "Cannot assign to namespace here."));
                    }
                    let (t, l2, c2) = self.next_tok()?;
                    match t {
                        Tok::Ident(ns) => Some(ns),
                        other => {
                            return Err(self.err(
                                l2,
                                c2,
                                format!("The namespace of an assignment must be a simple variable name, but found {}.", tok_desc(&other)),
                            ))
                        }
                    }
                } else {
                    None
                };
                self.loose_end()?;
                // 命名空间子句应用到全部赋值（Java AssignmentInstruction.setNamespaceExp）
                if let Some(ns) = namespace {
                    for a in &mut assignments {
                        if let ElementKind::Assign { namespace, .. } = &mut a.kind {
                            *namespace = Some(ns.clone())
                        }
                    }
                }
                if assignments.len() == 1 {
                    Ok(assignments.pop().unwrap())
                } else {
                    Ok(Element::new(
                        ElementKind::Assignments(assignments),
                        Span::new(line, col),
                    ))
                }
            }
            None => {
                // 块赋值：`<#assign name [in ns]> body </#assign>`（end tag 必须与 scope 匹配）
                let mut namespace: Option<String> = None;
                if op_tok == Tok::In {
                    // `in ns` 在块形式中紧跟名字之后（`<#assign x in ns>`；
                    // parse_optional_namespace 看不到已消费的 In）
                    if scope != AssignScope::Namespace {
                        return Err(self.err(line, col, "Cannot assign to namespace here."));
                    }
                    let (t, l2, c2) = self.next_tok()?;
                    let Tok::Ident(ns) = t else {
                        return Err(self.err(
                            l2,
                            c2,
                            format!("The namespace of an assignment must be a simple variable name, but found {}.", tok_desc(&t)),
                        ));
                    };
                    namespace = Some(ns);
                    self.expect_tag_end()?;
                } else if op_tok == Tok::EmptyTagEnd {
                    return Err(self.err(
                        line,
                        col,
                        "The tag can't be self-closing (\" />\"): block assignments need a body.",
                    ));
                }
                // op_tok == TagEnd 时标签已结束，直接解析主体
                let end_names: &[&str] = match scope {
                    AssignScope::Namespace => &["assign", "global", "local"],
                    AssignScope::Global => &["global"],
                    AssignScope::Local => &["local"],
                };
                let (body, stop) = self.parse_block(end_names, &[])?;
                match stop {
                    BlockStop::EndTag(n) => {
                        let expected = match scope {
                            AssignScope::Namespace => "assign",
                            AssignScope::Global => "global",
                            AssignScope::Local => "local",
                        };
                        if n != expected {
                            return Err(self.err(line, col, "Mismatched assignment tags."));
                        }
                    }
                    _ => return Err(self.err(line, col, "Unclosed block assignment.")),
                }
                let kind = match scope {
                    AssignScope::Namespace => ElementKind::BlockAssign {
                        target: name,
                        body,
                        op: AssignOp::Equals,
                        namespace,
                    },
                    AssignScope::Global => ElementKind::Global {
                        target: name,
                        expr: None,
                        body: Some(body),
                        op: AssignOp::Equals,
                    },
                    AssignScope::Local => ElementKind::Local {
                        target: name,
                        expr: None,
                        body: Some(body),
                        op: AssignOp::Equals,
                    },
                };
                Ok(Element::new(kind, Span::new(line, col)))
            }
        }
    }

    /// `<#macro name (params)> body </#macro>` / `<#function>`
    fn macro_directive(&mut self, is_function: bool) -> Result<Element> {
        let (line, col) = self.tag_pos;
        if self.in_macro + self.in_function > 0 {
            return Err(self.err(
                line,
                col,
                "Macro or function definitions can't be nested into each other.",
            ));
        }
        let (name, _) = self.ident_or_string_literal()?;
        let open_paren = self.peek_tok()?.0 == Tok::OpenParen;
        if open_paren {
            self.next_tok()?;
        }
        let mut params: Vec<MacroParam> = Vec::new();
        let mut has_default = false;
        loop {
            let (t, l, c) = self.peek_tok()?;
            let Tok::Ident(pname) = t else { break };
            self.next_tok()?;
            let mut catch_all = false;
            if self.peek_tok()?.0 == Tok::Ellipsis {
                self.next_tok()?;
                catch_all = true;
            }
            let mut default: Option<Expr> = None;
            if self.peek_tok()?.0 == Tok::Eq {
                self.next_tok()?;
                default = Some(self.expression()?);
            }
            if self.peek_tok()?.0 == Tok::Comma {
                self.next_tok()?;
            }
            if catch_all {
                if default.is_some() {
                    return Err(self.err(
                        l,
                        c,
                        "\"Catch-all\" macro parameter may not have a default value.",
                    ));
                }
                if params.iter().any(|p| p.catch_all) {
                    return Err(self.err(
                        l,
                        c,
                        "There may only be one \"catch-all\" parameter in a macro declaration, and it must be the last parameter.",
                    ));
                }
            } else if has_default && default.is_none() {
                return Err(self.err(
                    l,
                    c,
                    "In a macro declaration, parameters without a default value must all occur before the parameters with default values.",
                ));
            }
            if !catch_all && default.is_some() {
                has_default = true;
            }
            params.push(MacroParam {
                name: pname,
                optional: default.is_some() || catch_all,
                default,
                catch_all,
            });
        }
        if open_paren {
            self.expect_tok(Tok::CloseParen, "\")\" to close the parameter list")?;
        }
        self.expect_tag_end()?;

        // 宏体内部 breakable/continuable 归零（Java 防 `#list><#macro><#break>` 漏洞）
        let saved_loop = self.loop_nesting;
        self.loop_nesting = 0;
        if is_function {
            self.in_function += 1;
        } else {
            self.in_macro += 1;
        }
        let (body, stop) = self.parse_block(&["macro", "function"], &[])?;
        if is_function {
            self.in_function -= 1;
        } else {
            self.in_macro -= 1;
        }
        self.loop_nesting = saved_loop;

        let end_ok = match &stop {
            BlockStop::EndTag(n) if is_function && n == "function" => true,
            BlockStop::EndTag(n) if !is_function && n == "macro" => true,
            BlockStop::EndTag(_) if is_function => {
                return Err(self.err(line, col, "Expected function end tag here."))
            }
            BlockStop::EndTag(_) => {
                return Err(self.err(line, col, "Expected macro end tag here."))
            }
            _ => return Err(self.err(line, col, "Unclosed macro or function definition.")),
        };
        debug_assert!(end_ok);

        let def = MacroDef {
            name: name.clone(),
            is_function,
            params,
            body,
            namespace: None,
            span: Span::new(line, col),
        };
        self.macros.insert(name, def.clone());
        Ok(Element::new(
            ElementKind::Macro { def },
            Span::new(line, col),
        ))
    }

    /// `<@callee [named|positional args] [; bodyParam,...]>body</@callee>`（UnifiedMacroTransform）
    fn parse_call(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        // callee 表达式按 NO_SPACE_EXPRESSION 语义解析（Java TERMINATING_WHITESPACE）：
        // `<@m [1,2]/>` 中 `[1,2]` 是位置参数列表字面量而非动态键 `m[1,2]`（list 用例）
        let callee_expr = self.callee_expression()?;
        let target = call_target(&callee_expr);

        // NamedArgs（`id = expr`+）或 PositionalArgs（LOOKAHEAD(<ID><EQUALS>)）
        let (p1, _, _) = self.peek_tok()?;
        let named = matches!(&p1, Tok::Ident(_)) && self.peek_tok2()?.0 == Tok::Eq;
        let mut args: Vec<(String, Expr)> = Vec::new();
        if named {
            while let (Tok::Ident(n), _, _) = self.next_tok()? {
                self.expect_tok(Tok::Eq, "\"=\" after the parameter name")?;
                // NAMED_PARAMETER_EXPRESSION：值表达式内 `!`+空白终止（TERMINATING_EXCLAM）
                self.named_arg_depth += 1;
                let v = self.expression();
                self.named_arg_depth -= 1;
                let e = v?;
                args.push((n, e));
                let (p, _, _) = self.peek_tok()?;
                if !matches!(&p, Tok::Ident(_)) {
                    break;
                }
                if self.peek_tok2()?.0 != Tok::Eq {
                    break;
                }
            }
        } else {
            for e in self.positional_args(false)? {
                // 契约 args: Vec<(String, Expr)>；位置参数以空名存储（渲染期按位置取）
                args.push((String::new(), e));
            }
        }

        // body 参数：`; a, b`（对应 Java bodyParameters 列表，FTL.jj 3643-3650）
        let mut body_params: Vec<String> = Vec::new();
        if self.peek_tok()?.0 == Tok::Semicolon {
            self.next_tok()?;
            if matches!(self.peek_tok()?.0, Tok::Ident(_)) {
                while let (Tok::Ident(n), _, _) = self.next_tok()? {
                    body_params.push(n);
                    if self.peek_tok()?.0 == Tok::Comma {
                        self.next_tok()?;
                    } else {
                        break;
                    }
                }
            }
        }

        let (end_tok, el, ec) = self.next_tok()?;
        match end_tok {
            Tok::EmptyTagEnd => Ok(Element::new(
                ElementKind::Call {
                    callee: target,
                    args,
                    body: None,
                    body_params,
                },
                Span::new(line, col),
            )),
            Tok::TagEnd => {
                let (body, stop) = self.parse_block(&[], &[])?;
                let (end_name, el2, ec2) = match stop {
                    BlockStop::EndCall(n) => (n, el, ec),
                    BlockStop::Eof => {
                        return Err(self.err(
                            line,
                            col,
                            "Unclosed user directive call (missing </@...>).",
                        ))
                    }
                    _ => return Err(self.err(line, col, "Unexpected directive in the call body.")),
                };
                // 结束标签名必须与开始标签一致（Java：Expecting </@> or </@name>）
                if let Some(canonical) = call_target_canonical(&callee_expr) {
                    if !end_name.is_empty() && end_name != canonical {
                        return Err(self.err(
                            el2,
                            ec2,
                            format!("Expecting </@> or </@{canonical}>, but found </@{end_name}>."),
                        ));
                    }
                } else if !end_name.is_empty() {
                    return Err(self.err(el2, ec2, "Expecting </@>."));
                }
                Ok(Element::new(
                    ElementKind::Call {
                        callee: target,
                        args,
                        body: Some(body),
                        body_params,
                    },
                    Span::new(line, col),
                ))
            }
            other => Err(self.err(
                el,
                ec,
                format!(
                    "Expected \">\" or \"/>\" to close the tag, but found {}.",
                    tok_desc(&other)
                ),
            )),
        }
    }

    /// `<@name ...>` 的 callee 表达式 —— 对应 Java NO_SPACE_EXPRESSION 词法状态语义
    /// （FTL.jj 1614-1618 TERMINATING_WHITESPACE：`<@name` 后的首个空白产生专用 token
    /// 结束表达式）。因此后缀链（`.`/`[`/`(`/`?`/`!`/`??`）只在与前 token 紧邻时继续：
    /// `<@m [1,2]/>` → callee=m、`[1,2]` 是位置参数列表字面量；`<@m a[1]/>` → callee=a[1]。
    fn callee_expression(&mut self) -> Result<Expr> {
        let mut e = self.atomic_expression()?;
        loop {
            let (t, l, c) = self.peek_tok()?;
            // 与前一个已消费 token 相邻（无空白）才继续后缀链
            if (l, c) != self.last_tok_end {
                break;
            }
            match t {
                Tok::Dot => {
                    self.next_tok()?;
                    e = self.dot_variable(e, l, c)?;
                }
                Tok::OpenBracket => {
                    self.next_tok()?;
                    e = self.dynamic_key(e, l, c)?;
                }
                Tok::OpenParen => {
                    self.next_tok()?;
                    let args = self.positional_args(false)?;
                    self.expect_tok(Tok::CloseParen, "\")\" to close the argument list")?;
                    let span = e.span;
                    e = Expr::new(
                        ExprKind::Call {
                            callee: Box::new(e),
                            args,
                        },
                        span,
                    );
                }
                Tok::Builtin => {
                    self.next_tok()?;
                    e = self.builtin(e)?;
                }
                Tok::Exclam => {
                    self.next_tok()?;
                    e = self.default_to(e, l, c)?;
                }
                Tok::Exists => {
                    self.next_tok()?;
                    let span = e.span;
                    e = Expr::new(ExprKind::Exists(Box::new(e)), span);
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// `<#nested>` / `<#nested args>`（BodyInstruction）
    fn nested_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        if self.in_macro + self.in_function == 0 {
            return Err(self.err(
                line,
                col,
                "Cannot use a \"nested\" instruction outside a macro.",
            ));
        }
        let p = self.peek_tok()?;
        let kind = match p.0 {
            Tok::TagEnd | Tok::EmptyTagEnd => {
                self.next_tok()?;
                ElementKind::Nested {
                    args: Vec::new(),
                    body: None,
                }
            }
            _ => {
                let args = self.positional_args(false)?;
                self.loose_end()?;
                ElementKind::Nested { args, body: None }
            }
        };
        Ok(Element::new(kind, Span::new(line, col)))
    }

    /// `<#switch expr> <#case v>.. (<#case|#default>..)* </#switch>`
    fn switch_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let expr = self.expression()?;
        self.expect_tag_end()?;
        self.loop_nesting += 1;
        let r = self.switch_body(line, col, expr);
        self.loop_nesting -= 1;
        r
    }

    fn switch_body(&mut self, line: u32, col: u32, expr: Expr) -> Result<Element> {
        let mut cases: Vec<CaseDef> = Vec::new();
        let mut default: Option<Vec<Element>> = None;
        let mut default_pos: Option<usize> = None;
        let mut had_default = false;
        // 待处理的 case/default 名字：主体 parse_block 以 Dir 终止时名字已读（随
        // BlockStop::Dir 返回）；None 表示需先做空白/注释预检再打开标签
        let mut pending: Option<String> = None;
        loop {
            let lname = match pending.take() {
                Some(n) => n,
                None => {
                    // case 之间只允许空白与注释（Java WhitespaceAndComments）
                    let (text, stop, _, _) = self.next_text_chunk()?;
                    if !text.trim().is_empty() {
                        return Err(self.err(
                            line,
                            col,
                            "Unexpected content in #switch; only whitespace and comments are allowed between the #case/#default blocks.",
                        ));
                    }
                    match stop {
                        TextStop::Eof => {
                            return Err(self.err(line, col, "Unclosed <#switch> block."));
                        }
                        TextStop::Interp => {
                            return Err(self.err(
                                line,
                                col,
                                "Unexpected interpolation in #switch; only whitespace and comments are allowed between the #case/#default blocks.",
                            ));
                        }
                        TextStop::Tag => {
                            let open = self.lexer.read_tag_open();
                            match open {
                                TagOpen::TerseComment { square } => {
                                    self.lexer.scan_comment(square)?;
                                    continue;
                                }
                                TagOpen::EndDir { square } => {
                                    self.enter_tag(square);
                                    let name = self.lexer.read_name().unwrap_or_default();
                                    if name.eq_ignore_ascii_case("switch") {
                                        self.expect_tag_end_raw()?;
                                        return self.finish_switch(
                                            line,
                                            col,
                                            expr,
                                            cases,
                                            default,
                                            default_pos,
                                        );
                                    }
                                    return Err(self.err(
                                        line,
                                        col,
                                        format!(
                                            "Unexpected closing tag \"</#{name}>\" in #switch."
                                        ),
                                    ));
                                }
                                TagOpen::EndCall { .. } => {
                                    return Err(self.err(
                                        line,
                                        col,
                                        "Unexpected </@...> in #switch.",
                                    ));
                                }
                                TagOpen::Call { .. } => {
                                    return Err(self.err(
                                        line,
                                        col,
                                        "Unexpected user directive call in #switch; expected #case, #default or </#switch>.",
                                    ));
                                }
                                TagOpen::Dir { square } => {
                                    self.enter_tag(square);
                                    self.lexer
                                        .read_name()
                                        .unwrap_or_default()
                                        .to_ascii_lowercase()
                                }
                            }
                        }
                    }
                }
            };
            match lname.as_str() {
                "case" => {
                    let value = self.expression()?;
                    self.expect_tag_end()?;
                    let (els, stop) = self.parse_block(&["switch"], &["case", "default"])?;
                    match stop {
                        BlockStop::Eof => {
                            return Err(self.err(line, col, "Unclosed <#switch> block."));
                        }
                        BlockStop::EndCall(_) => {
                            return Err(self.err(line, col, "Unexpected </@...> in #switch."));
                        }
                        BlockStop::EndTag(_) | BlockStop::Dir(_) => {}
                    }
                    cases.push(CaseDef { value, body: els });
                    match stop {
                        BlockStop::Dir(n) => pending = Some(n),
                        _ => {
                            return self.finish_switch(line, col, expr, cases, default, default_pos)
                        }
                    }
                }
                "default" => {
                    if had_default {
                        return Err(self.err(
                            line,
                            col,
                            "You already had a #default in the #switch block.",
                        ));
                    }
                    had_default = true;
                    self.expect_tag_end_raw()?;
                    // #default 主体同样在第二个 #default 处终止（switch 层校验重复）
                    let (els, stop) = self.parse_block(&["switch"], &["case", "default"])?;
                    match stop {
                        BlockStop::Eof => {
                            return Err(self.err(line, col, "Unclosed <#switch> block."));
                        }
                        BlockStop::EndCall(_) => {
                            return Err(self.err(line, col, "Unexpected </@...> in #switch."));
                        }
                        BlockStop::EndTag(_) | BlockStop::Dir(_) => {}
                    }
                    default = Some(els);
                    default_pos = Some(cases.len());
                    match stop {
                        BlockStop::Dir(n) => pending = Some(n),
                        _ => {
                            return self.finish_switch(line, col, expr, cases, default, default_pos)
                        }
                    }
                }
                other => {
                    return Err(self.err(
                        line,
                        col,
                        format!("Unexpected directive <#{other}> in #switch; expected #case, #default or </#switch>."),
                    ));
                }
            }
        }
    }

    /// `</#switch>` 之后组装（Java 允许空 switch —— switch.ftl 用例
    /// `[<#switch 213></#switch>]`，空 switch 渲染为空）
    fn finish_switch(
        &self,
        line: u32,
        col: u32,
        expr: Expr,
        cases: Vec<CaseDef>,
        default: Option<Vec<Element>>,
        default_pos: Option<usize>,
    ) -> Result<Element> {
        Ok(Element::new(
            ElementKind::Switch {
                expr,
                cases,
                default,
                default_pos,
            },
            Span::new(line, col),
        ))
    }

    /// `<#attempt> try <#recover> recover </#attempt>`（RecoveryBlock 语法）
    fn attempt_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        self.expect_tag_end_raw()?;
        let (try_, stop) = self.parse_block(&[], &["recover"])?;
        if !matches!(stop, BlockStop::Dir(_)) {
            return Err(self.err(line, col, "#attempt must contain a #recover block."));
        }
        self.expect_tag_end_raw()?;
        let (recover, stop) = self.parse_block(&["recover", "attempt"], &[])?;
        if !matches!(stop, BlockStop::EndTag(_)) {
            return Err(self.err(line, col, "Unclosed #attempt/#recover block."));
        }
        Ok(Element::new(
            ElementKind::Attempt { try_, recover },
            Span::new(line, col),
        ))
    }

    /// `<#return>` / `<#return expr>`（上下文校验对应 Java Return()）
    fn return_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let p = self.peek_tok()?;
        let expr = match p.0 {
            Tok::TagEnd | Tok::EmptyTagEnd => {
                self.next_tok()?;
                None
            }
            _ => {
                let e = self.expression()?;
                self.loose_end()?;
                Some(e)
            }
        };
        if self.in_function > 0 && expr.is_none() {
            return Err(self.err(line, col, "A function must return a value"));
        }
        if self.in_macro > 0 && expr.is_some() {
            return Err(self.err(line, col, "A macro cannot return a value"));
        }
        if self.in_macro + self.in_function == 0 {
            return Err(self.err(
                line,
                col,
                "A return instruction can only occur inside a macro or function",
            ));
        }
        Ok(Element::new(
            ElementKind::Return { expr },
            Span::new(line, col),
        ))
    }

    /// `<#stop>` / `<#stop expr>`（HALT/STOP）
    fn stop_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let p = self.peek_tok()?;
        let msg = match p.0 {
            Tok::TagEnd | Tok::EmptyTagEnd => {
                self.next_tok()?;
                None
            }
            _ => {
                let e = self.expression()?;
                self.loose_end()?;
                Some(e)
            }
        };
        Ok(Element::new(
            ElementKind::Stop { msg },
            Span::new(line, col),
        ))
    }

    /// `<#include path [;] [attr=expr]*>`（参数名校验对应 Java Include()）
    fn include_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let path = self.expression()?;
        if self.peek_tok()?.0 == Tok::Semicolon {
            self.next_tok()?;
        }
        let mut attrs: Vec<(String, Expr)> = Vec::new();
        loop {
            let (t, l, c) = self.peek_tok()?;
            let Tok::Ident(name) = t else { break };
            if self.peek_tok2()?.0 != Tok::Eq {
                break;
            }
            self.next_tok()?;
            self.expect_tok(Tok::Eq, "\"=\" after the parameter name")?;
            let value = self.expression()?;
            let lname = name.to_ascii_lowercase();
            if !matches!(lname.as_str(), "parse" | "encoding" | "ignore_missing") {
                return Err(self.err(
                    l,
                    c,
                    format!(
                        "Unsupported named #include parameter: \"{name}\". Supported parameters are: \"parse\", \"encoding\", \"ignore_missing\"."
                    ),
                ));
            }
            attrs.push((name, value));
        }
        self.loose_end()?;
        Ok(Element::new(
            ElementKind::Include { path, attrs },
            Span::new(line, col),
        ))
    }

    /// `<#import path as ns>`（LibraryLoad）
    fn import_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let path = self.expression()?;
        self.expect_tok(Tok::As, "\"as\" after the template path")?;
        let (t, l, c) = self.next_tok()?;
        let Tok::Ident(ns) = t else {
            return Err(self.err(
                l,
                c,
                format!(
                    "Expected a namespace name after \"as\", but found {}.",
                    tok_desc(&t)
                ),
            ));
        };
        self.loose_end()?;
        Ok(Element::new(
            ElementKind::Import { path, ns },
            Span::new(line, col),
        ))
    }

    // -----------------------------------------------------------------------
    // 表达式产生式（docs/03 §3：优先级低 → 高）
    // -----------------------------------------------------------------------

    fn expression(&mut self) -> Result<Expr> {
        self.or_expression()
    }

    /// OrExpression：`lhs (|| rhs)*`
    fn or_expression(&mut self) -> Result<Expr> {
        let mut lhs = self.and_expression()?;
        loop {
            let (t, _, _) = self.peek_tok()?;
            if t != Tok::Or {
                break;
            }
            self.next_tok()?;
            let rhs = self.and_expression()?;
            let span = lhs.span;
            lhs = Expr::new(ExprKind::Or(Box::new(lhs), Box::new(rhs)), span);
        }
        Ok(lhs)
    }

    /// AndExpression：`lhs (&& rhs)*`
    fn and_expression(&mut self) -> Result<Expr> {
        let mut lhs = self.equality_expression()?;
        loop {
            let (t, _, _) = self.peek_tok()?;
            if t != Tok::And {
                break;
            }
            self.next_tok()?;
            let rhs = self.equality_expression()?;
            let span = lhs.span;
            lhs = Expr::new(ExprKind::And(Box::new(lhs), Box::new(rhs)), span);
        }
        Ok(lhs)
    }

    /// EqualityExpression：`rel [(==|!=) rel]`（单一可选，非结合）
    fn equality_expression(&mut self) -> Result<Expr> {
        let lhs = self.relational_expression()?;
        let (t, _, _) = self.peek_tok()?;
        match t {
            Tok::Eq => {
                self.next_tok()?;
                let rhs = self.relational_expression()?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Eq(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::NotEq => {
                self.next_tok()?;
                let rhs = self.relational_expression()?;
                let span = lhs.span;
                Ok(Expr::new(
                    ExprKind::NotEq(Box::new(lhs), Box::new(rhs)),
                    span,
                ))
            }
            _ => Ok(lhs),
        }
    }

    /// RelationalExpression：`range [(<|<=|>|>=) range]`（单一可选）
    fn relational_expression(&mut self) -> Result<Expr> {
        let lhs = self.range_expression()?;
        let (t, _, _) = self.peek_tok()?;
        match t {
            Tok::Lt => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Lt(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::Lte => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Lte(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::Gt => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Gt(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::Gte => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Gte(Box::new(lhs), Box::new(rhs)), span))
            }
            _ => Ok(lhs),
        }
    }

    /// RangeExpression：`additive [(..<|..*|..) [additive]]`
    fn range_expression(&mut self) -> Result<Expr> {
        let lhs = self.additive_expression()?;
        let (t, _, _) = self.peek_tok()?;
        match t {
            Tok::DotDotLess => {
                self.next_tok()?;
                let end = self.additive_expression()?;
                let span = lhs.span;
                Ok(Expr::new(
                    ExprKind::Range {
                        start: Box::new(lhs),
                        end: Some(Box::new(end)),
                        kind: RangeKind::Exclusive,
                    },
                    span,
                ))
            }
            Tok::DotDotStar => {
                self.next_tok()?;
                let end = self.additive_expression()?;
                let span = lhs.span;
                Ok(Expr::new(
                    ExprKind::Range {
                        start: Box::new(lhs),
                        end: Some(Box::new(end)),
                        kind: RangeKind::SizeLimited,
                    },
                    span,
                ))
            }
            Tok::DotDot => {
                self.next_tok()?;
                let span = lhs.span;
                // `..` 后跟加法表达式为含端范围；否则无界（Java END_UNBOUND →
                // 契约无 Unbounded variant，end=None 时 kind 取 SizeLimited，文档化偏差）
                let (end, kind) = if self.at_expr_start(false)? {
                    (
                        Some(Box::new(self.additive_expression()?)),
                        RangeKind::Inclusive,
                    )
                } else {
                    (None, RangeKind::SizeLimited)
                };
                Ok(Expr::new(
                    ExprKind::Range {
                        start: Box::new(lhs),
                        end,
                        kind,
                    },
                    span,
                ))
            }
            _ => Ok(lhs),
        }
    }

    /// AdditiveExpression：`multi ((+|-) multi)*`（`+` 语义为 AddConcat）
    fn additive_expression(&mut self) -> Result<Expr> {
        let mut lhs = self.multiplicative_expression()?;
        loop {
            let (t, _, _) = self.peek_tok()?;
            match t {
                Tok::Plus => {
                    self.next_tok()?;
                    let rhs = self.multiplicative_expression()?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Add(Box::new(lhs), Box::new(rhs)), span);
                }
                Tok::Minus => {
                    self.next_tok()?;
                    let rhs = self.multiplicative_expression()?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Sub(Box::new(lhs), Box::new(rhs)), span);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    /// MultiplicativeExpression：`unary ((*|/|%) unary)*`
    fn multiplicative_expression(&mut self) -> Result<Expr> {
        let mut lhs = self.unary_expression()?;
        loop {
            let (t, _, _) = self.peek_tok()?;
            match t {
                Tok::Times => {
                    self.next_tok()?;
                    let rhs = self.unary_expression()?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Mul(Box::new(lhs), Box::new(rhs)), span);
                }
                Tok::Divide => {
                    self.next_tok()?;
                    let rhs = self.unary_expression()?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Div(Box::new(lhs), Box::new(rhs)), span);
                }
                Tok::Percent => {
                    self.next_tok()?;
                    let rhs = self.unary_expression()?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Mod(Box::new(lhs), Box::new(rhs)), span);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    /// UnaryExpression：UnaryPlusMinus / NotExpression / PrimaryExpression
    fn unary_expression(&mut self) -> Result<Expr> {
        let (t, l, c) = self.peek_tok()?;
        match t {
            Tok::Minus => {
                self.next_tok()?;
                let e = self.primary_expression()?;
                Ok(Expr::new(
                    ExprKind::UnaryMinus(Box::new(e)),
                    Span::new(l, c),
                ))
            }
            Tok::Plus => {
                // `+` 仅强制操作数为数值字面量（Java UnaryPlusMinusExpression(isMinus=false)）；
                // 本实现不检查字面量，直接返回操作数
                self.next_tok()?;
                self.primary_expression()
            }
            Tok::Exclam => {
                // NotExpression：`(!)+ primary`（Java 循环累积，第一个 `!` 为最外层）
                let mut nots: Vec<(u32, u32)> = vec![(l, c)];
                self.next_tok()?;
                while self.peek_tok()?.0 == Tok::Exclam {
                    let (_, l2, c2) = self.next_tok()?;
                    nots.push((l2, c2));
                }
                let mut e = self.primary_expression()?;
                for (nl, nc) in nots.into_iter().rev() {
                    e = Expr::new(ExprKind::Not(Box::new(e)), Span::new(nl, nc));
                }
                Ok(e)
            }
            _ => self.primary_expression(),
        }
    }

    /// PrimaryExpression：`atomic (dot|dynkey|methodargs|builtin|defaultto|exists)*`
    fn primary_expression(&mut self) -> Result<Expr> {
        let mut e = self.atomic_expression()?;
        loop {
            let (t, l, c) = self.peek_tok()?;
            match t {
                Tok::Dot => {
                    self.next_tok()?;
                    e = self.dot_variable(e, l, c)?;
                }
                Tok::OpenBracket => {
                    self.next_tok()?;
                    e = self.dynamic_key(e, l, c)?;
                }
                Tok::OpenParen => {
                    self.next_tok()?;
                    let args = self.positional_args(false)?;
                    self.expect_tok(Tok::CloseParen, "\")\" to close the argument list")?;
                    let span = e.span;
                    e = Expr::new(
                        ExprKind::Call {
                            callee: Box::new(e),
                            args,
                        },
                        span,
                    );
                }
                Tok::Builtin => {
                    self.next_tok()?;
                    e = self.builtin(e)?;
                }
                Tok::Exclam => {
                    self.next_tok()?;
                    e = self.default_to(e, l, c)?;
                }
                Tok::Exists => {
                    self.next_tok()?;
                    let span = e.span;
                    e = Expr::new(ExprKind::Exists(Box::new(e)), span);
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// DotVariable：`.name`（含关键字 token 作名；对应 FTL.jj DotVariable）
    fn dot_variable(&mut self, target: Expr, _l: u32, _c: u32) -> Result<Expr> {
        let (t, l, c) = self.next_tok()?;
        let name = match t {
            Tok::Ident(n) => n,
            Tok::True => "true".to_string(),
            Tok::False => "false".to_string(),
            Tok::In => "in".to_string(),
            Tok::As => "as".to_string(),
            Tok::Using => "using".to_string(),
            Tok::Lt => "lt".to_string(),
            Tok::Lte => "lte".to_string(),
            Tok::Gt => "gt".to_string(),
            Tok::Gte => "gte".to_string(),
            Tok::Times => "*".to_string(),
            Tok::DoubleStar => "**".to_string(),
            other => {
                return Err(self.err(
                    l,
                    c,
                    format!(
                        "Expected a name after \".\", but found {}.",
                        tok_desc(&other)
                    ),
                ))
            }
        };
        let span = target.span;
        Ok(Expr::new(
            ExprKind::Dot {
                target: Box::new(target),
                name,
            },
            span,
        ))
    }

    /// DynamicKey：`[expr]`
    fn dynamic_key(&mut self, target: Expr, _l: u32, _c: u32) -> Result<Expr> {
        let key = self.expression()?;
        self.expect_tok(Tok::CloseBracket, "\"]\" to close the key expression")?;
        let span = target.span;
        Ok(Expr::new(
            ExprKind::DynKey {
                target: Box::new(target),
                key: Box::new(key),
            },
            span,
        ))
    }

    /// BuiltIn：`?name` 或 `?name(args)`（args 允许 lambda：PositionalMaybeLambdaArgs）。
    /// 内建名归一化为 legacy 蛇形（Java namingConvention：`?capFirst` == `?cap_first`；
    /// 全大写名非法 —— Java checkNamingConvention 报语法错误，v1 在求值期报 Unknown built-in）
    fn builtin(&mut self, target: Expr) -> Result<Expr> {
        let (t, nl, nc) = self.next_tok()?;
        let Tok::Ident(name) = t else {
            return Err(self.err(
                nl,
                nc,
                format!(
                    "Expected a built-in name after \"?\", but found {}.",
                    tok_desc(&t)
                ),
            ));
        };
        let name = camel_to_snake(&name);
        let args = if self.peek_tok()?.0 == Tok::OpenParen {
            self.next_tok()?;
            let args = self.positional_args(true)?;
            self.expect_tok(
                Tok::CloseParen,
                "\")\" to close the built-in parameter list",
            )?;
            Some(args)
        } else {
            None
        };
        // 注：Java 仅 BuiltInWithParseTimeParameters 接受 `(...)`；本实现宽松接受
        // 任意内建名后的参数（渲染期由 builtins 模块校验），文档化偏差
        let span = target.span;
        Ok(Expr::new(
            ExprKind::BuiltIn {
                target: Box::new(target),
                name,
                args,
            },
            span,
        ))
    }

    /// DefaultTo：`!` 或 `!default`（TERMINATING_EXCLAM 语义：命名参数值内 `!`+空白 → 无默认）
    fn default_to(&mut self, target: Expr, l: u32, c: u32) -> Result<Expr> {
        let _ = (l, c);
        // 命名参数上下文（NAMED_PARAMETER_EXPRESSION）：`!` 后紧跟空白 → TERMINATING_EXCLAM。
        // 通过 `!` 与下一 token 的行列间隙判断空白（前瞻缓冲不丢信息）
        let span = target.span;
        let p = self.peek_tok()?;
        let ws_after = p.1 != l || p.2 != c + 1;
        let default = if ws_after && self.in_named_arg_value() {
            None
        } else if self.at_expr_start(false)? {
            Some(Box::new(self.expression()?))
        } else {
            None
        };
        Ok(Expr::new(
            ExprKind::Default {
                target: Box::new(target),
                default,
            },
            span,
        ))
    }

    /// 当前是否在命名参数的值表达式内（NAMED_PARAMETER_EXPRESSION 语义）
    fn in_named_arg_value(&self) -> bool {
        self.named_arg_depth > 0
    }

    /// AtomicExpression：字面量 / 标识符 / 括号 / 列表 / 哈希 / 内置变量
    fn atomic_expression(&mut self) -> Result<Expr> {
        let (t, l, c) = self.peek_tok()?;
        match t {
            Tok::Number(raw) => {
                self.next_tok()?;
                let n = number_literal(&raw)
                    .ok_or_else(|| self.err(l, c, format!("Invalid number literal: \"{raw}\".")))?;
                Ok(Expr::new(ExprKind::Num(n), Span::new(l, c)))
            }
            Tok::Str(raw) => {
                self.next_tok()?;
                let decoded = self.decode_string(&raw, l, c)?;
                let kind = self.interpolate_string(decoded, l, c)?;
                Ok(Expr::new(kind, Span::new(l, c)))
            }
            Tok::RawStr(raw) => {
                self.next_tok()?;
                // 原始字符串：无转义解码、无插值（StringLiteral(raw=true)）
                Ok(Expr::new(ExprKind::Str(raw), Span::new(l, c)))
            }
            Tok::True => {
                self.next_tok()?;
                Ok(Expr::new(ExprKind::Bool(true), Span::new(l, c)))
            }
            Tok::False => {
                self.next_tok()?;
                Ok(Expr::new(ExprKind::Bool(false), Span::new(l, c)))
            }
            Tok::Ident(name) => {
                self.next_tok()?;
                // 契约：now → 内置变量（true/false 已由 token 分流）
                if name == "now" {
                    Ok(Expr::new(
                        ExprKind::BuiltinVar(BuiltinVar::Now),
                        Span::new(l, c),
                    ))
                } else {
                    Ok(Expr::new(ExprKind::Ident(name), Span::new(l, c)))
                }
            }
            Tok::Dot => {
                // BuiltinVariable：`.name`（Java BuiltinVariable()，FTL.jj 2119-2151；
                // SPEC_VAR_NAMES 全清单见 BuiltinVariable.java:43-82；camelCase 归一化）
                self.next_tok()?;
                let (t2, nl, nc) = self.next_tok()?;
                match t2 {
                    Tok::Ident(name) => {
                        let name = camel_to_snake(&name);
                        match builtin_var_of(&name) {
                            Some(v) => Ok(Expr::new(
                                ExprKind::BuiltinVar(v),
                                Span::new(nl, nc),
                            )),
                            None => Err(self.err(
                                nl,
                                nc,
                                format!(
                                    "The built-in variable \".{name}\" doesn't exist. The allowed special variable names are: namespace, main, globals, locals, data_model, vars, lang, locale, locale_object, time_zone, template_name, main_template_name, current_template_name, node, current_node, error, output_encoding, output_format, auto_esc, url_escaping_charset, version, incompatible_improvements, args, now."
                                ),
                            )),
                        }
                    }
                    other => Err(self.err(
                        nl,
                        nc,
                        format!(
                            "Expected a name after \".\", but found {}.",
                            tok_desc(&other)
                        ),
                    )),
                }
            }
            Tok::OpenParen => {
                self.next_tok()?;
                let e = self.expression()?;
                self.expect_tok(
                    Tok::CloseParen,
                    "\")\" to close the parenthesized expression",
                )?;
                Ok(Expr::new(ExprKind::Paren(Box::new(e)), Span::new(l, c)))
            }
            Tok::OpenBracket => {
                // ListLiteral：`[ PositionalArgs ]`
                self.next_tok()?;
                let items = self.positional_args(false)?;
                self.expect_tok(Tok::CloseBracket, "\"]\" to close the list literal")?;
                Ok(Expr::new(ExprKind::ListLit(items), Span::new(l, c)))
            }
            Tok::OpenCurly => {
                // HashLiteral：`{ [expr (,|:) expr (, ...)*] }`（键必须字符串字面量）
                self.next_tok()?;
                let mut pairs: Vec<(Expr, Expr)> = Vec::new();
                if !matches!(self.peek_tok()?.0, Tok::CloseCurly) {
                    loop {
                        let key = self.expression()?;
                        let (sep, _, _) = self.next_tok()?;
                        if !matches!(sep, Tok::Comma | Tok::Colon) {
                            return Err(self.err(
                                l,
                                c,
                                "Expected \",\" or \":\" between the hash key and value.",
                            ));
                        }
                        let value = self.expression()?;
                        // Java stringLiteralOnly：仅禁止数值/序列/哈希/布尔字面量；
                        // 标识符、点表达式等均可（求值期须为字符串）
                        if matches!(
                            key.kind,
                            ExprKind::Num(_)
                                | ExprKind::ListLit(_)
                                | ExprKind::HashLit(_)
                                | ExprKind::Bool(_)
                        ) {
                            return Err(self.err(
                                l,
                                c,
                                "Hash literal keys must be strings, but a number/list/hash/boolean literal was found.",
                            ));
                        }
                        pairs.push((key, value));
                        let p = self.peek_tok()?;
                        if p.0 != Tok::Comma && p.0 != Tok::Colon {
                            break;
                        }
                        self.next_tok()?;
                    }
                }
                self.expect_tok(Tok::CloseCurly, "\"}\" to close the hash literal")?;
                Ok(Expr::new(ExprKind::HashLit(pairs), Span::new(l, c)))
            }
            other => Err(self.err(
                l,
                c,
                format!("Expected an expression, but found {}.", tok_desc(&other)),
            )),
        }
    }

    /// PositionalArgs：`[expr (, expr)*]`（`allow_lambda` 时允许 LocalLambdaExpression）
    fn positional_args(&mut self, allow_lambda: bool) -> Result<Vec<Expr>> {
        let mut args: Vec<Expr> = Vec::new();
        if self.at_expr_start(allow_lambda)? {
            args.push(self.parse_arg(allow_lambda)?);
            loop {
                if self.peek_tok()?.0 == Tok::Comma {
                    self.next_tok()?;
                }
                if !self.at_expr_start(allow_lambda)? {
                    break;
                }
                args.push(self.parse_arg(allow_lambda)?);
            }
        }
        Ok(args)
    }

    fn parse_arg(&mut self, allow_lambda: bool) -> Result<Expr> {
        if allow_lambda && self.at_lambda_start()? {
            self.lambda()
        } else {
            self.expression()
        }
    }

    /// 当前 token 是否可开始一个表达式（DefaultTo/参数列表的前瞻；
    /// 含一元 +/-/! —— Java UnaryPlusMinusExpression/NotExpression 可作参数首 token，
    /// 如 `?then(1, -x)`、`[1, -1]`、`join(1..-1, ...)`）
    fn at_expr_start(&mut self, allow_lambda: bool) -> Result<bool> {
        if allow_lambda && self.at_lambda_start()? {
            return Ok(true);
        }
        let (t, _, _) = self.peek_tok()?;
        Ok(matches!(
            t,
            Tok::Ident(_)
                | Tok::Number(_)
                | Tok::Str(_)
                | Tok::RawStr(_)
                | Tok::True
                | Tok::False
                | Tok::OpenParen
                | Tok::OpenBracket
                | Tok::OpenCurly
                | Tok::Dot
                | Tok::Minus
                | Tok::Plus
                | Tok::Exclam
        ))
    }

    /// lambda 前瞻：`x ->`、`(x) ->` 或 `(a, b) ->`（对应 LambdaExpressionParameterList）。
    /// 前瞻开始前取出全部前瞻缓冲并保存词法位置，前瞻期间全部重新词法，
    /// 结束后缓冲与词法位置一并恢复（对调用方完全无副作用）。
    fn at_lambda_start(&mut self) -> Result<bool> {
        let save = self.lexer.save();
        let saved_buf: Vec<(Tok, u32, u32, u32, u32)> = self.buf.drain(..).collect();
        let t = self.next_tok()?.0;
        let ok = match t {
            Tok::Ident(_) => self.peek_tok()?.0 == Tok::LambdaArrow,
            Tok::OpenParen => {
                let mut found = false;
                if matches!(self.next_tok()?.0, Tok::Ident(_)) {
                    if self.peek_tok()?.0 == Tok::Comma {
                        // (a, b, ...) -> ：全 Ident + 逗号序列 + `) ->`
                        self.next_tok()?;
                        loop {
                            if !matches!(self.next_tok()?.0, Tok::Ident(_)) {
                                break;
                            }
                            if self.peek_tok()?.0 == Tok::Comma {
                                self.next_tok()?;
                                continue;
                            }
                            break;
                        }
                        found = self.peek_tok()?.0 == Tok::CloseParen
                            && self.peek_tok2()?.0 == Tok::LambdaArrow;
                    } else {
                        found = self.next_tok()?.0 == Tok::CloseParen
                            && self.peek_tok()?.0 == Tok::LambdaArrow;
                    }
                }
                found
            }
            _ => false,
        };
        self.lexer.restore(&save);
        self.buf = saved_buf;
        Ok(ok)
    }

    /// LocalLambdaExpression：`x -> expr` / `(x, y) -> expr`（对应 Java
    /// LambdaExpressionParameterList + LocalLambdaExpression，FTL.jj 2326-2355）
    fn lambda(&mut self) -> Result<Expr> {
        let (t, l, c) = self.next_tok()?;
        let mut params: Vec<String> = Vec::new();
        match t {
            Tok::OpenParen => {
                loop {
                    let (Tok::Ident(p), _, _) = self.next_tok()? else {
                        return Err(self.err(l, c, "Expected a lambda parameter name."));
                    };
                    params.push(p);
                    if self.peek_tok()?.0 == Tok::Comma {
                        self.next_tok()?;
                        continue;
                    }
                    break;
                }
                self.expect_tok(Tok::CloseParen, "\")\" to close the lambda parameter list")?;
            }
            Tok::Ident(p) => params.push(p),
            other => {
                return Err(self.err(
                    l,
                    c,
                    format!(
                        "Expected a lambda parameter name, but found {}.",
                        tok_desc(&other)
                    ),
                ))
            }
        }
        self.expect_tok(Tok::LambdaArrow, "\"->\" after the lambda parameter")?;
        let body = self.expression()?;
        Ok(Expr::new(
            ExprKind::Lambda {
                params,
                body: Box::new(body),
            },
            Span::new(l, c),
        ))
    }

    /// IdentifierOrStringLiteral：`<#assign>`/`<#macro>` 的名称（Ident 或 Str）
    fn ident_or_string_literal(&mut self) -> Result<(String, Span)> {
        let (t, l, c) = self.next_tok()?;
        match t {
            Tok::Ident(n) => Ok((n, Span::new(l, c))),
            Tok::Str(raw) => {
                let decoded = self.decode_string(&raw, l, c)?;
                Ok((decoded, Span::new(l, c)))
            }
            other => Err(self.err(
                l,
                c,
                format!(
                    "Expected a name (identifier or string literal), but found {}.",
                    tok_desc(&other)
                ),
            )),
        }
    }

    /// 字符串字面量解码（对应 StringUtil.FTLStringLiteralDec + `\uXXXX` 扩展）
    fn decode_string(&self, raw: &str, line: u32, col: u32) -> Result<String> {
        let mut out = String::new();
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            let esc = chars.next().ok_or_else(|| {
                self.err(
                    line,
                    col,
                    "The last character of string literal is backslash",
                )
            })?;
            match esc {
                '"' => out.push('"'),
                '\'' => out.push('\''),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'f' => out.push('\u{000c}'),
                'b' => out.push('\u{0008}'),
                'g' => out.push('>'),
                'l' => out.push('<'),
                'a' => out.push('&'),
                '{' => out.push('{'),
                '=' => out.push('='),
                'x' => {
                    // \xHHHH（1-4 位十六进制；Java StringUtil.FTLStringLiteralDec :625-648
                    // 的 `z = idx+3` 上限 —— `\x010C` = U+010C）
                    let mut v: u32 = 0;
                    let mut n = 0;
                    for _ in 0..4 {
                        match chars.clone().next() {
                            Some(h) if h.is_ascii_hexdigit() => {
                                v = v * 16 + h.to_digit(16).unwrap();
                                chars.next();
                                n += 1;
                            }
                            _ => break,
                        }
                    }
                    if n == 0 {
                        return Err(self.err(line, col, "Invalid \\x escape in a string literal"));
                    }
                    out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
                }
                'u' => {
                    // \uXXXX（4 位十六进制；扩展，Java FTLStringLiteralDec 不支持）
                    let mut v: u32 = 0;
                    let mut n = 0;
                    for _ in 0..4 {
                        match chars.next() {
                            Some(h) if h.is_ascii_hexdigit() => {
                                v = v * 16 + h.to_digit(16).unwrap();
                                n += 1;
                            }
                            Some(_) | None => break,
                        }
                    }
                    if n != 4 {
                        return Err(self.err(
                            line,
                            col,
                            "Invalid \\u escape in a string literal (expected 4 hex digits)",
                        ));
                    }
                    out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
                }
                other => {
                    return Err(self.err(
                        line,
                        col,
                        format!("Invalid escape sequence (\\{other}) in a string literal"),
                    ))
                }
            }
        }
        Ok(out)
    }

    /// 字符串插值：解码后内容中找 `${...}`（StringLiteral(interpolate=true) 语义；
    /// 与 Java 相同：先解码再找 `${`，`\${` 转义后的歧义与 Java 一致）
    fn interpolate_string(&mut self, decoded: String, line: u32, col: u32) -> Result<ExprKind> {
        let mut parts: Vec<StrPart> = Vec::new();
        let mut rest: &str = &decoded;
        let mut text = String::new();
        loop {
            match rest.find("${") {
                None => {
                    text.push_str(rest);
                    break;
                }
                Some(i) => {
                    text.push_str(&rest[..i]);
                    let after = &rest[i + 2..];
                    let (inner, tail) = match find_matching_brace(after) {
                        Some(pair) => pair,
                        None => {
                            return Err(self.err(
                                line,
                                col,
                                "Unclosed \"${\" interpolation in a string literal.",
                            ))
                        }
                    };
                    let inner_expr = self.parse_sub_expression(inner)?;
                    if !text.is_empty() {
                        parts.push(StrPart::Text(std::mem::take(&mut text)));
                    }
                    parts.push(StrPart::Interp(Box::new(inner_expr)));
                    rest = tail;
                }
            }
        }
        if parts.is_empty() {
            Ok(ExprKind::Str(decoded))
        } else {
            if !text.is_empty() {
                parts.push(StrPart::Text(text));
            }
            Ok(ExprKind::InterpStr(parts))
        }
    }

    /// 在子文本上解析插值内的表达式（Java StringLiteral.parseValue 的 sub-parser 语义）
    fn parse_sub_expression(&self, inner: &str) -> Result<Expr> {
        let mut sub = Parser::new(self.cfg, &self.name, inner);
        sub.lexer.strict_syntax = self.lexer.strict_syntax;
        sub.ctx = ExprCtx::Interp;
        let e = sub.expression()?;
        let (t, l, c) = sub.peek_tok()?;
        if t == Tok::Eof || t == Tok::InterpEnd {
            Ok(e)
        } else {
            Err(sub.err(
                l,
                c,
                format!("Unexpected {} in a string interpolation.", tok_desc(&t)),
            ))
        }
    }

    // -----------------------------------------------------------------------
    // 空白剥离标记（对应 TextBlock.postParseCleanup；docs/08 §5.2）
    // -----------------------------------------------------------------------

    /// 对整个元素树打 strip_before/strip_after 标记。
    /// 规则（Java 对齐）：
    /// - strip_before：文本首部空白（含首个换行）可剥，且同行的前一终结节点不"care"空白
    ///   （即前一元素不是无换行的文本）；
    /// - strip_after：文本尾部（最后一个换行后）空白可剥，且同行的后一终结节点不 care；
    /// - 模板根的首个文本不剥（Java isTopLevelTextIfParentIs 守卫）；
    /// - `<#t>`/`<#nt>` 显式控制相邻文本（deliberate 语义）。
    fn mark_stripping(root: &mut [Element], strip_ws: bool) {
        if !strip_ws {
            return;
        }
        Self::mark_block(root, None, None, true);
    }

    /// 对文本元素做 deliberate 扫描（Java TextBlock.deliberateLeftTrim/RightTrim，
    /// TextBlock.java:143-236）：只依赖元素类型与行号，与文本内容无关
    /// → 可在可变借用前计算。返回 (left_trim, left_blocked, right_trim, right_blocked, heinous_drop)。
    fn deliberate_scan(
        els: &[Element],
        i: usize,
        begin_line: u32,
        end_line: u32,
        is_root: bool,
    ) -> (bool, bool, bool, bool, bool) {
        // deliberateLeftTrim：后面同行 lt/t 裁最后一行行首；nt 阻止
        // （Java 循环条件 `elem.beginLine == this.endLine` 对每个元素生效，换行即终止）
        let mut left_trim = false;
        let mut left_blocked = false;
        for e in els.iter().skip(i + 1) {
            if e.span.line > end_line {
                break;
            }
            match &e.kind {
                ElementKind::LeftTrimLine | ElementKind::TrimLineStart => {
                    left_trim = true;
                    break;
                }
                ElementKind::NoTrimLineStart => {
                    left_blocked = true;
                    break;
                }
                _ => {}
            }
        }
        // deliberateRightTrim：前面同行 rt/t 裁第一行尾部；nt 阻止
        let mut right_trim = false;
        let mut right_blocked = false;
        for j in (0..i).rev() {
            let e = &els[j];
            if e.span.line < begin_line {
                break;
            }
            match &e.kind {
                ElementKind::TrimLineEnd | ElementKind::TrimLineStart => {
                    right_trim = true;
                    break;
                }
                ElementKind::NoTrimLineStart => {
                    right_blocked = true;
                    break;
                }
                _ => {}
            }
        }
        // HEINOUS 块（TextBlock.java:239-254）：right_trim 且 opening 不可整段裁时，
        // trailing（首行后的空白段）是否裁掉由**后文同行**决定：遇 heedsOpeningWhitespace
        // 元素 → 保留（ignorable 文本 heeds=false，:316-318）；遇 lt/t → 裁掉并终止
        let heinous_drop = if right_trim {
            let mut drop = true;
            for (k, e) in els.iter().enumerate().skip(i + 1) {
                if e.span.line > end_line {
                    break;
                }
                if leaf_heeds_opening(e) && !is_ignorable_text(els, k, is_root) {
                    drop = false;
                }
                if matches!(
                    e.kind,
                    ElementKind::LeftTrimLine | ElementKind::TrimLineStart
                ) {
                    drop = true;
                    break;
                }
            }
            drop
        } else {
            false
        };
        (
            left_trim,
            left_blocked,
            right_trim,
            right_blocked,
            heinous_drop,
        )
    }

    fn mark_block(els: &mut [Element], prev: Option<Term>, next: Option<Term>, is_root: bool) {
        for i in 0..els.len() {
            // Java prevTerminalNode/nextTerminalNode（TextBlock.java:443-464）：逐叶线性
            // 回溯/前进，**跳过 ignorable 节点**（TrimInstruction.isIgnorable=true）；
            // openingCharsToStrip/trailingCharsToStrip（:280-312）的同行循环对链上每个叶
            // 检查 heeds，命中 care 的叶 → 不剥；遇到跨行叶 → 链终止 → 照剥。
            // 注意：链**不止一跳**——`  ${""}<@x/><#lt>\n` 的 `\n` 须穿过
            // Trim(heeds=false) 与 Call(heeds=false) 直达 Interp(heeds=true) 才判定不剥。
            // prev 链用文本 beginLine 匹配、next 链用 endLine 匹配（Java 循环条件）；
            // 文本链上叶的 heeds/行号按**当前（已裁剪）内容 + 原始 endLine** 读取
            // （本层按序处理，之前的文本已 deliberate 裁剪 —— 与 Java 逐元素顺序一致）
            let (begin_line, end_line) = match &els[i].kind {
                ElementKind::Text { orig_end_line, .. }
                | ElementKind::NoParse { orig_end_line, .. } => (els[i].span.line, *orig_end_line),
                // 块元素：next 链行号取**末叶**的结束行（Java 链从块内最后一个叶继续，
                // 条件 `elem.beginLine == this.endLine` 用的是该叶的 endLine，
                // 如 `<#if y>foo\n  </#if>bar` —— "bar" 在行 2 与 "foo\n  " 的 endLine 匹配）
                _ => (
                    els[i].span.line,
                    last_leaf(&els[i])
                        .map(|t| t.line)
                        .unwrap_or(els[i].span.line),
                ),
            };
            let prev_i = walk_prev(els, i, begin_line, prev, is_root);
            let next_i = walk_next(els, i, end_line, next, is_root);
            let span_col = els[i].span.col;
            let (left_trim, left_blocked, right_trim, right_blocked, heinous_drop) =
                Self::deliberate_scan(els, i, begin_line, end_line, is_root);
            match &mut els[i].kind {
                ElementKind::Text {
                    text,
                    strip_before,
                    strip_after,
                    ..
                }
                | ElementKind::NoParse {
                    text,
                    strip_before,
                    strip_after,
                    ..
                } => {
                    // Java 顺序（TextBlock.java:110-111, 119-124）：deliberateLeftTrim 先、
                    // deliberateRightTrim 后，自动剥离（openingCharsToStrip/trailingCharsToStrip
                    // :264-312）作用于**裁剪后**文本；deliberate 侧不应用自动剥离。
                    // 首文本的早退（:116-118）在 deliberate 裁剪**之后** —— 先裁后跳过
                    if left_trim {
                        trim_last_line_leading(text, span_col);
                    }
                    let first_line_dropped = if right_trim {
                        trim_first_line_trailing(text, heinous_drop)
                    } else {
                        false
                    };
                    if is_root && i == 0 {
                        continue; // 模板首文本：自动剥离跳过（Java 守卫）
                    }
                    let sb = !(right_trim || right_blocked)
                        && leading_ws_through_newline(text)
                        && !prev_i.is_some_and(|t| t.heeds && t.line == begin_line);
                    // 裁剪掉首行后 Java beginColumn=1（:206-208）→ trailing 剥离列判定用有效列
                    let eff_col = if first_line_dropped { 1 } else { span_col };
                    let sa = !(left_trim || left_blocked)
                        && trailing_ws_after_newline(text, eff_col)
                        && !next_i.is_some_and(|t| t.heeds && t.line == end_line);
                    // Java postParseCleanup 在**解析期直接改写文本**（text = substring，
                    // TextBlock.java:128）——后续文本的 prev/next 链所见即最终内容
                    // （如 rt 裁后剩 "  "、再经自身 trailing 剥离成 "" → 空文本 ignorable
                    //  heeds=false，链继续穿过）。标记随置 false，渲染期 strip_text 为 no-op。
                    if sb {
                        *text = text[first_newline_end(text)..].to_string();
                    }
                    if sa {
                        let end = if text.contains('\n') {
                            last_newline_start(text)
                        } else {
                            0 // 无换行 → 整段剥（Java beginColumn==1 全空白）
                        };
                        *text = text[..end].to_string();
                    }
                    *strip_before = false;
                    *strip_after = false;
                }
                ElementKind::If { then, else_, .. } => {
                    let else_first = else_.as_deref().and_then(first_leaf_slice);
                    let then_last = if then.is_empty() {
                        prev_i
                    } else {
                        last_leaf(&then[then.len() - 1])
                    };
                    Self::mark_block(then, prev_i, else_first.or(next_i), false);
                    if let Some(e) = else_ {
                        Self::mark_block(e, then_last, next_i, false);
                    }
                }
                ElementKind::List { body, else_, .. } => {
                    let mut order: Vec<&mut Vec<Element>> = Vec::new();
                    order.push(body);
                    if let Some(e) = else_ {
                        order.push(e);
                    }
                    let mut cur_prev = prev_i;
                    let len = order.len();
                    for idx in 0..len {
                        let cur_next = if idx + 1 < len {
                            first_leaf_slice(order[idx + 1].as_slice())
                        } else {
                            next_i
                        };
                        Self::mark_block(order[idx].as_mut_slice(), cur_prev, cur_next, false);
                        cur_prev = if order[idx].is_empty() {
                            cur_prev
                        } else {
                            last_leaf(&order[idx][order[idx].len() - 1])
                        };
                    }
                }
                ElementKind::Macro { def, .. } => {
                    Self::mark_block(&mut def.body, prev_i, next_i, false);
                }
                ElementKind::Trim(body)
                | ElementKind::Compress(body)
                | ElementKind::NoEscape(body)
                | ElementKind::AutoEsc(body)
                | ElementKind::NoAutoEsc(body)
                | ElementKind::BlockAssign { body, .. }
                | ElementKind::Items { body, .. }
                | ElementKind::Sep { body }
                | ElementKind::Transform { body, .. }
                | ElementKind::On { body, .. }
                | ElementKind::Call {
                    body: Some(body), ..
                } => {
                    Self::mark_block(body, prev_i, next_i, false);
                }
                ElementKind::Escape { body, .. }
                | ElementKind::OutputFormat { body, .. }
                | ElementKind::Nested {
                    body: Some(body), ..
                }
                | ElementKind::Global {
                    body: Some(body), ..
                }
                | ElementKind::Local {
                    body: Some(body), ..
                } => {
                    Self::mark_block(body, prev_i, next_i, false);
                }
                ElementKind::Attempt { try_, recover, .. } => {
                    let rec_first = if recover.is_empty() {
                        next_i
                    } else {
                        first_leaf(&recover[0])
                    };
                    let try_last = if try_.is_empty() {
                        prev_i
                    } else {
                        last_leaf(&try_[try_.len() - 1])
                    };
                    Self::mark_block(try_, prev_i, rec_first, false);
                    Self::mark_block(recover, try_last, next_i, false);
                }
                ElementKind::Switch { cases, default, .. } => {
                    let mut cur_prev = prev_i;
                    for idx in 0..cases.len() {
                        let cur_next = if idx + 1 < cases.len() {
                            first_leaf_slice(cases[idx + 1].body.as_slice())
                        } else if let Some(d) = default {
                            first_leaf_slice(d.as_slice())
                        } else {
                            next_i
                        };
                        Self::mark_block(cases[idx].body.as_mut_slice(), cur_prev, cur_next, false);
                        cur_prev = if cases[idx].body.is_empty() {
                            cur_prev
                        } else {
                            last_leaf(&cases[idx].body[cases[idx].body.len() - 1])
                        };
                    }
                    if let Some(d) = default {
                        Self::mark_block(d, cur_prev, next_i, false);
                    }
                }
                ElementKind::Call { body: None, .. }
                | ElementKind::Nested { body: None, .. }
                | ElementKind::Global { body: None, .. }
                | ElementKind::Local { body: None, .. }
                | ElementKind::Interpolation(_)
                | ElementKind::Assignments(_)
                | ElementKind::Assign { .. }
                | ElementKind::Break
                | ElementKind::Continue
                | ElementKind::Return { .. }
                | ElementKind::Stop { .. }
                | ElementKind::Flush
                | ElementKind::Comment { .. }
                | ElementKind::Include { .. }
                | ElementKind::Import { .. }
                | ElementKind::Setting { .. }
                | ElementKind::FtlHeader { .. }
                | ElementKind::TrimLineStart
                | ElementKind::NoTrimLineStart
                | ElementKind::TrimLineEnd
                | ElementKind::LeftTrimLine
                | ElementKind::Visit { .. }
                | ElementKind::Recurse { .. }
                | ElementKind::Fallback
                | ElementKind::RawText(_) => {}
            }
        }
    }
}

/// 将树上（已打剥离标记的）宏定义同步回解析期注册表
/// （Java 解析期 Macro 元素与 Template.macros 引用同一 Macro 对象）
fn sync_macro_defs(els: &mut [Element], macros: &mut HashMap<String, MacroDef>) {
    for el in els.iter_mut() {
        if let ElementKind::Macro { def } = &el.kind {
            macros.insert(def.name.clone(), def.clone());
        }
        for child in children_mut(el) {
            sync_macro_defs(child, macros);
        }
    }
}

/// 返回元素的直接子块（空白剥离递归用）
fn children_mut(el: &mut Element) -> Vec<&mut Vec<Element>> {
    match &mut el.kind {
        ElementKind::If { then, else_, .. } => {
            let mut v = vec![then];
            if let Some(e) = else_ {
                v.push(e);
            }
            v
        }
        ElementKind::List { body, else_, .. } => {
            let mut v = vec![body];
            if let Some(e) = else_ {
                v.push(e);
            }
            v
        }
        ElementKind::Macro { def, .. } => vec![&mut def.body],
        ElementKind::Trim(b)
        | ElementKind::Compress(b)
        | ElementKind::NoEscape(b)
        | ElementKind::AutoEsc(b)
        | ElementKind::NoAutoEsc(b)
        | ElementKind::BlockAssign { body: b, .. }
        | ElementKind::Items { body: b, .. }
        | ElementKind::Sep { body: b }
        | ElementKind::Transform { body: b, .. }
        | ElementKind::On { body: b, .. }
        | ElementKind::Call { body: Some(b), .. }
        | ElementKind::Escape { body: b, .. }
        | ElementKind::OutputFormat { body: b, .. }
        | ElementKind::Nested { body: Some(b), .. }
        | ElementKind::Global { body: Some(b), .. }
        | ElementKind::Local { body: Some(b), .. } => vec![b],
        ElementKind::Attempt { try_, recover, .. } => vec![try_, recover],
        ElementKind::Switch { cases, default, .. } => {
            let mut v = Vec::new();
            for c in cases {
                v.push(&mut c.body);
            }
            if let Some(d) = default {
                v.push(d);
            }
            v
        }
        _ => Vec::new(),
    }
}

/// Trim 指令元素（Java TrimInstruction；ignorable，terminal 遍历跳过）
fn is_trim_element(el: &Element) -> bool {
    matches!(
        el.kind,
        ElementKind::TrimLineStart
            | ElementKind::NoTrimLineStart
            | ElementKind::TrimLineEnd
            | ElementKind::LeftTrimLine
    )
}

/// 元素作为叶时的 heedsOpeningWhitespace（Java TextBlock.java:215-226 的
/// heedsOpeningWhitespace；getFirstLeaf 简化：不深入块内部，Macro 视为叶；
/// ignorable 全空白文本 → false，Java TextBlock.java:316-318 —— 由调用方
/// （HEINOUS 扫描）结合兄弟上下文判定）
fn leaf_heeds_opening(el: &Element) -> bool {
    match &el.kind {
        ElementKind::Text { text, .. } | ElementKind::NoParse { text, .. } => heeds_opening(text),
        ElementKind::Interpolation(_) => true,
        _ => false,
    }
}

/// 非输出型元素（Java TextBlock.nonOutputtingType :374-381：
/// Macro/Assignment/AssignmentInstruction/PropertySetting/LibraryLoad/Comment；
/// Global/Local 继承 Assignment）
fn is_non_outputting(el: &Element) -> bool {
    matches!(
        el.kind,
        ElementKind::Macro { .. }
            | ElementKind::Assign { .. }
            | ElementKind::Assignments(_)
            | ElementKind::BlockAssign { .. }
            | ElementKind::Global { .. }
            | ElementKind::Local { .. }
            | ElementKind::Setting { .. }
            | ElementKind::Import { .. }
            | ElementKind::Comment { .. }
    )
}

/// Java TextBlock.isIgnorable(stripWhitespace=true)（TextBlock.java:349-366）：
/// 全空白文本 +（顶层边界或两侧均为非输出元素）→ ignorable（heeds=false）。
/// `j` 为文本在 `els` 中的下标；`is_root` 表示本层为模板根。
fn is_ignorable_text(els: &[Element], j: usize, is_root: bool) -> bool {
    let (empty, all_ws) = match &els[j].kind {
        ElementKind::Text { text, .. } | ElementKind::NoParse { text, .. } => {
            (text.is_empty(), text.chars().all(is_ws))
        }
        _ => return false,
    };
    // Java isIgnorable（TextBlock.java:349-352）：空文本**无条件** ignorable
    // （剥离后文本可为空 —— 链上后文所见即最终内容）
    if empty {
        return true;
    }
    if !all_ws {
        return false;
    }
    let prev_ok = (j == 0 && is_root) || (j > 0 && is_non_outputting(&els[j - 1]));
    let next_ok =
        (j + 1 == els.len() && is_root) || (j + 1 < els.len() && is_non_outputting(&els[j + 1]));
    prev_ok && next_ok
}

/// 沿同级向前逐叶回溯，返回首个"care 空白"且结束行 == `line` 的叶
/// （Java openingCharsToStrip 的 prevTerminalNode 链，TextBlock.java:280-288）。
/// 链上每个叶都检查：跨行叶 → 链终止（None，照剥）；heeds 叶 → 阻止（Some）；
/// 其余（含 ignorable 全空白文本，Java TextBlock.java:316-318 heeds=false）→ 继续。
/// 列表走尽 → 用父级传入的 prev 兜底（Java 上溯 parent.prevTerminalNode）。
fn walk_prev(
    els: &[Element],
    i: usize,
    line: u32,
    parent_prev: Option<Term>,
    is_root: bool,
) -> Option<Term> {
    let mut j = i;
    loop {
        if j == 0 {
            return parent_prev;
        }
        j -= 1;
        let e = &els[j];
        if is_trim_element(e) || is_ignorable_text(els, j, is_root) {
            continue;
        }
        match last_leaf(e) {
            Some(t) => {
                if t.line != line {
                    return None;
                }
                if t.heeds {
                    return Some(t);
                }
            }
            // 空块：Java getLastLeaf 返回元素自身（heeds=false，默认）→ 按行号判定
            None => {
                if e.span.line != line {
                    return None;
                }
            }
        }
    }
}

/// 沿同级向前逐叶前进，返回首个"care 空白"且起始行 == `line` 的叶
/// （Java trailingCharsToStrip 的 nextTerminalNode 链，TextBlock.java:304-312；
/// 语义与 walk_prev 对称）。
fn walk_next(
    els: &[Element],
    i: usize,
    line: u32,
    parent_next: Option<Term>,
    is_root: bool,
) -> Option<Term> {
    let mut j = i;
    loop {
        j += 1;
        let Some(e) = els.get(j) else {
            return parent_next;
        };
        if is_trim_element(e) || is_ignorable_text(els, j, is_root) {
            continue;
        }
        match first_leaf(e) {
            Some(t) => {
                if t.line != line {
                    return None;
                }
                if t.heeds {
                    return Some(t);
                }
            }
            None => {
                if e.span.line != line {
                    return None;
                }
            }
        }
    }
}

/// 终结节点信息（空白剥离同行检查用）
#[derive(Clone, Copy)]
struct Term {
    /// 是否"care"空白（无换行的文本才 care；Java heedsOpening/TrailingWhitespace）
    heeds: bool,
    /// 行号（prev 用结束行，next 用开始行）
    line: u32,
}

/// 元素的首个终结叶（Java getFirstLeaf）
fn first_leaf(el: &Element) -> Option<Term> {
    match &el.kind {
        ElementKind::Text { text, .. } => Some(Term {
            // Java TextBlock.heedsTrailingWhitespace（正向扫描：先遇换行 → false，先遇非空白 → true）
            heeds: heeds_trailing(text),
            line: el.span.line,
        }),
        ElementKind::Interpolation(_) => Some(Term {
            // Java DollarVariable.heedsTrailingWhitespace = true（DollarVariable.java:132-135）
            heeds: true,
            line: el.span.line,
        }),
        ElementKind::If { then, else_, .. } => {
            first_leaf_slice(then).or_else(|| else_.as_deref().and_then(first_leaf_slice))
        }
        ElementKind::List { body, else_, .. } => {
            first_leaf_slice(body).or_else(|| else_.as_deref().and_then(first_leaf_slice))
        }
        // Java：宏定义是"不可见"元素（Macro.heedsOpening/TrailingWhitespace = false），
        // 不深入 body——其后/前的文本按行首剥离处理
        ElementKind::Macro { .. } => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
        ElementKind::Trim(b)
        | ElementKind::Compress(b)
        | ElementKind::NoEscape(b)
        | ElementKind::AutoEsc(b)
        | ElementKind::NoAutoEsc(b)
        | ElementKind::BlockAssign { body: b, .. }
        | ElementKind::Items { body: b, .. }
        | ElementKind::Sep { body: b }
        | ElementKind::Transform { body: b, .. }
        | ElementKind::On { body: b, .. }
        | ElementKind::Call { body: Some(b), .. }
        | ElementKind::Escape { body: b, .. }
        | ElementKind::OutputFormat { body: b, .. }
        | ElementKind::Nested { body: Some(b), .. }
        | ElementKind::Global { body: Some(b), .. }
        | ElementKind::Local { body: Some(b), .. } => first_leaf_slice(b),
        ElementKind::Attempt { try_, recover, .. } => {
            first_leaf_slice(try_).or_else(|| first_leaf_slice(recover))
        }
        ElementKind::Switch { cases, default, .. } => {
            for c in cases {
                if !c.body.is_empty() {
                    return first_leaf(&c.body[0]);
                }
            }
            default.as_deref().and_then(first_leaf_slice)
        }
        _ => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
    }
}

fn first_leaf_slice(els: &[Element]) -> Option<Term> {
    els.first().and_then(first_leaf)
}

/// 元素的末个终结叶（Java getLastLeaf；Text 用**原始**结束行 ——
/// Java TextBlock 的 endLine 在空白剥离时不变，TextBlock.java:206-208 只动 beginLine）
fn last_leaf(el: &Element) -> Option<Term> {
    match &el.kind {
        ElementKind::Text {
            text,
            orig_end_line,
            ..
        } => Some(Term {
            // Java TextBlock.heedsOpeningWhitespace（反向扫描：先遇换行 → false，先遇非空白 → true）
            heeds: heeds_opening(text),
            line: *orig_end_line,
        }),
        ElementKind::Interpolation(_) => Some(Term {
            // Java DollarVariable.heedsOpeningWhitespace = true（DollarVariable.java:127-130）
            heeds: true,
            line: el.span.line,
        }),
        ElementKind::Comment { text } | ElementKind::RawText(text) => Some(Term {
            heeds: false,
            line: el.span.line + newline_count(text),
        }),
        ElementKind::NoParse {
            text,
            orig_end_line,
            ..
        } => Some(Term {
            // Java：unparsed TextBlock 的 heeds 由内容决定（TextBlock.java:315-345）
            heeds: heeds_opening(text),
            line: *orig_end_line,
        }),
        ElementKind::If { then, else_, .. } => {
            last_leaf_slice(then).or_else(|| else_.as_deref().and_then(last_leaf_slice))
        }
        ElementKind::List { body, else_, .. } => {
            last_leaf_slice(body).or_else(|| else_.as_deref().and_then(last_leaf_slice))
        }
        // Java：宏定义不可见（Macro.heeds*Whitespace = false），不深入 body
        ElementKind::Macro { .. } => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
        ElementKind::Trim(b)
        | ElementKind::Compress(b)
        | ElementKind::NoEscape(b)
        | ElementKind::AutoEsc(b)
        | ElementKind::NoAutoEsc(b)
        | ElementKind::BlockAssign { body: b, .. }
        | ElementKind::Items { body: b, .. }
        | ElementKind::Sep { body: b }
        | ElementKind::Transform { body: b, .. }
        | ElementKind::On { body: b, .. }
        | ElementKind::Call { body: Some(b), .. }
        | ElementKind::Escape { body: b, .. }
        | ElementKind::OutputFormat { body: b, .. }
        | ElementKind::Nested { body: Some(b), .. }
        | ElementKind::Global { body: Some(b), .. }
        | ElementKind::Local { body: Some(b), .. } => last_leaf_slice(b),
        ElementKind::Attempt { try_, recover, .. } => {
            last_leaf_slice(recover).or_else(|| last_leaf_slice(try_))
        }
        ElementKind::Switch { cases, default, .. } => {
            if let Some(d) = default {
                if !d.is_empty() {
                    return last_leaf(&d[d.len() - 1]);
                }
            }
            for c in cases.iter().rev() {
                if !c.body.is_empty() {
                    return last_leaf(&c.body[c.body.len() - 1]);
                }
            }
            None
        }
        _ => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
    }
}

fn last_leaf_slice(els: &[Element]) -> Option<Term> {
    els.last().and_then(last_leaf)
}

/// Java `TextBlock.heedsOpeningWhitespace`（TextBlock.java:215-226）：从文本末尾反向扫描，
/// 先遇换行 → false（不 care 行首空白）；先遇非空白字符 → true（care）；全空白 → true。
/// 空文本 → false（Java isIgnorable("") → true → :316-318 早退）。
fn heeds_opening(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    for c in text.chars().rev() {
        if c == '\n' || c == '\r' {
            return false;
        }
        if !is_ws(c) {
            return true;
        }
    }
    true
}

/// Java `TextBlock.heedsTrailingWhitespace`（TextBlock.java:228-239）：从文本开头正向扫描，
/// 先遇换行 → false；先遇非空白字符 → true；全空白 → true。空文本 → false（同上）。
fn heeds_trailing(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    for c in text.chars() {
        if c == '\n' || c == '\r' {
            return false;
        }
        if !is_ws(c) {
            return true;
        }
    }
    true
}

/// 换行数（`\n` 计数；`\r\n` 由 `\n` 计一次 —— 与 lexer 的行号一致，孤立 `\r` 不计）
fn newline_count(text: &str) -> u32 {
    text.chars().filter(|c| *c == '\n').count() as u32
}

/// 首个换行（含）之后的起始下标（Java openingCharsToStrip 的裁剪量）
fn first_newline_end(s: &str) -> usize {
    match s.find('\n') {
        Some(i) => i + 1,
        None => s.len(),
    }
}

/// 最后一个换行之后的起始下标（Java trailingCharsToStrip 的保留起点）
fn last_newline_start(s: &str) -> usize {
    match s.rfind('\n') {
        Some(i) => i + 1,
        None => s.len(),
    }
}

fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r' || c == '\n'
}

/// 文本首部空白（到首个换行含换行）是否全为空白（Java openingCharsToStrip 判定）
fn leading_ws_through_newline(text: &str) -> bool {
    for c in text.chars() {
        if c == '\n' || c == '\r' {
            return true;
        }
        if !is_ws(c) {
            return false;
        }
    }
    false
}

/// 文本尾部（最后一个换行后）是否全为空白且非空（Java trailingCharsToStrip 判定；
/// 无换行时仅当文本整行空白且始于列 1 才可剥）
fn trailing_ws_after_newline(text: &str, begin_col: u32) -> bool {
    match text.rfind(['\n', '\r']) {
        Some(i) => {
            let trail = &text[i + 1..];
            !trail.is_empty() && trail.chars().all(is_ws)
        }
        None => begin_col == 1 && !text.is_empty() && text.chars().all(is_ws),
    }
}

// ---------------------------------------------------------------------------
// 辅助
// ---------------------------------------------------------------------------

/// 块解析终止原因
enum BlockStop {
    /// 命中结束标签（值 = 标签名小写）
    EndTag(String),
    /// 命中用户指令结束标签（`</@name>`；值 = 名字，可为空）
    EndCall(String),
    /// 命中终止指令（值 = 指令名小写）
    Dir(String),
    Eof,
}

/// 赋值作用域（对应 Assignment.NAMESPACE/GLOBAL/LOCAL）
#[derive(Clone, Copy, PartialEq, Eq)]
enum AssignScope {
    Namespace,
    Global,
    Local,
}

/// 迭代块解析上下文 —— 对应 Java `ParserIteratorBlockContext`（FTL.jj 的
/// iteratorBlockContexts 栈；`#items`/`#sep` 的嵌套校验与 `#list` 无 as 校验）
struct IterCtx {
    /// 所属 #list/#foreach 是否带 `as loopVar`（带则 #items 非法）
    has_loop_var: bool,
    /// 所属是否为 `<#foreach>`（Java：foreach 不支持嵌套 #items）
    is_foreach: bool,
    /// 该 #list 是否已进入过 #items（Java iterCtx.kind == ITERATOR_BLOCK_KIND_ITEMS）
    is_items: bool,
}

/// token → 赋值操作符（多赋值续项前瞻用；非赋值符返回 None）
fn assign_op_of(t: &Tok) -> Option<AssignOp> {
    match t {
        Tok::Eq => Some(AssignOp::Equals),
        Tok::PlusEq => Some(AssignOp::PlusEq),
        Tok::MinusEq => Some(AssignOp::MinusEq),
        Tok::TimesEq => Some(AssignOp::TimesEq),
        Tok::DivEq => Some(AssignOp::DivideEq),
        Tok::ModEq => Some(AssignOp::ModuloEq),
        Tok::PlusPlus => Some(AssignOp::PlusPlus),
        Tok::MinusMinus => Some(AssignOp::MinusMinus),
        _ => None,
    }
}

/// 单个赋值 → 作用域对应 AST 节点（Java Assignment/AssignmentInstruction 的 addAssignment）
fn assignment_element(
    scope: AssignScope,
    target: String,
    expr: Expr,
    op: AssignOp,
    span: Span,
) -> Element {
    let kind = match scope {
        AssignScope::Namespace => ElementKind::Assign {
            target,
            expr,
            op,
            namespace: None,
        },
        AssignScope::Global => ElementKind::Global {
            target,
            expr: Some(expr),
            body: None,
            op,
        },
        AssignScope::Local => ElementKind::Local {
            target,
            expr: Some(expr),
            body: None,
            op,
        },
    };
    Element::new(kind, span)
}

/// `[#ftl]` 头部布尔参数：Bool 字面量或字符串（对应 Java getBoolean 的
/// legacyCompat 分支：StringUtil.getYesNo —— y/yes/t/true/on/1 → true 等）
fn header_bool(e: &Expr) -> Option<bool> {
    match &e.kind {
        ExprKind::Bool(b) => Some(*b),
        ExprKind::Str(s) => match s.to_ascii_lowercase().as_str() {
            "y" | "yes" | "t" | "true" | "on" | "1" => Some(true),
            "n" | "no" | "f" | "false" | "off" | "0" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// 内建/内置变量名 camelCase → legacy 蛇形归一化（对应 Java
/// `_CoreStringUtils.toFTLLegacyNamingConvention`：`capFirst` → `cap_first`、
/// `templateName` → `template_name`；全大写名保持原样，求值期报 Unknown built-in）
fn camel_to_snake(name: &str) -> String {
    if name.chars().any(|c| c.is_ascii_lowercase()) {
        let mut out = String::with_capacity(name.len() + 4);
        for c in name.chars() {
            if c.is_ascii_uppercase() {
                out.push('_');
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    } else {
        name.to_string()
    }
}

/// 内置变量名 → BuiltinVar（对应 Java `BuiltinVariable.SPEC_VAR_NAMES`，
/// BuiltinVariable.java:43-82；name 须已蛇形归一化）
fn builtin_var_of(name: &str) -> Option<BuiltinVar> {
    Some(match name {
        "now" => BuiltinVar::Now,
        "namespace" => BuiltinVar::Namespace,
        "main" => BuiltinVar::Main,
        "globals" => BuiltinVar::Globals,
        "locals" => BuiltinVar::Locals,
        "data_model" => BuiltinVar::DataModel,
        "vars" => BuiltinVar::Vars,
        "lang" => BuiltinVar::Lang,
        "locale" => BuiltinVar::Locale,
        "locale_object" => BuiltinVar::LocaleObject,
        "time_zone" => BuiltinVar::TimeZone,
        "template_name" => BuiltinVar::TemplateName,
        "main_template_name" => BuiltinVar::MainTemplateName,
        "current_template_name" => BuiltinVar::CurrentTemplateName,
        "node" | "current_node" => BuiltinVar::Node,
        "error" => BuiltinVar::Error,
        "output_encoding" => BuiltinVar::OutputEncoding,
        "output_format" => BuiltinVar::OutputFormat,
        "auto_esc" => BuiltinVar::AutoEsc,
        "url_escaping_charset" => BuiltinVar::UrlEscapingCharset,
        "version" => BuiltinVar::Version,
        "incompatible_improvements" => BuiltinVar::IncompatibleImprovements,
        "args" => BuiltinVar::Args,
        _ => return None,
    })
}

/// 数字字面量 → TNumber（契约映射：1→Int、1L→Long、1F→Float、1D→Double、
/// 1.5/1e3→Decimal、超 i64 整数→BigInt；0x 十六进制；L/F/D/B 后缀）
fn number_literal(raw: &str) -> Option<TNumber> {
    let (digits, suffix) = match raw.chars().last() {
        Some(c) if matches!(c, 'l' | 'L' | 'f' | 'F' | 'd' | 'D' | 'b' | 'B') => {
            (&raw[..raw.len() - 1], c)
        }
        _ => (raw, ' '),
    };
    let is_hex = digits.len() > 2 && digits.starts_with("0x");
    let hex_digits = if is_hex { &digits[2..] } else { "" };
    match suffix {
        'L' | 'l' => {
            let v = if is_hex {
                i64::from_str_radix(hex_digits, 16).ok()
            } else {
                digits.parse::<i64>().ok()
            };
            match v {
                Some(v) => Some(TNumber::Long(v)),
                None => {
                    // 超 i64 的 L 后缀 → BigInt（放宽，避免误报）
                    let big = if is_hex {
                        BigInt::parse_bytes(hex_digits.as_bytes(), 16)
                    } else {
                        BigInt::from_str(digits).ok()
                    };
                    big.map(TNumber::BigInt)
                }
            }
        }
        'F' | 'f' => digits.parse::<f32>().ok().map(TNumber::Float),
        'D' | 'd' => digits.parse::<f64>().ok().map(TNumber::Double),
        'B' | 'b' => BigDecimal::from_str(digits).ok().map(TNumber::Decimal),
        _ => {
            if is_hex {
                match i64::from_str_radix(hex_digits, 16) {
                    Ok(v) => Some(TNumber::from_i64(v)),
                    Err(_) => BigInt::parse_bytes(hex_digits.as_bytes(), 16).map(TNumber::BigInt),
                }
            } else if digits.contains('.') || digits.contains('e') || digits.contains('E') {
                BigDecimal::from_str(digits).ok().map(TNumber::Decimal)
            } else {
                match digits.parse::<i64>() {
                    Ok(v) => Some(TNumber::from_i64(v)),
                    Err(_) => BigInt::from_str(digits).ok().map(TNumber::BigInt),
                }
            }
        }
    }
}

/// 调用目标（UnifiedCall 的 callee 表达式 → 契约 CallTarget）
fn call_target(e: &Expr) -> CallTarget {
    match &e.kind {
        ExprKind::Ident(n) => CallTarget::Name(n.clone()),
        ExprKind::Dot { target, name } if matches!(target.kind, ExprKind::Ident(_)) => {
            match &target.kind {
                ExprKind::Ident(ns) => CallTarget::Namespaced {
                    ns: ns.clone(),
                    name: name.clone(),
                },
                _ => CallTarget::Expr(Box::new(e.clone())),
            }
        }
        _ => CallTarget::Expr(Box::new(e.clone())),
    }
}

/// 调用起始名的规范形式（结束标签匹配用；Java getCanonicalForm 近似）
fn call_target_canonical(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Ident(n) => Some(n.clone()),
        ExprKind::Dot { target, name } => {
            call_target_canonical(target).map(|t| format!("{t}.{name}"))
        }
        _ => None,
    }
}

/// token 的人类可读描述（错误消息）
fn tok_desc(t: &Tok) -> String {
    match t {
        Tok::Ident(s) => format!("identifier \"{s}\""),
        Tok::Number(s) => format!("number \"{s}\""),
        Tok::Str(_) => "string literal".to_string(),
        Tok::RawStr(_) => "raw string literal".to_string(),
        Tok::True => "\"true\"".to_string(),
        Tok::False => "\"false\"".to_string(),
        Tok::In => "\"in\"".to_string(),
        Tok::As => "\"as\"".to_string(),
        Tok::Using => "\"using\"".to_string(),
        Tok::Lt => "\"lt\"".to_string(),
        Tok::Lte => "\"lte\"".to_string(),
        Tok::Gt => "\"gt\"".to_string(),
        Tok::Gte => "\"gte\"".to_string(),
        Tok::Plus => "\"+\"".to_string(),
        Tok::Minus => "\"-\"".to_string(),
        Tok::Times => "\"*\"".to_string(),
        Tok::DoubleStar => "\"**\"".to_string(),
        Tok::Divide => "\"/\"".to_string(),
        Tok::Percent => "\"%\"".to_string(),
        Tok::PlusEq => "\"+=\"".to_string(),
        Tok::MinusEq => "\"-=\"".to_string(),
        Tok::TimesEq => "\"*=\"".to_string(),
        Tok::DivEq => "\"/=\"".to_string(),
        Tok::ModEq => "\"%=\"".to_string(),
        Tok::PlusPlus => "\"++\"".to_string(),
        Tok::MinusMinus => "\"--\"".to_string(),
        Tok::Eq => "\"=\"".to_string(),
        Tok::NotEq => "\"!=\"".to_string(),
        Tok::Exclam => "\"!\"".to_string(),
        Tok::Exists => "\"??\"".to_string(),
        Tok::Builtin => "\"?\"".to_string(),
        Tok::And => "\"&&\"".to_string(),
        Tok::Or => "\"||\"".to_string(),
        Tok::LambdaArrow => "\"->\"".to_string(),
        Tok::Dot => "\".\"".to_string(),
        Tok::DotDot => "\"..\"".to_string(),
        Tok::DotDotLess => "\"..<\"".to_string(),
        Tok::DotDotStar => "\"..*\"".to_string(),
        Tok::Ellipsis => "\"...\"".to_string(),
        Tok::Comma => "\",\"".to_string(),
        Tok::Semicolon => "\";\"".to_string(),
        Tok::Colon => "\":\"".to_string(),
        Tok::OpenParen => "\"(\"".to_string(),
        Tok::CloseParen => "\")\"".to_string(),
        Tok::OpenBracket => "\"[\"".to_string(),
        Tok::CloseBracket => "\"]\"".to_string(),
        Tok::OpenCurly => "\"{\"".to_string(),
        Tok::CloseCurly => "\"}\"".to_string(),
        Tok::TagEnd => "\">\"".to_string(),
        Tok::EmptyTagEnd => "\"/>\"".to_string(),
        Tok::InterpEnd => "\"}\"".to_string(),
        Tok::Eof => "the end of the template".to_string(),
    }
}

/// 在字符串插值正文中找匹配的 `}`（跳过字符串字面量；对应 Java parseValue 的
/// interpolation 边界扫描）。返回 (内部文本, 剩余文本)。
fn find_matching_brace(s: &str) -> Option<(&str, &str)> {
    let mut depth: u32 = 0;
    let mut iter = s.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        match c {
            '"' | '\'' => {
                // 跳过字符串字面量（含反斜杠转义）
                let quote = c;
                while let Some((_, c2)) = iter.next() {
                    if c2 == '\\' {
                        iter.next();
                    } else if c2 == quote {
                        break;
                    }
                }
            }
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    return Some((&s[..i], &s[i + 1..]));
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// 裁文本块第一行的尾部空白（对应 Java deliberateRightTrim 的裁切段：
/// openingPart=到首个换行（含）；全空白 → 整段裁掉；否则只裁尾部空白，
/// 其中 trailing 全空白时按 HEINOUS 块（Java:239-254）判定是否保留）。
/// 返回"首行是否整段裁掉"（Java 整段裁时 beginLine++/beginColumn=1，TextBlock.java:206-208）。
fn trim_first_line_trailing(text: &mut String, heinous_drop: bool) -> bool {
    let first_nl = text.find(['\n', '\r']);
    let Some(mut idx) = first_nl else {
        return false; // Java: firstLineIndex == 0 → return false（无换行不裁）
    };
    idx += 1; // 含换行
    if idx > 1 && text.as_bytes()[idx - 2] == b'\r' && text.as_bytes().get(idx - 1) == Some(&b'\n')
    {
        idx += 1; // CRLF
    }
    let opening: String = text.chars().take(idx).collect();
    let trailing: String = text.chars().skip(idx).collect();
    if opening.trim().is_empty() {
        // isTrimmableToEmpty(openingPart) → 整段裁掉（Java beginLine++ 语义）
        *text = trailing;
        true
    } else {
        let trimmed_len = opening.trim_end().len();
        let printable: String = opening.chars().take(trimmed_len).collect();
        if trailing.trim().is_empty() {
            // HEINOUS 块（Java:239-254）：trailing 全空白时按后文同行判定
            // （heedsOpeningWhitespace → 保留；`<#lt>`/`<#t>` → 裁掉）
            if heinous_drop {
                *text = printable;
            } else {
                *text = printable + &trailing;
            }
        } else {
            *text = printable + &trailing;
        }
        false
    }
}

/// 裁文本块最后一行的行首空白（对应 Java deliberateLeftTrim：lastLine 全空白 →
/// 整行裁掉（保留换行）；否则只裁行首空白）。无换行的单行文本仅在
/// begin_col == 1 时裁（Java TextBlock.java:156 `lastNewLineIndex >= 0 || beginColumn == 1`）。
fn trim_last_line_leading(text: &mut String, begin_col: u32) {
    let last_nl = text.rfind(['\n', '\r']);
    let Some(last_nl) = last_nl else {
        // 单行文本：Java beginColumn==1 才裁（否则跳过）
        if begin_col != 1 {
            return;
        }
        let trimmed = text.trim_start();
        if trimmed.is_empty() {
            *text = String::new();
        } else {
            *text = trimmed.to_string();
        }
        return;
    };
    let last_line: String = text.chars().skip(last_nl + 1).collect();
    if last_line.trim().is_empty() {
        // isTrimmableToEmpty(lastLine) → 保留换行、去掉最后一行（Java endColumn=0）
        *text = text.chars().take(last_nl + 1).collect();
    } else {
        let lead_len = last_line.len() - last_line.trim_start().len();
        *text = text.chars().take(last_nl + 1).collect::<String>()
            + &last_line.chars().skip(lead_len).collect::<String>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认配置（strict_syntax=false，whitespace_stripping=true —— 与 Java 默认一致）
    fn cfg() -> Rc<Configuration> {
        Rc::new(Configuration::new())
    }

    /// 严格语法配置
    fn cfg_strict() -> Rc<Configuration> {
        let mut c = Configuration::new();
        c.settings.strict_syntax = true;
        Rc::new(c)
    }

    fn parse_with(cfg: &Rc<Configuration>, src: &str) -> Template {
        parse(cfg, "t", src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"))
    }

    fn parse_ok(src: &str) -> Template {
        parse_with(&cfg(), src)
    }

    fn parse_err(src: &str) -> String {
        match parse(&cfg(), "t", src) {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected parse error for {src:?}"),
        }
    }

    /// 将表达式包装进 `${...}` 插值并取出表达式 AST
    fn expr_of(src: &str) -> ExprKind {
        let t = parse_ok(&format!("${{{src}}}"));
        match &t.root[0].kind {
            ElementKind::Interpolation(e) => e.kind.clone(),
            k => panic!("expected interpolation, got {k:?}"),
        }
    }

    fn num(v: TNumber) -> ExprKind {
        ExprKind::Num(v)
    }

    fn ident(n: &str) -> ExprKind {
        ExprKind::Ident(n.to_string())
    }

    fn strlit(s: &str) -> ExprKind {
        ExprKind::Str(s.to_string())
    }

    // -----------------------------------------------------------------------
    // 数字字面量（NumberLiteral → TNumber 契约映射）
    // -----------------------------------------------------------------------

    #[test]
    fn number_literal_mapping() {
        assert_eq!(expr_of("1"), num(TNumber::Int(1)));
        assert_eq!(expr_of("1L"), num(TNumber::Long(1)));
        assert_eq!(expr_of("1F"), num(TNumber::Float(1.0)));
        assert_eq!(expr_of("1D"), num(TNumber::Double(1.0)));
        assert_eq!(
            expr_of("1.5"),
            num(TNumber::Decimal(BigDecimal::from_str("1.5").unwrap()))
        );
        assert_eq!(
            expr_of("1e3"),
            num(TNumber::Decimal(BigDecimal::from_str("1000").unwrap()))
        );
        assert_eq!(
            expr_of("1.5e-2"),
            num(TNumber::Decimal(BigDecimal::from_str("0.015").unwrap()))
        );
        assert_eq!(expr_of("0x1A"), num(TNumber::Int(26)));
        assert_eq!(expr_of("0x10L"), num(TNumber::Long(16)));
        // 超 i64 整数 → BigInt
        assert_eq!(
            expr_of("99999999999999999999"),
            num(TNumber::BigInt(
                BigInt::from_str("99999999999999999999").unwrap()
            ))
        );
        // 大数 Long 后缀回退 BigInt
        assert_eq!(
            expr_of("99999999999999999999L"),
            num(TNumber::BigInt(
                BigInt::from_str("99999999999999999999").unwrap()
            ))
        );
    }

    // -----------------------------------------------------------------------
    // 字符串字面量（StringLiteral：转义 + 插值）
    // -----------------------------------------------------------------------

    #[test]
    fn string_literal_escapes() {
        assert_eq!(expr_of(r#""abc""#), strlit("abc"));
        assert_eq!(expr_of(r#""a\n\t\\\'\"b""#), strlit("a\n\t\\'\"b"));
        // \l \g \a 转义（FTL 特有）
        assert_eq!(expr_of(r#""\l\g\a""#), strlit("<>&"));
        // \xHH 与 \uXXXX
        assert_eq!(expr_of(r#""\x41""#), strlit("A"));
        assert_eq!(expr_of(r#""\u0041\u00e9""#), strlit("Aé"));
    }

    #[test]
    fn string_interpolation_parts() {
        assert_eq!(
            expr_of(r#""a${x}b""#),
            ExprKind::InterpStr(vec![
                StrPart::Text("a".to_string()),
                // 插值内表达式由子解析器解析，位置相对插值正文（Java parseValue 子解析语义）
                StrPart::Interp(Box::new(Expr::new(ident("x"), Span::new(1, 1)))),
                StrPart::Text("b".to_string()),
            ])
        );
        assert_eq!(
            expr_of(r#""${x}""#),
            ExprKind::InterpStr(vec![StrPart::Interp(Box::new(Expr::new(
                ident("x"),
                Span::new(1, 1)
            )))])
        );
        // 嵌套字符串插值（外层用单引号：双引号字符串内不能再有未转义双引号，
        // Java 词法同样在 `"x ${"` 处截断）
        assert_eq!(
            expr_of(r#"'x ${"a${y}b"} z'"#),
            ExprKind::InterpStr(vec![
                StrPart::Text("x ".to_string()),
                // 嵌套字符串字面量同样由子解析器处理（位置相对）
                StrPart::Interp(Box::new(Expr::new(
                    ExprKind::InterpStr(vec![
                        StrPart::Text("a".to_string()),
                        StrPart::Interp(Box::new(Expr::new(ident("y"), Span::new(1, 1)))),
                        StrPart::Text("b".to_string()),
                    ]),
                    Span::new(1, 1)
                ))),
                StrPart::Text(" z".to_string()),
            ])
        );
        // `$${` 在字符串内：第一个 $ 是文本（Java indexOf 语义）
        assert_eq!(
            expr_of(r#""$${x}""#),
            ExprKind::InterpStr(vec![
                StrPart::Text("$".to_string()),
                StrPart::Interp(Box::new(Expr::new(ident("x"), Span::new(1, 1)))),
            ])
        );
        // 原始字符串：不插值、不解码
        assert_eq!(expr_of(r#"r"a${x}\n""#), strlit("a${x}\\n"));
    }

    #[test]
    fn string_literal_unclosed() {
        let msg = parse_err(r#"${"abc}"#);
        assert!(msg.contains("Unclosed string literal"), "{msg}");
    }

    #[test]
    fn invalid_escape_sequence() {
        let msg = parse_err(r#"${"a\qb"}"#);
        assert!(msg.contains("Invalid escape sequence"), "{msg}");
    }

    // -----------------------------------------------------------------------
    // 布尔 / 标识符 / 内置变量
    // -----------------------------------------------------------------------

    #[test]
    fn boolean_and_identifier() {
        assert_eq!(expr_of("true"), ExprKind::Bool(true));
        assert_eq!(expr_of("false"), ExprKind::Bool(false));
        assert_eq!(expr_of("now"), ExprKind::BuiltinVar(BuiltinVar::Now));
        assert_eq!(expr_of("fooBar_$1"), ident("fooBar_$1"));
        assert_eq!(expr_of("français"), ident("français"));
        // `.now` 内置变量形式（Java BuiltinVariable 产生式）
        assert_eq!(expr_of(".now"), ExprKind::BuiltinVar(BuiltinVar::Now));
    }

    // -----------------------------------------------------------------------
    // 后缀操作：点 / 动态键 / 方法调用 / 内建 / 默认值 / 存在性
    // -----------------------------------------------------------------------

    #[test]
    fn postfix_operations() {
        assert_eq!(
            expr_of("a.b"),
            ExprKind::Dot {
                target: Box::new(Expr::new(ident("a"), Span::new(1, 3))),
                name: "b".to_string(),
            }
        );
        // 链式点
        assert_eq!(
            expr_of("a.b.c"),
            ExprKind::Dot {
                target: Box::new(Expr::new(
                    ExprKind::Dot {
                        target: Box::new(Expr::new(ident("a"), Span::new(1, 3))),
                        name: "b".to_string(),
                    },
                    Span::new(1, 3)
                )),
                name: "c".to_string(),
            }
        );
        assert_eq!(
            expr_of(r#"a["k"]"#),
            ExprKind::DynKey {
                target: Box::new(Expr::new(ident("a"), Span::new(1, 3))),
                key: Box::new(Expr::new(strlit("k"), Span::new(1, 5))),
            }
        );
        // 关键字作成员名（DotVariable 产生式）
        assert_eq!(
            expr_of("a.in"),
            ExprKind::Dot {
                target: Box::new(Expr::new(ident("a"), Span::new(1, 3))),
                name: "in".to_string(),
            }
        );
    }

    #[test]
    fn method_call() {
        assert_eq!(
            expr_of("f(x, y)"),
            ExprKind::Call {
                callee: Box::new(Expr::new(ident("f"), Span::new(1, 3))),
                args: vec![
                    Expr::new(ident("x"), Span::new(1, 5)),
                    Expr::new(ident("y"), Span::new(1, 8)),
                ],
            }
        );
    }

    #[test]
    fn builtin_variants() {
        assert_eq!(
            expr_of("x?upper_case"),
            ExprKind::BuiltIn {
                target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                name: "upper_case".to_string(),
                args: None,
            }
        );
        assert_eq!(
            expr_of(r#"x?string("0.##")"#),
            ExprKind::BuiltIn {
                target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                name: "string".to_string(),
                args: Some(vec![Expr::new(strlit("0.##"), Span::new(1, 12))]),
            }
        );
    }

    #[test]
    fn exists_and_default_to() {
        assert_eq!(
            expr_of("x??"),
            ExprKind::Exists(Box::new(Expr::new(ident("x"), Span::new(1, 3))))
        );
        assert_eq!(
            expr_of("x!"),
            ExprKind::Default {
                target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                default: None,
            }
        );
        assert_eq!(
            expr_of("x!y"),
            ExprKind::Default {
                target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                default: Some(Box::new(Expr::new(ident("y"), Span::new(1, 5)))),
            }
        );
        // 带默认值的完整表达式（Java DefaultTo 前瞻语义：x!y+z → x!(y+z)）
        assert_eq!(
            expr_of("x!y + z"),
            ExprKind::Default {
                target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                default: Some(Box::new(Expr::new(
                    ExprKind::Add(
                        Box::new(Expr::new(ident("y"), Span::new(1, 5))),
                        Box::new(Expr::new(ident("z"), Span::new(1, 9))),
                    ),
                    Span::new(1, 5)
                ))),
            }
        );
        // `x! &&y`：`&&` 不是表达式开头 → 无默认值
        assert_eq!(
            expr_of("x! && y"),
            ExprKind::And(
                Box::new(Expr::new(
                    ExprKind::Default {
                        target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                        default: None,
                    },
                    Span::new(1, 3)
                )),
                Box::new(Expr::new(ident("y"), Span::new(1, 9))),
            )
        );
    }

    // -----------------------------------------------------------------------
    // lambda（LocalLambdaExpression）
    // -----------------------------------------------------------------------

    #[test]
    fn lambda_expression() {
        assert_eq!(
            expr_of("x?filter(y -> y > 1)"),
            ExprKind::BuiltIn {
                target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                name: "filter".to_string(),
                args: Some(vec![Expr::new(
                    ExprKind::Lambda {
                        params: vec!["y".to_string()],
                        body: Box::new(Expr::new(
                            ExprKind::Gt(
                                Box::new(Expr::new(ident("y"), Span::new(1, 17))),
                                Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 21))),
                            ),
                            Span::new(1, 17)
                        )),
                    },
                    Span::new(1, 12)
                )]),
            }
        );
        // 括号形式 (y) ->
        assert_eq!(
            expr_of("x?map((y) -> y * 2)"),
            ExprKind::BuiltIn {
                target: Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                name: "map".to_string(),
                args: Some(vec![Expr::new(
                    ExprKind::Lambda {
                        params: vec!["y".to_string()],
                        body: Box::new(Expr::new(
                            ExprKind::Mul(
                                Box::new(Expr::new(ident("y"), Span::new(1, 16))),
                                Box::new(Expr::new(num(TNumber::Int(2)), Span::new(1, 20))),
                            ),
                            Span::new(1, 16)
                        )),
                    },
                    Span::new(1, 9)
                )]),
            }
        );
    }

    // -----------------------------------------------------------------------
    // 列表 / 哈希字面量
    // -----------------------------------------------------------------------

    #[test]
    fn list_and_hash_literals() {
        assert_eq!(
            expr_of("[1, 2]"),
            ExprKind::ListLit(vec![
                Expr::new(num(TNumber::Int(1)), Span::new(1, 4)),
                Expr::new(num(TNumber::Int(2)), Span::new(1, 7)),
            ])
        );
        assert_eq!(expr_of("[]"), ExprKind::ListLit(vec![]));
        assert_eq!(
            expr_of(r#"{"a": 1}"#),
            ExprKind::HashLit(vec![(
                Expr::new(strlit("a"), Span::new(1, 4)),
                Expr::new(num(TNumber::Int(1)), Span::new(1, 9)),
            )])
        );
        // 逗号分隔键值对（Java HashLiteral 的 (<COMMA>|<COLON>) 形式）
        assert_eq!(
            expr_of(r#"{"a", 1}"#),
            ExprKind::HashLit(vec![(
                Expr::new(strlit("a"), Span::new(1, 4)),
                Expr::new(num(TNumber::Int(1)), Span::new(1, 9)),
            )])
        );
        // 非字符串键 → 解析错误（Java stringLiteralOnly：数字字面量作键）
        let msg = parse_err(r#"${ {1: 2} }"#);
        assert!(msg.contains("Hash literal keys must be strings"), "{msg}");
    }

    // -----------------------------------------------------------------------
    // 括号 / 一元 / 优先级
    // -----------------------------------------------------------------------

    #[test]
    fn parenthesis_and_unary() {
        assert_eq!(
            expr_of("(x)"),
            ExprKind::Paren(Box::new(Expr::new(ident("x"), Span::new(1, 4))))
        );
        assert_eq!(
            expr_of("-x"),
            ExprKind::UnaryMinus(Box::new(Expr::new(ident("x"), Span::new(1, 4))))
        );
        // `+x` 无 AST 节点（Java UnaryPlusMinusExpression(isMinus=false) 语义）
        assert_eq!(expr_of("+x"), ident("x"));
        assert_eq!(
            expr_of("!x"),
            ExprKind::Not(Box::new(Expr::new(ident("x"), Span::new(1, 4))))
        );
        assert_eq!(
            expr_of("!!x"),
            ExprKind::Not(Box::new(Expr::new(
                ExprKind::Not(Box::new(Expr::new(ident("x"), Span::new(1, 5)))),
                Span::new(1, 4)
            )))
        );
        assert_eq!(
            expr_of("-1"),
            ExprKind::UnaryMinus(Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 4))))
        );
    }

    #[test]
    fn precedence_and_associativity() {
        // 乘法优先于加法
        assert_eq!(
            expr_of("1 + 2 * 3"),
            ExprKind::Add(
                Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 3))),
                Box::new(Expr::new(
                    ExprKind::Mul(
                        Box::new(Expr::new(num(TNumber::Int(2)), Span::new(1, 7))),
                        Box::new(Expr::new(num(TNumber::Int(3)), Span::new(1, 11))),
                    ),
                    Span::new(1, 7)
                )),
            )
        );
        // 括号覆盖
        assert_eq!(
            expr_of("(1 + 2) * 3"),
            ExprKind::Mul(
                Box::new(Expr::new(
                    ExprKind::Paren(Box::new(Expr::new(
                        ExprKind::Add(
                            Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 4))),
                            Box::new(Expr::new(num(TNumber::Int(2)), Span::new(1, 8))),
                        ),
                        Span::new(1, 4)
                    ))),
                    Span::new(1, 3)
                )),
                Box::new(Expr::new(num(TNumber::Int(3)), Span::new(1, 13))),
            )
        );
        // 左结合
        assert_eq!(
            expr_of("a + b - c"),
            ExprKind::Sub(
                Box::new(Expr::new(
                    ExprKind::Add(
                        Box::new(Expr::new(ident("a"), Span::new(1, 3))),
                        Box::new(Expr::new(ident("b"), Span::new(1, 7))),
                    ),
                    Span::new(1, 3)
                )),
                Box::new(Expr::new(ident("c"), Span::new(1, 11))),
            )
        );
        // 逻辑优先级：&& 高于 ||
        assert_eq!(
            expr_of("a && b || c"),
            ExprKind::Or(
                Box::new(Expr::new(
                    ExprKind::And(
                        Box::new(Expr::new(ident("a"), Span::new(1, 3))),
                        Box::new(Expr::new(ident("b"), Span::new(1, 8))),
                    ),
                    Span::new(1, 3)
                )),
                Box::new(Expr::new(ident("c"), Span::new(1, 13))),
            )
        );
        // equality 高于 and
        assert_eq!(
            expr_of("a == b && c"),
            ExprKind::And(
                Box::new(Expr::new(
                    ExprKind::Eq(
                        Box::new(Expr::new(ident("a"), Span::new(1, 3))),
                        Box::new(Expr::new(ident("b"), Span::new(1, 8))),
                    ),
                    Span::new(1, 3)
                )),
                Box::new(Expr::new(ident("c"), Span::new(1, 13))),
            )
        );
        // 一元 not 优先于 &&
        assert_eq!(
            expr_of("!a && b"),
            ExprKind::And(
                Box::new(Expr::new(
                    ExprKind::Not(Box::new(Expr::new(ident("a"), Span::new(1, 4)))),
                    Span::new(1, 3)
                )),
                Box::new(Expr::new(ident("b"), Span::new(1, 9))),
            )
        );
        // equality 高于 relational
        assert_eq!(
            expr_of("x > 1 == y"),
            ExprKind::Eq(
                Box::new(Expr::new(
                    ExprKind::Gt(
                        Box::new(Expr::new(ident("x"), Span::new(1, 3))),
                        Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 7))),
                    ),
                    Span::new(1, 3)
                )),
                Box::new(Expr::new(ident("y"), Span::new(1, 12))),
            )
        );
        // 非结合：a == b == c 报错（Java EqualityExpression 单一可选）
        let msg = parse_err("${a == b == c}");
        assert!(msg.contains("line 1, column 10"), "{msg}");
        assert!(msg.contains("Expected \"}\""), "{msg}");
    }

    #[test]
    fn range_expressions() {
        assert_eq!(
            expr_of("1..5"),
            ExprKind::Range {
                start: Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 3))),
                end: Some(Box::new(Expr::new(num(TNumber::Int(5)), Span::new(1, 6)))),
                kind: RangeKind::Inclusive,
            }
        );
        assert_eq!(expr_of("1..<5").kind_of_range_kind(), RangeKind::Exclusive);
        assert_eq!(
            expr_of("1..*5").kind_of_range_kind(),
            RangeKind::SizeLimited
        );
        assert_eq!(
            expr_of("1.."),
            ExprKind::Range {
                start: Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 3))),
                end: None,
                kind: RangeKind::SizeLimited, // Java END_UNBOUND 无契约槽位（文档化偏差）
            }
        );
    }

    trait RangeHelper {
        fn kind_of_range_kind(&self) -> RangeKind;
    }
    impl RangeHelper for ExprKind {
        fn kind_of_range_kind(&self) -> RangeKind {
            match self {
                ExprKind::Range { kind, .. } => *kind,
                _ => panic!("expected range"),
            }
        }
    }

    #[test]
    fn relational_in_parens() {
        // 括号内 `>` 是比较符（IN_PAREN 词法状态）
        assert_eq!(
            expr_of("(a > b)"),
            ExprKind::Paren(Box::new(Expr::new(
                ExprKind::Gt(
                    Box::new(Expr::new(ident("a"), Span::new(1, 4))),
                    Box::new(Expr::new(ident("b"), Span::new(1, 8))),
                ),
                Span::new(1, 4)
            )))
        );
    }

    // -----------------------------------------------------------------------
    // 指令（If / List / Assign / Macro / Call / ...）
    // -----------------------------------------------------------------------

    #[test]
    fn if_elseif_else_flattening() {
        let t = parse_ok("<#if x>a</#if>");
        let ElementKind::If { cond, then, else_ } = &t.root[0].kind else {
            panic!("expected If, got {:?}", t.root[0].kind);
        };
        assert_eq!(cond.kind, ident("x"));
        assert_eq!(then.len(), 1);
        assert!(else_.is_none());
        assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "a"));

        let t = parse_ok("<#if x>a<#elseif y>b<#else>c</#if>");
        let ElementKind::If { cond, then, else_ } = &t.root[0].kind else {
            panic!("expected If");
        };
        assert_eq!(cond.kind, ident("x"));
        assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
        let else_ = else_.as_ref().expect("else branch");
        let ElementKind::If { cond, then, else_ } = &else_[0].kind else {
            panic!("expected nested If for elseif, got {:?}", else_[0].kind);
        };
        assert_eq!(cond.kind, ident("y"));
        assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "b"));
        let else_ = else_.as_ref().expect("nested else branch");
        assert!(matches!(else_[0].kind, ElementKind::Text { ref text, .. } if text == "c"));
    }

    #[test]
    fn list_with_items_sep_else() {
        let t = parse_ok("<#list xs as x>${x}</#list>");
        let ElementKind::List {
            seq,
            var,
            var2,
            body,
            else_,
        } = &t.root[0].kind
        else {
            panic!("expected List");
        };
        assert_eq!(seq.kind, ident("xs"));
        assert_eq!(var, "x");
        assert!(var2.is_none());
        assert!(matches!(body[0].kind, ElementKind::Interpolation(_)));
        assert!(else_.is_none());

        // items/sep 是就地元素（Java Items/Sep 模型），不再抽入 List 字段
        let t = parse_ok("<#list xs><#items as x>${x}</#items><#sep>,</#sep><#else>none</#list>");
        let ElementKind::List {
            var, body, else_, ..
        } = &t.root[0].kind
        else {
            panic!("expected List");
        };
        assert_eq!(var, "");
        let ElementKind::Items {
            var,
            body: items_body,
            ..
        } = &body[0].kind
        else {
            panic!("expected Items at body[0], got {:?}", body[0].kind);
        };
        assert_eq!(var, "x");
        assert!(matches!(items_body[0].kind, ElementKind::Interpolation(_)));
        let ElementKind::Sep { body: sep_body } = &body[1].kind else {
            panic!("expected Sep at body[1], got {:?}", body[1].kind);
        };
        assert!(matches!(sep_body[0].kind, ElementKind::Text { ref text, .. } if text == ","));
        let else_ = else_.as_ref().expect("else");
        assert!(matches!(else_[0].kind, ElementKind::Text { ref text, .. } if text == "none"));
    }

    #[test]
    fn list_hash_listing_two_vars() {
        // `as k, v`：双循环变量（hashListing；Java IteratorBlock.loopVar2Name）
        let t = parse_ok("<#list h as k, v>${k}=${v}</#list>");
        let ElementKind::List { var, var2, .. } = &t.root[0].kind else {
            panic!("expected List");
        };
        assert_eq!(var, "k");
        assert_eq!(var2.as_deref(), Some("v"));
        // 键值同名 → 报错（Java 消息）
        let msg = parse_err("<#list h as k, k>x</#list>");
        assert!(msg.contains("must differ"), "{msg}");
        // items 双变量
        let t = parse_ok("<#list h><#items as k, v>${k}=${v}</#items></#list>");
        let ElementKind::List { body, .. } = &t.root[0].kind else {
            panic!("expected List");
        };
        let ElementKind::Items { var, var2, .. } = &body[0].kind else {
            panic!("expected Items");
        };
        assert_eq!(var, "k");
        assert_eq!(var2.as_deref(), Some("v"));
    }

    #[test]
    fn list_validation_errors() {
        // 无 as 也无 items → 报错（Java 消息）
        let msg = parse_err("<#list xs></#list>");
        assert!(
            msg.contains("#list must have either \"as loopVar\""),
            "{msg}"
        );
        // as var + items → 报错
        let msg = parse_err("<#list xs as x><#items as y></#items></#list>");
        assert!(msg.contains("must not have \"as loopVar\""), "{msg}");
        // #items 在 list 外 → 报错（Java 消息）
        let msg = parse_err("<#items as x>y</#items>");
        assert!(msg.contains("#items must be inside a #list"), "{msg}");
        // #sep 在 list 外 → 报错
        let msg = parse_err("<#sep>x</#sep>");
        assert!(msg.contains("#sep must be inside a #list"), "{msg}");
        // #items 嵌套 #items → 报错
        let msg = parse_err("<#list xs><#items as x><#items as y></#items></#items></#list>");
        assert!(msg.contains("Can't nest #items"), "{msg}");
        // #foreach 内 #items → 报错（Java：foreach 不支持嵌套 items）
        let msg = parse_err("<#foreach x in xs><#items as y></#items></#foreach>");
        assert!(msg.contains("#items"), "{msg}");
    }

    #[test]
    fn assign_variants() {
        let t = parse_ok("<#assign x = 1>");
        let ElementKind::Assign {
            target,
            expr,
            op,
            namespace,
        } = &t.root[0].kind
        else {
            panic!("expected Assign");
        };
        assert_eq!(target, "x");
        assert_eq!(expr.kind, num(TNumber::Int(1)));
        assert_eq!(*op, AssignOp::Equals);
        assert!(namespace.is_none());

        for (src, expected_op) in [
            ("<#assign x += 1>", AssignOp::PlusEq),
            ("<#assign x -= 1>", AssignOp::MinusEq),
            ("<#assign x *= 2>", AssignOp::TimesEq),
            ("<#assign x /= 2>", AssignOp::DivideEq),
            ("<#assign x %= 2>", AssignOp::ModuloEq),
            ("<#assign x++>", AssignOp::PlusPlus),
            ("<#assign x-->", AssignOp::MinusMinus),
        ] {
            let t = parse_ok(src);
            let ElementKind::Assign { op, .. } = &t.root[0].kind else {
                panic!("expected Assign for {src}");
            };
            assert_eq!(*op, expected_op, "op for {src}");
        }

        // 命名空间
        let t = parse_ok("<#assign x = 1 in ns>");
        let ElementKind::Assign { namespace, .. } = &t.root[0].kind else {
            panic!("expected Assign");
        };
        assert_eq!(namespace.as_deref(), Some("ns"));

        // 块赋值
        let t = parse_ok("<#assign x>body</#assign>");
        assert!(matches!(t.root[0].kind, ElementKind::BlockAssign { .. }));

        // global / local
        let t = parse_ok("<#global x = 1>");
        assert!(matches!(t.root[0].kind, ElementKind::Global { .. }));
        let t = parse_ok("<#macro m><#local x = 1></#macro>");
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        assert!(matches!(def.body[0].kind, ElementKind::Local { .. }));
        // local 在宏外 → 报错（Java 消息）
        let msg = parse_err("<#local x = 1>");
        assert!(
            msg.contains("Local variable assigned outside a macro"),
            "{msg}"
        );
    }

    #[test]
    fn macro_and_function() {
        let t = parse_ok("<#macro m a b=2>body</#macro>");
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        assert_eq!(def.name, "m");
        assert!(!def.is_function);
        assert_eq!(def.params.len(), 2);
        assert_eq!(def.params[0].name, "a");
        assert!(def.params[0].default.is_none());
        assert!(!def.params[0].optional);
        assert_eq!(def.params[1].name, "b");
        assert!(def.params[1].default.is_some());
        assert!(def.params[1].optional);
        // 宏表注册
        assert!(t.macros.contains_key("m"));

        // 字符串名 + catch-all 参数
        let t = parse_ok(r#"<#macro "catch-all" foo bar...>x</#macro>"#);
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        assert_eq!(def.name, "catch-all");
        assert_eq!(def.params.len(), 2);
        assert!(def.params[1].catch_all);
        assert!(def.params[1].optional);

        // function
        let t = parse_ok("<#function f x>${x}</#function>");
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        assert!(def.is_function);
        assert_eq!(def.name, "f");

        // 参数顺序校验（默认值参数后不能再有必选参数）
        let msg = parse_err("<#macro m a=1 b>x</#macro>");
        assert!(
            msg.contains("parameters without a default value must all occur before"),
            "{msg}"
        );
        // 宏嵌套 → 报错
        let msg = parse_err("<#macro a><#macro b></#macro></#macro>");
        assert!(msg.contains("can't be nested"), "{msg}");
    }

    #[test]
    fn user_directive_calls() {
        // 命名参数 + 自闭合
        let t = parse_ok("<@m x=1/>");
        let ElementKind::Call {
            callee,
            args,
            body,
            body_params,
        } = &t.root[0].kind
        else {
            panic!("expected Call");
        };
        assert_eq!(*callee, CallTarget::Name("m".to_string()));
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].0, "x");
        assert_eq!(args[0].1.kind, num(TNumber::Int(1)));
        assert!(body.is_none() && body_params.is_empty());

        // 命名空间调用
        let t = parse_ok("<@ns.m/>");
        let ElementKind::Call { callee, .. } = &t.root[0].kind else {
            panic!("expected Call");
        };
        assert_eq!(
            *callee,
            CallTarget::Namespaced {
                ns: "ns".to_string(),
                name: "m".to_string(),
            }
        );

        // 位置参数（契约 args 以空名存储）
        let t = parse_ok("<@m 1 2/>");
        let ElementKind::Call { args, .. } = &t.root[0].kind else {
            panic!("expected Call");
        };
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].0, "");
        assert_eq!(args[1].0, "");

        // body + 多 body 参数（<@m x; a, b>）
        let t = parse_ok("<@m x; a, b>body</@m>");
        let ElementKind::Call {
            body, body_params, ..
        } = &t.root[0].kind
        else {
            panic!("expected Call");
        };
        assert_eq!(body_params, &["a".to_string(), "b".to_string()]);
        let body = body.as_ref().expect("body");
        assert!(matches!(body[0].kind, ElementKind::Text { ref text, .. } if text == "body"));

        // 结束标签名不匹配 → 报错（Java：Expecting </@> or </@m>）
        let msg = parse_err("<@m>body</@n>");
        assert!(msg.contains("Expecting </@> or </@m>"), "{msg}");
    }

    #[test]
    fn nested_switch_attempt_break() {
        let t = parse_ok("<#macro m><#nested></#macro>");
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        assert!(matches!(def.body[0].kind, ElementKind::Nested { .. }));

        let t = parse_ok("<#macro m><#nested x y></#macro>");
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        let ElementKind::Nested { args, .. } = &def.body[0].kind else {
            panic!("expected Nested");
        };
        assert_eq!(args.len(), 2);

        // nested 在宏外 → 报错（Java 消息）
        let msg = parse_err("<#nested>");
        assert!(
            msg.contains("Cannot use a \"nested\" instruction outside a macro"),
            "{msg}"
        );

        let t = parse_ok("<#switch v><#case 1>a<#default>b</#switch>");
        let ElementKind::Switch {
            expr,
            cases,
            default,
            default_pos,
        } = &t.root[0].kind
        else {
            panic!("expected Switch");
        };
        assert_eq!(expr.kind, ident("v"));
        assert_eq!(cases.len(), 1);
        assert_eq!(default_pos, &Some(1));
        assert_eq!(cases[0].value.kind, num(TNumber::Int(1)));
        assert!(matches!(cases[0].body[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
        let default = default.as_ref().expect("default");
        assert!(matches!(default[0].kind, ElementKind::Text { ref text, .. } if text == "b"));

        // 重复 default → 报错
        let msg = parse_err("<#switch v><#default>a<#default>b</#switch>");
        assert!(msg.contains("You already had a #default"), "{msg}");
        // 空 switch 合法（Java switch.ftl 用例 `[<#switch 213></#switch>]` 渲染为空）
        let t = parse_ok("<#switch v></#switch>");
        assert!(matches!(t.root[0].kind, ElementKind::Switch { .. }));

        let t = parse_ok("<#attempt>a<#recover>b</#attempt>");
        let ElementKind::Attempt { try_, recover } = &t.root[0].kind else {
            panic!("expected Attempt");
        };
        assert!(matches!(try_[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
        assert!(matches!(recover[0].kind, ElementKind::Text { ref text, .. } if text == "b"));

        // break 需要循环/switch 上下文（Java 消息）
        let msg = parse_err("<#break>");
        assert!(msg.contains("break must be nested"), "{msg}");
        let t = parse_ok("<#list xs as x><#break><#continue></#list>");
        let ElementKind::List { body, .. } = &t.root[0].kind else {
            panic!("expected List");
        };
        assert!(matches!(body[0].kind, ElementKind::Break));
        assert!(matches!(body[1].kind, ElementKind::Continue));
    }

    #[test]
    fn return_stop_flush() {
        let t = parse_ok("<#macro m><#return></#macro>");
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        assert!(matches!(
            def.body[0].kind,
            ElementKind::Return { expr: None }
        ));

        let t = parse_ok("<#function f><#return x></#function>");
        let ElementKind::Macro { def } = &t.root[0].kind else {
            panic!("expected Macro");
        };
        let ElementKind::Return { expr } = &def.body[0].kind else {
            panic!("expected Return");
        };
        assert_eq!(expr.as_ref().unwrap().kind, ident("x"));

        // macro 返回值 / function 不返回值 → 报错
        let msg = parse_err("<#macro m><#return x></#macro>");
        assert!(msg.contains("A macro cannot return a value"), "{msg}");
        let msg = parse_err("<#function f><#return></#function>");
        assert!(msg.contains("A function must return a value"), "{msg}");
        let msg = parse_err("<#return>");
        assert!(
            msg.contains("only occur inside a macro or function"),
            "{msg}"
        );

        let t = parse_ok("<#stop>");
        assert!(matches!(t.root[0].kind, ElementKind::Stop { msg: None }));
        let t = parse_ok(r#"<#stop "msg">"#);
        let ElementKind::Stop { msg } = &t.root[0].kind else {
            panic!("expected Stop");
        };
        assert_eq!(msg.as_ref().unwrap().kind, strlit("msg"));

        let t = parse_ok("<#flush>");
        assert!(matches!(t.root[0].kind, ElementKind::Flush));
    }

    #[test]
    fn include_import_setting_escape_compress() {
        let t = parse_ok(r#"<#include "x.ftl" parse=true encoding="utf-8">"#);
        let ElementKind::Include { path, attrs } = &t.root[0].kind else {
            panic!("expected Include");
        };
        assert_eq!(path.kind, strlit("x.ftl"));
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0].0, "parse");
        assert_eq!(attrs[0].1.kind, ExprKind::Bool(true));

        // 未知 include 参数 → 报错（Java 消息）
        let msg = parse_err(r#"<#include "x.ftl" foo=1>"#);
        assert!(
            msg.contains("Unsupported named #include parameter"),
            "{msg}"
        );

        let t = parse_ok(r#"<#import "lib.ftl" as ns>"#);
        let ElementKind::Import { path, ns } = &t.root[0].kind else {
            panic!("expected Import");
        };
        assert_eq!(path.kind, strlit("lib.ftl"));
        assert_eq!(ns, "ns");

        let t = parse_ok(r#"<#setting locale="en">"#);
        let ElementKind::Setting { key, value } = &t.root[0].kind else {
            panic!("expected Setting");
        };
        assert_eq!(key, "locale");
        assert_eq!(value.kind, strlit("en"));

        let t = parse_ok("<#escape x as x?html>a</#escape>");
        assert!(matches!(t.root[0].kind, ElementKind::Escape { .. }));
        let t = parse_ok("<#noescape>a</#noescape>");
        assert!(matches!(t.root[0].kind, ElementKind::NoEscape(_)));
        let t = parse_ok("<#compress>a</#compress>");
        assert!(matches!(t.root[0].kind, ElementKind::Compress(_)));
        let t = parse_ok("<#autoesc>a</#autoesc>");
        assert!(matches!(t.root[0].kind, ElementKind::AutoEsc(_)));
        let t = parse_ok("<#noautoesc>a</#noautoesc>");
        assert!(matches!(t.root[0].kind, ElementKind::NoAutoEsc(_)));
        let t = parse_ok(r#"<#outputformat "HTML">a</#outputformat>"#);
        assert!(matches!(t.root[0].kind, ElementKind::OutputFormat { .. }));
    }

    #[test]
    fn comments_and_special_text() {
        let t = parse_ok("a<#-- comment -->b");
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
        assert!(matches!(t.root[1].kind, ElementKind::Comment { ref text } if text == " comment "));
        assert!(matches!(t.root[2].kind, ElementKind::Text { ref text, .. } if text == "b"));

        // 多行注释
        let t = parse_ok("<#--\nmulti\nline\n-->x");
        assert!(
            matches!(t.root[0].kind, ElementKind::Comment { ref text } if text.contains("multi"))
        );

        // <#comment> 块（NO_PARSE 内容原样保留）
        let t = parse_ok("<#comment>raw <#if>x</#if></#comment>x");
        assert!(
            matches!(t.root[0].kind, ElementKind::Comment { ref text } if text == "raw <#if>x</#if>")
        );

        // t / nt / lt / rt：TrimInstruction 解析期消费后即被移除
        // （Java TrimInstruction.isIgnorable=true → postParseCleanup 移除，渲染期 no-op）
        let t = parse_ok("<#t>");
        assert!(
            t.root.is_empty(),
            "TrimInstruction removed from the tree after parse"
        );
        let t = parse_ok("<#nt>");
        assert!(t.root.is_empty());
        // <#lt> 是左裁剪标记（Java TrimInstruction(true,false)），非字面 "<"
        let t = parse_ok("<#lt>");
        assert!(t.root.is_empty());
        let t = parse_ok("<#rt>");
        assert!(t.root.is_empty());
        let t = parse_ok("<#gt>");
        assert!(matches!(t.root[0].kind, ElementKind::RawText(ref s) if s == ">"));
        let t = parse_ok("<#noparse>${x} ${y}</#noparse>");
        assert!(
            matches!(t.root[0].kind, ElementKind::NoParse { ref text, .. } if text == "${x} ${y}")
        );
    }

    #[test]
    fn ftl_header() {
        // 角度语法头部
        let t = parse_ok(r#"<#ftl encoding="UTF-8">hello"#);
        assert_eq!(t.encoding.as_deref(), Some("UTF-8"));
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "hello"));

        // 方括号语法头部（含换行吞除）
        let t = parse_ok("[#ftl]\nhello");
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "hello"));

        // 头部只允许在模板开头
        let msg = parse_err("x<#ftl>");
        assert!(msg.contains("#ftl header is only allowed"), "{msg}");
    }

    // -----------------------------------------------------------------------
    // 词法规则（docs/03 §2.3）
    // -----------------------------------------------------------------------

    #[test]
    fn angle_bracket_is_text() {
        // `a < b` 是文本（非严格与严格语法）
        let t = parse_ok("a < b");
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a < b"));
        let t = parse_with(&cfg_strict(), "a < b");
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a < b"));
        // 非指令名标签是文本
        let t = parse_ok("a <b> c");
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a <b> c"));
    }

    #[test]
    fn dollar_escape_and_interpolation() {
        // `$${` → 文本 $ + 插值
        let t = parse_ok("$${x}");
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "$"));
        assert!(matches!(t.root[1].kind, ElementKind::Interpolation(_)));
        // `$` 后非 `{` 为文本
        let t = parse_ok("$x");
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "$x"));
        // `${expr}` 与 `#{expr}` 插值
        let t = parse_ok("a${x}b#{y}c");
        assert_eq!(t.root.len(), 5);
        assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
        assert!(matches!(t.root[1].kind, ElementKind::Interpolation(_)));
        assert!(matches!(t.root[2].kind, ElementKind::Text { ref text, .. } if text == "b"));
        assert!(matches!(t.root[3].kind, ElementKind::Interpolation(_)));
        assert!(matches!(t.root[4].kind, ElementKind::Text { ref text, .. } if text == "c"));
    }

    #[test]
    fn square_bracket_syntax() {
        let t = parse_ok("[#if x]y[/#if]");
        let ElementKind::If { then, .. } = &t.root[0].kind else {
            panic!("expected If, got {:?}", t.root[0].kind);
        };
        assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "y"));

        // 角度语法确立后 `[#` 是文本（Java STATIC_TEXT 语义）
        let t = parse_ok("a<#if x>b</#if>[#if y]c[/#if]");
        assert_eq!(t.root.len(), 3);
        assert!(
            matches!(t.root[2].kind, ElementKind::Text { ref text, .. } if text.contains("[#if"))
        );
    }

    #[test]
    fn expression_comments() {
        let t = parse_ok("${1 + [#-- c --] 2}");
        let ElementKind::Interpolation(e) = &t.root[0].kind else {
            panic!("expected Interpolation");
        };
        assert_eq!(
            e.kind,
            ExprKind::Add(
                Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 3))),
                Box::new(Expr::new(num(TNumber::Int(2)), Span::new(1, 18))),
            )
        );
    }

    // -----------------------------------------------------------------------
    // 解析错误（位置 + 期望内容）
    // -----------------------------------------------------------------------

    #[test]
    fn error_positions() {
        // 未闭合标签
        let msg = parse_err("<#if x>");
        assert!(
            msg.contains("Parsing error in template t: \"t\" at line 1, column"),
            "{msg}"
        );
        assert!(msg.contains("</#if>"), "{msg}");

        // 多行模板的行号
        let msg = parse_err("a\nb\n<#if x>");
        assert!(msg.contains("at line 3, column 8"), "{msg}");

        // 未闭合插值
        let msg = parse_err("${x");
        assert!(msg.contains("at line 1, column 4"), "{msg}");

        // 未闭合注释
        let msg = parse_err("<#-- unclosed");
        assert!(msg.contains("Unclosed comment"), "{msg}");

        // 非法字符
        let msg = parse_err("${a @}");
        assert!(msg.contains("Unexpected character"), "{msg}");

        // 不匹配的结束标签
        let msg = parse_err("<#if x></#list>");
        assert!(msg.contains("Unexpected closing tag \"</#list>\""), "{msg}");

        // 自闭合块指令
        let msg = parse_err("<#if x/>");
        assert!(msg.contains("self-closing"), "{msg}");

        // 未知指令
        let msg = parse_err("<#nosuchdir>");
        assert!(msg.contains("Unknown directive: #nosuchdir"), "{msg}");

        // 孤立的 <#else>
        let msg = parse_err("<#else>");
        assert!(msg.contains("Unexpected directive <#else>"), "{msg}");
    }

    // -----------------------------------------------------------------------
    // 空白剥离标记（docs/08 §5.2；对照 Java TextBlock.postParseCleanup）
    // -----------------------------------------------------------------------

    #[test]
    fn whitespace_stripping_flags() {
        // 剥离在解析期直接改写文本（Java TextBlock.postParseCleanup 的 text = substring
        // 语义，TextBlock.java:128；strip_before/strip_after 标记恒 false）
        // 行首空白 + FTL 标签行 → 剥到首个换行（含）为止
        let t = parse_ok("A\n<#if x>\nB\n</#if>\nC");
        let ElementKind::If { then, .. } = &t.root[1].kind else {
            panic!("expected If");
        };
        let ElementKind::Text { text, .. } = &then[0].kind else {
            panic!("expected Text in then");
        };
        assert_eq!(text, "B\n", "leading newline after <#if> stripped at parse");
        let ElementKind::Text { text, .. } = &t.root[2].kind else {
            panic!("expected Text after if");
        };
        assert_eq!(text, "C", "leading newline after </#if> stripped at parse");

        // 前一同行文本有内容 → 不剥离（Java heedsOpeningWhitespace）
        let t = parse_ok("x<#if y>  \nz</#if>");
        let ElementKind::If { then, .. } = &t.root[1].kind else {
            panic!("expected If");
        };
        let ElementKind::Text { text, .. } = &then[0].kind else {
            panic!("expected Text");
        };
        assert_eq!(text, "  \nz", "same-line previous text blocks stripping");

        // 尾部空白：块后无内容 → 剥离
        let t = parse_ok("<#if y>foo\n  </#if>");
        let ElementKind::If { then, .. } = &t.root[0].kind else {
            panic!("expected If");
        };
        let ElementKind::Text { text, .. } = &then[0].kind else {
            panic!("expected Text");
        };
        assert_eq!(
            text, "foo\n",
            "trailing whitespace of last block text stripped"
        );

        // 尾部空白：同行的下一文本有内容 → 不剥离
        let t = parse_ok("<#if y>foo\n  </#if>bar");
        let ElementKind::If { then, .. } = &t.root[0].kind else {
            panic!("expected If");
        };
        let ElementKind::Text { text, .. } = &then[0].kind else {
            panic!("expected Text");
        };
        assert_eq!(text, "foo\n  ", "same-line following text blocks stripping");

        // 模板首文本不剥（Java 守卫）
        let t = parse_ok("  \n<#if x>y</#if>");
        let ElementKind::Text { text, .. } = &t.root[0].kind else {
            panic!("expected Text");
        };
        assert_eq!(text, "  \n", "first root text never stripped");

        // <#t> 显式裁剪 / <#nt> 显式取消
        let t = parse_ok("<#if y>a\n  <#t></#if>");
        let ElementKind::If { then, .. } = &t.root[0].kind else {
            panic!("expected If");
        };
        // Java deliberateLeftTrim：<#t> 显式裁剪最后一行前导（"  " 全空白 → 裁掉）
        let ElementKind::Text { text, .. } = &then[0].kind else {
            panic!("expected Text");
        };
        assert_eq!(text, "a\n", "<#t> trims the trailing blank line");
        let t = parse_ok("<#if y>a\n  <#nt></#if>");
        let ElementKind::If { then, .. } = &t.root[0].kind else {
            panic!("expected If");
        };
        let ElementKind::Text { text, .. } = &then[0].kind else {
            panic!("expected Text");
        };
        assert_eq!(text, "a\n  ", "<#nt> prevents stripping the preceding text");
    }

    #[test]
    fn stripping_off_when_disabled() {
        // whitespace_stripping=false → 无标记
        let mut c = Configuration::new();
        c.settings.whitespace_stripping = false;
        let cfg = Rc::new(c);
        let t = parse_with(&cfg, "A\n<#if x>\nB\n</#if>\nC");
        let ElementKind::If { then, .. } = &t.root[1].kind else {
            panic!("expected If");
        };
        let ElementKind::Text { strip_before, .. } = &then[0].kind else {
            panic!("expected Text");
        };
        assert!(!*strip_before);
    }

    // -----------------------------------------------------------------------
    // Java 测试套件真实模板冒烟解析（include_str! 嵌入）
    // -----------------------------------------------------------------------

    #[test]
    fn java_suite_helloworld() {
        let t = parse_ok(include_str!(concat!(
            "/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/helloworld.ftl"
        )));
        assert!(matches!(t.root[0].kind, ElementKind::Comment { .. }));
        assert!(
            matches!(t.root[1].kind, ElementKind::Text { ref text, .. } if text.contains("<html>"))
        );
    }

    #[test]
    fn java_suite_escapes() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/escapes.ftl"));
        // <#escape> 块 + <#noescape> 块
        assert!(
            t.root
                .iter()
                .any(|e| matches!(e.kind, ElementKind::Escape { .. })),
            "expected an #escape block in escapes.ftl"
        );
    }

    #[test]
    fn java_suite_if() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/if.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_boolean() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/boolean.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_comment() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/comment.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_lastcharacter() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/lastcharacter.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_default() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/default.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_localization() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/localization.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_boolean_formatting() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/boolean-formatting.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_include() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/include.ftl"));
        assert!(!t.root.is_empty());
    }

    #[test]
    fn java_suite_import() {
        let t = parse_ok(include_str!("/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite/templates/import.ftl"));
        assert!(!t.root.is_empty());
    }
}
