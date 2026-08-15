//! 指令分派与基本指令实现（if/list/foreach/items/sep/call）。

use super::grammar_element_helpers::newline_count;
use super::grammar_helpers::{AssignScope, BlockStop, IterCtx};
use super::{Parser, SETTING_NAMES};
use crate::core::{CallTarget, Element, ElementKind, Expr, ExprKind, OutputFormatKind};
use crate::error::Result;
use crate::parser::lexer::Tok;
use crate::span::Span;

impl<'a> Parser<'a> {
    pub(crate) fn dispatch_directive(
        &mut self,
        name: &str,
        line: u32,
        col: u32,
    ) -> Result<Element> {
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
                    // Java FTL.jj :3080：`start.image + " must be nested inside ..."`
                    // （start.image = `<#break>` 标签原文，jar 实测 break_outside 基线）
                    return Err(self.err(
                        line,
                        col,
                        "<#break> must be nested inside a directive that supports it:  #list with \"as\", #items, #switch (or the deprecated #foreach)",
                    ));
                }
                Element::new(ElementKind::Break, span)
            }
            "continue" => {
                self.expect_tag_end_raw()?;
                if self.continue_nesting == 0 {
                    // Java FTL.jj :3101（start.image = `<#continue>`）
                    return Err(self.err(
                        line,
                        col,
                        "<#continue> must be nested inside a directive that supports it:  #list with \"as\", #items (or the deprecated #foreach)",
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
                // Java OutputFormatBlock（FTL.jj :4079-4128）：参数必须为解析期可
                // 求值的字符串字面量；未注册的格式名 → 逐字报
                // "Unregistered output format name, \"{name}\". The output formats
                // registered in the Configuration are: ..."（位置 = 指令标签起始，
                // jar 实测 unknown_output_format 基线 col 1）
                if let ExprKind::Str(s) = &name_expr.kind {
                    if OutputFormatKind::parse(s).is_none() {
                        return Err(self.err(
                            line,
                            col,
                            format!(
                                "Unregistered output format name, \"{s}\". The output formats registered in the Configuration are: \"CSS\", \"HTML\", \"JSON\", \"JavaScript\", \"RTF\", \"XHTML\", \"XML\", \"plainText\", \"undefined\""
                            ),
                        ));
                    }
                }
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
                // 配置级名两种命名约定同样拒绝（Configuration 的
                // SETTING_NAMES_SNAKE_CASE/CAMEL_CASE 双写收录，jar 实测）
                if matches!(
                    key.as_str(),
                    "whitespace_stripping"
                        | "whitespaceStripping"
                        | "strict_syntax"
                        | "strictSyntax"
                        | "output_format"
                        | "outputFormat"
                        | "auto_escaping"
                ) {
                    return Err(self.err(
                        kl,
                        kc,
                        "The setting name is recognized, but changing this setting from inside a template isn't supported.",
                    ));
                }
                // Java PropertySetting.SETTING_NAMES（:43-68）白名单：未知名解析期
                // 报错（jar 实测 parse_setting_unknown 基线；两种命名约定 canonical
                // 后校验）。Java 在读取 key 后立即校验（先于 `=` 检查）
                let canonical = crate::core::canonical_setting_key(&key);
                if !SETTING_NAMES.contains(&canonical) && canonical != "template_exception_handler"
                {
                    // template_exception_handler：Rust 文档化偏差（允许模板内设置，
                    // Java 属 Configurable 级）——不出现在 allowed 列表
                    return Err(self.err(
                        kl,
                        kc,
                        format!(
                            "Unknown setting name: \"{key}\". The allowed setting names are: {}",
                            SETTING_NAMES.join(", ")
                        ),
                    ));
                }
                self.expect_tok(Tok::Eq, "\"=\"")?;
                let value = self.expression()?;
                self.loose_end()?;
                // 设置名规范化：Java PropertySetting.SETTING_NAMES（:43-68）
                // 两种命名约定并存（booleanFormat/boolean_format 等 12 项），
                // 渲染期 exec_setting 按 snake_case 规范键匹配
                // （configurable.rs canonical_setting_key）
                let key = canonical.to_string();
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
                // Java VisitNode（FTL.jj）：`<#visit [node] [using target]>`
                // （无参 = 当前节点；XML 命名空间场景）
                let expr = if self.at_expr_start(false)? {
                    Some(self.expression()?)
                } else {
                    None
                };
                let using = if self.peek_tok()?.0 == Tok::Using {
                    self.next_tok()?;
                    Some(self.expression()?)
                } else {
                    None
                };
                self.loose_end()?;
                Element::new(ElementKind::Visit { expr, using }, span)
            }
            "recurse" => {
                // Java RecurseNode（FTL.jj）：`<#recurse [node] [using target]>`
                // （无参 = 当前节点）
                let expr = if self.at_expr_start(false)? {
                    Some(self.expression()?)
                } else {
                    None
                };
                let using = if self.peek_tok()?.0 == Tok::Using {
                    self.next_tok()?;
                    Some(self.expression()?)
                } else {
                    None
                };
                self.loose_end()?;
                Element::new(ElementKind::Recurse { expr, using }, span)
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
                // Java UNKNOWN_DIRECTIVE（FTL.jj :1128-1172）：消息含 tip 段；
                // 位置 = beginColumn + 1（`#` 处，jar 实测 parse_unknown_directive）
                return Err(self.unknown_directive_err(line, col, other));
            }
        };
        Ok(elem)
    }

    /// `<#if cond> ... [<#elseif>] ... [<#else>] ... </#if>`
    /// elseif 扁平化为嵌套 If 的 else 分支（契约注释）
    pub(crate) fn if_directive(&mut self) -> Result<Element> {
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

    pub(crate) fn parse_if_tail(
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
    pub(crate) fn list_directive(&mut self) -> Result<Element> {
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
            items_open: false,
        });
        if has_var {
            self.loop_nesting += 1;
            self.continue_nesting += 1;
        }
        let r = self.list_body(line, col, var, var2, seq);
        if has_var {
            self.loop_nesting -= 1;
            self.continue_nesting -= 1;
        }
        self.iter_stack.pop();
        r
    }

    pub(crate) fn list_body(
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
                // Java List()：主块（MixedContentElements）解析后 breakable/
                // continuable 嵌套已递减，再解析 `#else` 块 —— 故 `#else` 内的
                // #break/#continue 非法（Java FTL.jj 2813-2831）
                let saved_loop = self.loop_nesting;
                let saved_continue = self.continue_nesting;
                if var.is_some() || var2.is_some() {
                    self.loop_nesting -= 1;
                    self.continue_nesting -= 1;
                }
                let els_res = self.parse_block(&["list"], &[]);
                self.loop_nesting = saved_loop;
                self.continue_nesting = saved_continue;
                let (els, stop) = els_res?;
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
    pub(crate) fn foreach_directive(&mut self) -> Result<Element> {
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
            items_open: false,
        });
        self.loop_nesting += 1;
        self.continue_nesting += 1;
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
        self.continue_nesting -= 1;
        self.iter_stack.pop();
        r
    }

    /// `<#items as x[, y]> body </#items>`（就地元素 —— 对应 Java Items()，FTL.jj 2913-2971：
    /// 从最近的 #list 迭代上下文驱动 body 逐项执行；可嵌套在 list 体内的任意位置）
    pub(crate) fn items_directive(&mut self) -> Result<Element> {
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
        // Java Items()：peekIteratorBlockContext 校验（FTL.jj 2925-2942：
        // iterCtx.loopVarName != null → 按 kind 分派错误消息）
        if self.iter_stack.is_empty() {
            return Err(self.err(line, col, "#items must be inside a #list block."));
        }
        let ctx = self.iter_stack.last_mut().expect("已判非空");
        if ctx.has_loop_var || ctx.items_open {
            let msg = if ctx.is_foreach {
                "#foreach doesn't support nested #items."
            } else if ctx.is_items {
                "Can't nest #items into each other when they belong to the same #list."
            } else {
                "The parent #list of the #items must not have \"as loopVar\" parameter."
            };
            return Err(self.err(line, col, msg));
        }
        // Java Items()：iterCtx.kind = ITERATOR_BLOCK_KIND_ITEMS（:2943）——
        // 进入后不重置，供 `#list` 无 as 的结束校验（list_body 的 entered_items）
        ctx.is_items = true;
        ctx.items_open = true;
        self.loop_nesting += 1;
        self.continue_nesting += 1;
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
        self.continue_nesting -= 1;
        // Java Items()：END_ITEMS 后 iterCtx.loopVarName = null（FTL.jj 2966-2968）——
        // 同一 #list 中顺序多个 #items 合法（list3 用例 switch 的不同分支）；
        // "已进入过" 校验由 is_items（kind）承担，嵌套校验由 items_open 承担
        if let Some(ctx) = self.iter_stack.last_mut() {
            ctx.items_open = false;
        }
        r
    }

    /// `<#sep> body </#sep>`（就地元素 —— 对应 Java Sep()，FTL.jj 2974-2995：
    /// 当前迭代 hasNext 时渲染 body；`</#sep>` 可选，父块结束标签自动收尾）
    pub(crate) fn sep_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        if self.iter_stack.is_empty() {
            return Err(self.err(
                line,
                col,
                "#sep must be inside a #list (or #foreach) block.",
            ));
        }
        self.expect_tag_end_raw()?;
        let (body, stop) = self.parse_sep_block()?;
        match stop {
            // 显式 `</#sep>`（Java `end = <END_SEP>`）
            BlockStop::EndTag(n) if n == "sep" => {}
            // Java Sep()：`[LOOKAHEAD(1) end = <END_SEP>]` 可选 —— 父块结束标签 /
            // `<#else>` 等终止指令 / EOF 自动收尾，停止信息上抛给外层块
            // （外层 parse_block 循环顶部的 pending_stop 检查接手）
            BlockStop::EndTag(_) | BlockStop::Dir(_) | BlockStop::EndCall(_) | BlockStop::Eof => {
                self.pending_stop = Some(stop);
            }
        }
        Ok(Element::new(
            ElementKind::Sep { body },
            Span::new(line, col),
        ))
    }

    /// `<#call name (args)>`（legacy；对应 Java Call()，FTL.jj 3711-3751：
    /// 构造 UnifiedCall(legacySyntax=true)，无 body；括号可选）
    pub(crate) fn call_directive(&mut self) -> Result<Element> {
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
}
