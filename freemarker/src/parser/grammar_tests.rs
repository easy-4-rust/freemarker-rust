//! 表达式解析测试。

use super::parse;
use crate::core::{BuiltinVar, ElementKind, Expr, ExprKind, RangeKind, StrPart};
use crate::span::Span;
use crate::template::{Configuration, Template};
use crate::value::TNumber;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use std::rc::Rc;
use std::str::FromStr;

fn cfg() -> Rc<Configuration> {
    Rc::new(Configuration::new())
}

fn parse_ok(src: &str) -> Template {
    parse(&cfg(), "t", src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"))
}

#[allow(dead_code)]
fn parse_err(src: &str) -> String {
    match parse(&cfg(), "t", src) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected parse error for {src:?}"),
    }
}

fn expr_of(src: &str) -> ExprKind {
    let t = parse_ok(&format!("${{{src}}}"));
    match &t.root[0].kind {
        ElementKind::Interpolation { expr, .. } => expr.kind.clone(),
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
    // Java TokenMgrError 词法错误格式（jar 实测 parse_unclosed_string 基线）
    let msg = parse_err(r#"${"abc}"#);
    assert!(
        msg.contains("Lexical error: encountered <EOF> after \"\\\"abc}\"."),
        "{msg}"
    );
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
    // 非字符串键 → 解析错误（Java stringLiteralOnly：数字字面量作键，
    // 消息逐字 "Found number literal: 1. Expecting string"，jar 实测）
    let msg = parse_err(r#"${ {1: 2} }"#);
    assert!(
        msg.contains("Found number literal: 1. Expecting string"),
        "{msg}"
    );
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
