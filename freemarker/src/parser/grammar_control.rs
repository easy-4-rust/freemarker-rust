//! 控制流与模板包含指令实现（switch/attempt/return/stop/include/import）。

use super::grammar_helpers::{tok_desc, BlockStop};
use super::Parser;
use crate::core::{CaseDef, Element, ElementKind, Expr};
use crate::error::Result;
use crate::parser::lexer::{TagOpen, TextStop, Tok};
use crate::span::Span;

impl<'a> Parser<'a> {
    /// `<#switch expr> <#case v>.. (<#case|#default>..)* </#switch>`
    pub(crate) fn switch_directive(&mut self) -> Result<Element> {
        let (line, col) = self.tag_pos;
        let expr = self.expression()?;
        self.expect_tag_end()?;
        // breakable 计数迁入 switch_body：Java 在首个 #on 处撤销（FTL.jj Switch()），
        // #case 模式在 END_SWITCH 撤销——两种模式的净效果不同
        self.switch_body(line, col, expr)
    }

    pub(crate) fn switch_body(&mut self, line: u32, col: u32, expr: Expr) -> Result<Element> {
        let mut cases: Vec<CaseDef> = Vec::new();
        let mut default: Option<Vec<Element>> = None;
        let mut default_pos: Option<usize> = None;
        let mut had_default = false;
        let mut had_case = false;
        let mut had_on = false;
        // Java FTL.jj Switch()：进入即递增 breakable 计数（#case 体内 #break 合法）
        self.loop_nesting += 1;
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
                            // Java getOrRenderDescription EOF 分支（END_SWITCH → "#switch"）
                            return Err(self.eof_unclosed(&["#switch"]));
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
                                            had_on,
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
                // Java 2.3.28+：#switch 内 `<#on expr>` 是 `<#case expr>` 的等价形式
                // （FTL.jj SWITCH_BODY：case/on 同为 CASE 关键字；Java 2.3.34 起
                // #case 废弃别名，SwitchTest testOn 即此语法）
                "case" | "on" => {
                    // Java FTL.jj Switch() 的互斥校验（错误消息逐字）：
                    // #case 在 #on 之后 → "already had an #on"；#on 在 #case 之后 →
                    // "already had a #case"；#on 在 #default 之后 → "#on after #default"
                    if lname == "on" {
                        if had_case {
                            return Err(self.err(
                                line,
                                col,
                                "You can't use both #on, and #case in a #switch block, and you already had a #case.",
                            ));
                        }
                        if had_default {
                            return Err(self.err(
                                line,
                                col,
                                "You can't use #on after #default in a #switch block; #default must come last.",
                            ));
                        }
                        if !had_on {
                            had_on = true;
                            // Java FTL.jj Switch()："breakableDirectiveNesting++ 在发现
                            // 'on' 调用时撤销"——#on 体内 #break 视为非法
                            self.loop_nesting -= 1;
                        }
                    } else if had_on {
                        return Err(self.err(
                            line,
                            col,
                            "You can't use both #on, and #case in a #switch block, and you already had an #on.",
                        ));
                    } else {
                        had_case = true;
                    }
                    let mut values = vec![self.expression()?];
                    // Java 2.3.28+ #on 多值：`<#on 4, 5>` 一个块匹配多个值
                    // （FTL.jj ON 语法；#case 保持单值——Java 中 #case 无逗号语法）
                    // 注意不能用 lexer.peek() 探测：peek_tok/next_tok 已把逗号扫入
                    // 缓冲 buf（lexer 位置越过逗号），须从 token 流判断
                    while lname == "on" && self.peek_tok()?.0 == Tok::Comma {
                        self.next_tok()?;
                        values.push(self.expression()?);
                    }
                    self.expect_tag_end()?;
                    let (els, stop) = self.parse_block(&["switch"], &["case", "default", "on"])?;
                    // Java On：**不**追加 break 子元素——#on 不 fall-through 由执行器
                    // （SwitchBlock.accept 的 processOnDirectives 分支）保证；追加 break
                    // 会破坏空白剥离的 next 终端链（case body 尾部空白须保留，
                    // testOnWhitespace 断言 `C1\n    ]`）
                    match stop {
                        BlockStop::Eof => {
                            return Err(self.err(line, col, "Unclosed <#switch> block."));
                        }
                        BlockStop::EndCall(_) => {
                            return Err(self.err(line, col, "Unexpected </@...> in #switch."));
                        }
                        BlockStop::EndTag(_) | BlockStop::Dir(_) => {}
                    }
                    for value in values {
                        cases.push(CaseDef {
                            value,
                            body: els.clone(),
                            is_on: lname == "on",
                        });
                    }
                    match stop {
                        BlockStop::Dir(n) => pending = Some(n),
                        _ => {
                            return self.finish_switch(
                                line,
                                col,
                                expr,
                                cases,
                                default,
                                default_pos,
                                had_on,
                            )
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
                    let (els, stop) = self.parse_block(&["switch"], &["case", "default", "on"])?;
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
                            return self.finish_switch(
                                line,
                                col,
                                expr,
                                cases,
                                default,
                                default_pos,
                                had_on,
                            )
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
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn finish_switch(
        &mut self,
        line: u32,
        col: u32,
        expr: Expr,
        cases: Vec<CaseDef>,
        default: Option<Vec<Element>>,
        default_pos: Option<usize>,
        had_on: bool,
    ) -> Result<Element> {
        // Java END_SWITCH："If we had #on, then this was already decreased there"——
        // 只有 #case 模式在此撤销 breakable 计数
        if !had_on {
            self.loop_nesting -= 1;
        }
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
    pub(crate) fn attempt_directive(&mut self) -> Result<Element> {
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
    pub(crate) fn return_directive(&mut self) -> Result<Element> {
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
    pub(crate) fn stop_directive(&mut self) -> Result<Element> {
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
    pub(crate) fn include_directive(&mut self) -> Result<Element> {
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
    pub(crate) fn import_directive(&mut self) -> Result<Element> {
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
}
