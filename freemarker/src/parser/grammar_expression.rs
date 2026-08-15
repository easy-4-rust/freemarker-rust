//! 表达式产生式（docs/03 §3：优先级低 → 高）。

use super::grammar_helpers::{
    builtin_var_of, camel_to_snake, is_known_builtin, literal_only_check, number_literal, tok_desc,
    BOOLEAN_ONLY, EQUALITY_CHECK, JAVA_BUILTIN_LIST, NUMBER_ONLY, STRING_ONLY,
};
use super::{Parser, EXPRESSION_START_PATTERNS};
use crate::core::{BuiltinVar, Expr, ExprKind, RangeKind};
use crate::error::Result;
use crate::parser::lexer::Tok;
use crate::span::Span;

impl<'a> Parser<'a> {
    pub(crate) fn expression(&mut self) -> Result<Expr> {
        self.or_expression()
    }

    /// OrExpression：`lhs (|| rhs)*`；每步两侧按 Java booleanLiteralOnly
    /// 拒绝字面量（FTL.jj :2044-2045）
    pub(crate) fn or_expression(&mut self) -> Result<Expr> {
        let mut lhs = self.and_expression()?;
        loop {
            let (t, _, _) = self.peek_tok()?;
            if t != Tok::Or {
                break;
            }
            self.next_tok()?;
            let rhs = self.and_expression()?;
            literal_only_check(self, &lhs, BOOLEAN_ONLY)?;
            literal_only_check(self, &rhs, BOOLEAN_ONLY)?;
            let span = lhs.span;
            lhs = Expr::new(ExprKind::Or(Box::new(lhs), Box::new(rhs)), span);
        }
        Ok(lhs)
    }

    /// AndExpression：`lhs (&& rhs)*`；每步两侧按 Java booleanLiteralOnly
    /// 拒绝字面量（FTL.jj :2021-2022）
    pub(crate) fn and_expression(&mut self) -> Result<Expr> {
        let mut lhs = self.equality_expression()?;
        loop {
            let (t, _, _) = self.peek_tok()?;
            if t != Tok::And {
                break;
            }
            self.next_tok()?;
            let rhs = self.equality_expression()?;
            literal_only_check(self, &lhs, BOOLEAN_ONLY)?;
            literal_only_check(self, &rhs, BOOLEAN_ONLY)?;
            let span = lhs.span;
            lhs = Expr::new(ExprKind::And(Box::new(lhs), Box::new(rhs)), span);
        }
        Ok(lhs)
    }

    /// EqualityExpression：`rel [(==|!=) rel]`（单一可选，非结合）；两侧按 Java
    /// 拒绝哈希/列表字面量（FTL.jj :1902-1911 notHashLiteral + notListLiteral）
    pub(crate) fn equality_expression(&mut self) -> Result<Expr> {
        let lhs = self.relational_expression()?;
        let (t, _, _) = self.peek_tok()?;
        match t {
            Tok::Eq => {
                self.next_tok()?;
                let rhs = self.relational_expression()?;
                literal_only_check(self, &lhs, EQUALITY_CHECK)?;
                literal_only_check(self, &rhs, EQUALITY_CHECK)?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Eq(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::NotEq => {
                self.next_tok()?;
                let rhs = self.relational_expression()?;
                literal_only_check(self, &lhs, EQUALITY_CHECK)?;
                literal_only_check(self, &rhs, EQUALITY_CHECK)?;
                let span = lhs.span;
                Ok(Expr::new(
                    ExprKind::NotEq(Box::new(lhs), Box::new(rhs)),
                    span,
                ))
            }
            _ => Ok(lhs),
        }
    }

    /// RelationalExpression：`range [(<|<=|>|>=) range]`（单一可选）；两侧按 Java
    /// numberLiteralOnly 拒绝字面量（FTL.jj :1948-1949）
    pub(crate) fn relational_expression(&mut self) -> Result<Expr> {
        let lhs = self.range_expression()?;
        let (t, _, _) = self.peek_tok()?;
        match t {
            Tok::Lt => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                literal_only_check(self, &lhs, NUMBER_ONLY)?;
                literal_only_check(self, &rhs, NUMBER_ONLY)?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Lt(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::Lte => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                literal_only_check(self, &lhs, NUMBER_ONLY)?;
                literal_only_check(self, &rhs, NUMBER_ONLY)?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Lte(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::Gt => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                literal_only_check(self, &lhs, NUMBER_ONLY)?;
                literal_only_check(self, &rhs, NUMBER_ONLY)?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Gt(Box::new(lhs), Box::new(rhs)), span))
            }
            Tok::Gte => {
                self.next_tok()?;
                let rhs = self.range_expression()?;
                literal_only_check(self, &lhs, NUMBER_ONLY)?;
                literal_only_check(self, &rhs, NUMBER_ONLY)?;
                let span = lhs.span;
                Ok(Expr::new(ExprKind::Gte(Box::new(lhs), Box::new(rhs)), span))
            }
            _ => Ok(lhs),
        }
    }

    /// RangeExpression：`additive [(..<|..*|..) [additive]]`；两侧按 Java
    /// numberLiteralOnly 拒绝字面量（FTL.jj :1991-1993）
    pub(crate) fn range_expression(&mut self) -> Result<Expr> {
        let lhs = self.additive_expression()?;
        let (t, _, _) = self.peek_tok()?;
        match t {
            Tok::DotDotLess => {
                self.next_tok()?;
                let end = self.additive_expression()?;
                literal_only_check(self, &lhs, NUMBER_ONLY)?;
                literal_only_check(self, &end, NUMBER_ONLY)?;
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
                literal_only_check(self, &lhs, NUMBER_ONLY)?;
                literal_only_check(self, &end, NUMBER_ONLY)?;
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
                    let end = self.additive_expression()?;
                    literal_only_check(self, &lhs, NUMBER_ONLY)?;
                    literal_only_check(self, &end, NUMBER_ONLY)?;
                    (Some(Box::new(end)), RangeKind::Inclusive)
                } else {
                    literal_only_check(self, &lhs, NUMBER_ONLY)?;
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

    /// AdditiveExpression：`mul ((+|-) mul)*`；`-` 两侧按 Java numberLiteralOnly
    /// 拒绝字面量（FTL.jj :1843-1844，jar 实测 `${1 - "a"}` 解析期报
    /// "Found string literal: \"a\". Expecting: number"）；`+` 为连接运算不校验
    pub(crate) fn additive_expression(&mut self) -> Result<Expr> {
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
                    // Java AdditiveExpression（FTL.jj :1826-1850）：仅 SUBTRACTION
                    // 分支调用 numberLiteralOnly(lhs/rhs)（:1843-1844）
                    literal_only_check(self, &lhs, NUMBER_ONLY)?;
                    literal_only_check(self, &rhs, NUMBER_ONLY)?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Sub(Box::new(lhs), Box::new(rhs)), span);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    /// MultiplicativeExpression：`unary ((*|/|%) unary)*`；每步两侧按 Java
    /// numberLiteralOnly 拒绝字面量（FTL.jj :1880-1881）
    pub(crate) fn multiplicative_expression(&mut self) -> Result<Expr> {
        let mut lhs = self.unary_expression()?;
        loop {
            let (t, _, _) = self.peek_tok()?;
            match t {
                Tok::Times => {
                    self.next_tok()?;
                    let rhs = self.unary_expression()?;
                    literal_only_check(self, &lhs, NUMBER_ONLY)?;
                    literal_only_check(self, &rhs, NUMBER_ONLY)?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Mul(Box::new(lhs), Box::new(rhs)), span);
                }
                Tok::Divide => {
                    self.next_tok()?;
                    let rhs = self.unary_expression()?;
                    literal_only_check(self, &lhs, NUMBER_ONLY)?;
                    literal_only_check(self, &rhs, NUMBER_ONLY)?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Div(Box::new(lhs), Box::new(rhs)), span);
                }
                Tok::Percent => {
                    self.next_tok()?;
                    let rhs = self.unary_expression()?;
                    literal_only_check(self, &lhs, NUMBER_ONLY)?;
                    literal_only_check(self, &rhs, NUMBER_ONLY)?;
                    let span = lhs.span;
                    lhs = Expr::new(ExprKind::Mod(Box::new(lhs), Box::new(rhs)), span);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    /// UnaryExpression：UnaryPlusMinus / NotExpression / PrimaryExpression
    pub(crate) fn unary_expression(&mut self) -> Result<Expr> {
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
    pub(crate) fn primary_expression(&mut self) -> Result<Expr> {
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
    pub(crate) fn dot_variable(&mut self, target: Expr, _l: u32, _c: u32) -> Result<Expr> {
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
    pub(crate) fn dynamic_key(&mut self, target: Expr, _l: u32, _c: u32) -> Result<Expr> {
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
    pub(crate) fn builtin(&mut self, target: Expr) -> Result<Expr> {
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
        // Java BuiltIn.newBuiltIn（BuiltIn.java:349-397）：未知内建名在**解析期**报
        // "Unknown built-in: \"{name}\". Help (latest version): ..." + 字母序内建
        // 名清单（位置 = 内建名 token，jar 实测 unknown_builtin 基线 col 5）；
        // 清单文本与换行规则逐字对齐（name 排序 + 首字母换行 + `, ` 分隔）
        if !is_known_builtin(&name) {
            return Err(self.err(
                nl,
                nc,
                format!(
                    "Unknown built-in: \"{name}\". Help (latest version): https://freemarker.apache.org/docs/ref_builtins.html; you're using FreeMarker 2.3.34.\nThe alphabetical list of built-ins:\n{JAVA_BUILTIN_LIST}"
                ),
            ));
        }
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
    pub(crate) fn default_to(&mut self, target: Expr, l: u32, c: u32) -> Result<Expr> {
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
    pub(crate) fn in_named_arg_value(&self) -> bool {
        self.named_arg_depth > 0
    }
    pub(crate) fn atomic_expression(&mut self) -> Result<Expr> {
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
                        // Java BuiltinVariable.java:258-262：GET_OPTIONAL_TEMPLATE(_CC)
                        // 是两个独立名称（错误消息用各自方法名），须在归一化前区分
                        if name == "getOptionalTemplate" {
                            return Ok(Expr::new(
                                ExprKind::BuiltinVar(BuiltinVar::GetOptionalTemplateCc),
                                Span::new(nl, nc),
                            ));
                        }
                        // Java BuiltinVariable.java:81-82：CALLER_TEMPLATE_NAME(_CC)
                        // 同理（错误消息 "Can't get .callerTemplateName here..." 用
                        // 各自字面名，BuiltinVariable.java:285-293 getRequiredMacroContext）
                        if name == "callerTemplateName" {
                            return Ok(Expr::new(
                                ExprKind::BuiltinVar(BuiltinVar::CallerTemplateNameCc),
                                Span::new(nl, nc),
                            ));
                        }
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
                                    "The built-in variable \".{name}\" doesn't exist. The allowed special variable names are: namespace, main, globals, locals, data_model, vars, lang, locale, locale_object, time_zone, template_name, main_template_name, current_template_name, caller_template_name, node, current_node, error, output_encoding, output_format, auto_esc, url_escaping_charset, version, incompatible_improvements, args, now, get_optional_template."
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
                let (t, tl, tc) = self.next_tok()?;
                if t != Tok::CloseParen {
                    if t == Tok::Eof {
                        // Java EOF 分支：表达式状态期望 CLOSE_PAREN → desc "\"(\""
                        // （jar 实测 testUnclosedDirectives `${(blah`）
                        return Err(self.eof_unclosed(&["\"(\""]));
                    }
                    return Err(self.err(
                        tl,
                        tc,
                        format!(
                            "Expected \")\" to close the parenthesized expression, but found {}.",
                            tok_desc(&t)
                        ),
                    ));
                }
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
                        // Java stringLiteralOnly（FTL.jj :2565,2575）：键仅允许字符串
                        // 字面量——数字/列表/哈希/布尔字面量按 notXxxLiteral 逐字报错
                        // （jar 实测 `${{1: 2}}` → "Found number literal: 1. Expecting
                        // string" 于键位置）；标识符、点表达式等均可（求值期须为字符串）
                        literal_only_check(self, &key, STRING_ONLY)?;
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
            other => {
                // JavaCC ParseException 标准格式（ParseException :394-465 渲染
                // expectedTokenSequences；FTL.jj 表达式起始 LOOKAHEAD，jar 实测
                // parse_invalid_char 基线）。EOF 走 getOrRenderDescription 的 EOF
                // 分支——primary 起始状态 expected 无 END token → 无 unclosed 段
                if other == Tok::Eof {
                    return Err(self.eof_unclosed(&[]));
                }
                Err(self.err(
                    l,
                    c,
                    format!(
                        "Encountered {}, but was expecting one of these patterns:\n    {}",
                        tok_desc(&other),
                        EXPRESSION_START_PATTERNS.join("\n    ")
                    ),
                ))
            }
        }
    }
}
