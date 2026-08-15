//! 赋值/宏/call/嵌套指令实现。

use super::grammar_helpers::{
    assign_op_of, assignment_element, call_target, call_target_canonical, tok_desc, AssignScope,
    BlockStop,
};
use super::Parser;
use crate::core::{AssignOp, Element, ElementKind, Expr, ExprKind, MacroDef, MacroParam};
use crate::error::Result;
use crate::parser::lexer::Tok;
use crate::span::Span;

impl<'a> Parser<'a> {
    pub(crate) fn assign_directive(&mut self, scope: AssignScope) -> Result<Element> {
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
                // `expr in ns`（Java：`[id = <IN> nsExp = Expression()]`——nsExp 为任意
                // 表达式，运行期 eval 后检查类型，Assignment.java:112-122）
                let namespace = if self.peek_tok()?.0 == Tok::In {
                    self.next_tok()?;
                    if scope != AssignScope::Namespace {
                        return Err(self.err(line, col, "Cannot assign to namespace here."));
                    }
                    Some(self.expression()?)
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
                let mut namespace: Option<Expr> = None;
                if op_tok == Tok::In {
                    // `in ns` 在块形式中紧跟名字之后（Java：`[id = <IN> nsExp = Expression()]`）
                    if scope != AssignScope::Namespace {
                        return Err(self.err(line, col, "Cannot assign to namespace here."));
                    }
                    namespace = Some(self.expression()?);
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
    pub(crate) fn macro_directive(&mut self, is_function: bool) -> Result<Element> {
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
        let saved_continue = self.continue_nesting;
        self.loop_nesting = 0;
        self.continue_nesting = 0;
        if is_function {
            self.in_function += 1;
        } else {
            self.in_macro += 1;
        }
        // 注意：即使 parse_block 报错（如宏体内非法 `<#break>`）也必须恢复
        // loop_nesting，否则外层 #list 结束处 `loop_nesting -= 1` 会下溢 panic
        let body_res = self.parse_block(&["macro", "function"], &[]);
        if is_function {
            self.in_function -= 1;
        } else {
            self.in_macro -= 1;
        }
        self.loop_nesting = saved_loop;
        self.continue_nesting = saved_continue;
        let (body, stop) = body_res?;

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
            // Java Macro 的 template 字段（TemplateObject.setLocation）：解析期绑定
            // 定义所在模板（`.caller_template_name` 的调用点词法模板判定用）
            template_name: self.name.clone(),
            span: Span::new(line, col),
        };
        self.macros.insert(name, def.clone());
        Ok(Element::new(
            ElementKind::Macro { def },
            Span::new(line, col),
        ))
    }

    /// `<@callee [named|positional args] [; bodyParam,...]>body</@callee>`（UnifiedMacroTransform）
    pub(crate) fn parse_call(&mut self) -> Result<Element> {
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
    pub(crate) fn callee_expression(&mut self) -> Result<Expr> {
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
    pub(crate) fn nested_directive(&mut self) -> Result<Element> {
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
}
