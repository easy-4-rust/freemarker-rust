//! 语法分析辅助函数与类型。
//!
//! 包含：枚举类型（BlockStop/AssignScope/IterCtx）、字面量校验（literal_only_check）、
//! 数字/字符串字面量解析、内建名判定、camelCase→snake_case 归一化、token 描述等。

use super::Parser;
use crate::core::{
    AssignOp, BuiltinVar, CallTarget, Element, ElementKind, Expr, ExprKind, StrPart,
};
use crate::error::Result;
use crate::parser::lexer::Tok;
use crate::span::Span;
use crate::value::TNumber;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// 辅助
// ---------------------------------------------------------------------------

/// 块解析终止原因
pub(super) enum BlockStop {
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
pub(super) enum AssignScope {
    Namespace,
    Global,
    Local,
}

/// 迭代块解析上下文 —— 对应 Java `ParserIteratorBlockContext`（FTL.jj 的
/// iteratorBlockContexts 栈；`#items`/`#sep` 的嵌套校验与 `#list` 无 as 校验）
pub(super) struct IterCtx {
    /// 所属 #list/#foreach 是否带 `as loopVar`（带则 #items 非法）
    pub(super) has_loop_var: bool,
    /// 所属是否为 `<#foreach>`（Java：foreach 不支持嵌套 #items）
    pub(super) is_foreach: bool,
    /// 该 #list 是否已进入过 #items（Java iterCtx.kind == ITERATOR_BLOCK_KIND_ITEMS，
    /// 进入后不重置 —— `#list` 无 as 的结束校验用）
    pub(super) is_items: bool,
    /// 是否有未闭合的 #items 块（Java iterCtx.loopVarName != null；`</#items>` 时
    /// 重置 —— 同一 #list 中顺序多个 #items 是合法的，list3 用例的 switch 分支）
    pub(super) items_open: bool,
}

/// token → 赋值操作符（多赋值续项前瞻用；非赋值符返回 None）
/// Java FTL.jj 的 `notXxxLiteral(exp, expected)` 字面量校验族（:399-459）：
/// 算术（`-`/`*`/`/`/`%`）与关系（`<`/`<=`/`>`/`>=`）与范围操作数 → numberLiteralOnly；
/// `&&`/`||` 操作数 → booleanLiteralOnly；`==`/`!=` 操作数 → notHashLiteral +
/// notListLiteral；哈希字面量键 → stringLiteralOnly；`#{...}` → numberLiteralOnly。
/// 消息逐字对齐 notStringLiteral :399-408 / notNumberLiteral :410-417 /
/// notBooleanLiteral :421-428 / notHashLiteral :430-438 / notListLiteral :440-451
/// （canonical 形式见 literal_canonical）。
#[derive(Clone, Copy)]
pub(super) struct LiteralCheck {
    /// 拒绝的字面量种类位掩码：1=字符串, 2=列表, 4=哈希, 8=布尔, 16=数字
    mask: u8,
    /// 期望类型描述（Java `expected` 参数：number / boolean (true/false) / string /
    /// different type for equality check）
    expected: &'static str,
}

impl LiteralCheck {
    const fn all_but(mask: u8, expected: &'static str) -> Self {
        LiteralCheck { mask, expected }
    }
}

/// numberLiteralOnly（FTL.jj :454-459）：`#{}` 与算术/关系/范围操作数校验
pub(super) const NUMBER_ONLY: LiteralCheck = LiteralCheck::all_but(1 | 2 | 4 | 8, "number");
/// booleanLiteralOnly（FTL.jj :475-480）：`&&`/`||` 操作数校验
pub(super) const BOOLEAN_ONLY: LiteralCheck =
    LiteralCheck::all_but(1 | 2 | 4 | 16, "boolean (true/false)");
/// stringLiteralOnly（FTL.jj :464-473）：哈希字面量键校验
pub(super) const STRING_ONLY: LiteralCheck = LiteralCheck::all_but(2 | 4 | 8 | 16, "string");
/// EqualityExpression（FTL.jj :1902-1911）：`==`/`!=` 拒绝哈希与列表字面量
pub(super) const EQUALITY_CHECK: LiteralCheck =
    LiteralCheck::all_but(2 | 4, "different type for equality check");

pub(super) fn literal_only_check(p: &Parser, e: &Expr, check: LiteralCheck) -> Result<()> {
    use crate::core::ExprKind as K;
    let (l, c) = (e.span.line, e.span.col);
    let msg = match &e.kind {
        K::Str(_) | K::InterpStr(_) if check.mask & 1 != 0 => {
            // Java notStringLiteral 的 expected 前有冒号（"Expecting: number"），
            // 其余 notXxxLiteral 无冒号（jar 实测逐字）
            format!(
                "Found string literal: {}. Expecting: {}",
                literal_canonical(e),
                check.expected
            )
        }
        K::ListLit(_) if check.mask & 2 != 0 => {
            format!(
                "Found list literal: {}. Expecting {}",
                literal_canonical(e),
                check.expected
            )
        }
        K::HashLit(_) if check.mask & 4 != 0 => {
            format!(
                "Found hash literal: {}. Expecting {}",
                literal_canonical(e),
                check.expected
            )
        }
        K::Bool(_) if check.mask & 8 != 0 => {
            format!(
                "Found: {} literal. Expecting {}",
                literal_canonical(e),
                check.expected
            )
        }
        K::Num(_) if check.mask & 16 != 0 => {
            format!(
                "Found number literal: {}. Expecting {}",
                literal_canonical(e),
                check.expected
            )
        }
        _ => return Ok(()),
    };
    Err(p.err(l, c, msg))
}

/// 兼容旧调用点（`#{...}` 字面量校验）
pub(super) fn number_literal_only(p: &Parser, e: &Expr) -> Result<()> {
    literal_only_check(p, e, NUMBER_ONLY)
}

/// 字面量 canonical 形式（Java `Expression.getCanonicalForm` 的字面量子集；
/// 供 `#{}` 字面量校验消息逐字对齐）
pub(super) fn literal_canonical(e: &Expr) -> String {
    use crate::core::ExprKind as K;
    match &e.kind {
        K::Str(s) => format!("\"{s}\""),
        K::InterpStr(parts) => {
            let inner: String = parts
                .iter()
                .map(|p| match p {
                    StrPart::Text(t) => t.clone(),
                    StrPart::Interp(i) => format!("${{{}}}", literal_canonical(i)),
                })
                .collect();
            format!("\"{inner}\"")
        }
        K::Num(n) => n.to_plain_string(),
        K::Bool(b) => b.to_string(),
        K::ListLit(items) => format!(
            "[{}]",
            items
                .iter()
                .map(literal_canonical)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        K::HashLit(pairs) => format!(
            "{{{}}}",
            pairs
                .iter()
                .map(|(k, v)| format!("{}: {}", literal_canonical(k), literal_canonical(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        other => crate::core::environment::expr_desc(&Expr::new(other.clone(), e.span)),
    }
}

/// 旧式 `#{e ; fmt}` 格式串解析 —— 对应 Java NumericalOutput 的
/// StringTokenizer 循环（FTL.jj 2645-2687）：`m`/`M` 交替分隔数字；
/// 错误消息逐字对齐：
/// - "Invalid format specifier {fmt}"（结构非法 / m、M 重复）
/// - "Invalid number in the format specifier {fmt}"（数字解析失败）
/// - "Invalid format specification, at least one of m and M must be specified!"
/// - "Invalid format specification, min cannot be greater than max!"
/// - "Cannot specify more than 50 fraction digits"
pub(super) fn parse_legacy_number_format(
    p: &Parser,
    fmt: &str,
    l: u32,
    c: u32,
) -> Result<(u32, u32)> {
    // StringTokenizer(fmt, "mM", true)：分隔符 m/M 作为独立 token
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in fmt.chars() {
        if ch == 'm' || ch == 'M' {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            tokens.push(ch.to_string());
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    let mut typ: Option<char> = None;
    let mut min_frac: Option<u32> = None;
    let mut max_frac: Option<u32> = None;
    for tok in &tokens {
        match typ {
            Some(t) => {
                // Java：Integer.parseInt(token) 失败 → "Invalid number..."
                let n: u32 = match tok.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        return Err(p.err(
                            l,
                            c,
                            format!("Invalid number in the format specifier {fmt}"),
                        ))
                    }
                };
                match t {
                    'm' => {
                        // Java：minFrac != -1 → "Invalid formatting string" → 包装为
                        // "Invalid format specifier {fmt}"
                        if min_frac.is_some() {
                            return Err(p.err(l, c, format!("Invalid format specifier {fmt}")));
                        }
                        min_frac = Some(n);
                    }
                    'M' => {
                        if max_frac.is_some() {
                            return Err(p.err(l, c, format!("Invalid format specifier {fmt}")));
                        }
                        max_frac = Some(n);
                    }
                    _ => unreachable!("m/M 分隔符 token"),
                }
                typ = None;
            }
            None => {
                if tok == "m" {
                    typ = Some('m');
                } else if tok == "M" {
                    typ = Some('M');
                } else {
                    return Err(p.err(l, c, format!("Invalid format specifier {fmt}")));
                }
            }
        }
    }
    // Java :2687-2701：maxFrac 缺省 = minFrac；minFrac 缺省 = 0
    let max_frac = match max_frac {
        Some(v) => v,
        None => match min_frac {
            Some(v) => v,
            None => {
                return Err(p.err(
                    l,
                    c,
                    "Invalid format specification, at least one of m and M must be specified!",
                ))
            }
        },
    };
    let min_frac = min_frac.unwrap_or(0);
    if min_frac > max_frac {
        return Err(p.err(
            l,
            c,
            "Invalid format specification, min cannot be greater than max!",
        ));
    }
    if min_frac > 50 || max_frac > 50 {
        return Err(p.err(l, c, "Cannot specify more than 50 fraction digits"));
    }
    Ok((min_frac, max_frac))
}

pub(super) fn assign_op_of(t: &Tok) -> Option<AssignOp> {
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
pub(super) fn assignment_element(
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
pub(super) fn header_bool(e: &Expr) -> Option<bool> {
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

/// Java 2.3.34 内建名清单文本（BuiltIn.newBuiltIn :354-397 逐字：
/// BUILT_INS_BY_NAME 键排序，首字母变化换行，`, ` 分隔；LEGACY 命名约定视图）。
/// 文本与换行规则从 `error/expected_messages/unknown_builtin.txt` 基线生成
/// （183 个内建名；Rust 扩展内建 has_previous/is_even/is_lambda/is_nothing/
/// is_odd/iso_fz/replace_re 不在 Java 清单中，单独放行）
pub(crate) const JAVA_BUILTIN_LIST: &str = concat!(
    "abs, absolute_template_name, ancestors, api,\n",
    "blank_to_null, boolean, byte,\n",
    "c, c_lower_case, c_upper_case, cap_first, capitalize, ceiling, children, chop_linebreak, chunk, cn, contains, counter,\n",
    "date, date_if_unknown, datetime, datetime_if_unknown, default, double, drop_while,\n",
    "empty_to_null, ends_with, ensure_ends_with, ensure_starts_with, esc, eval, eval_json, exists,\n",
    "filter, first, float, floor,\n",
    "groups,\n",
    "has_api, has_content, has_next, html,\n",
    "if_exists, index, index_of, int, interpret, is_boolean, is_collection, is_collection_ex, is_date, is_date_like, is_date_only, is_datetime, is_directive, is_enumerable, is_even_item, is_first, is_hash, is_hash_ex, is_indexable, is_infinite, is_last, is_macro, is_markup_output, is_method, is_nan, is_node, is_number, is_odd_item, is_sequence, is_string, is_time, is_transform, is_unknown_date_like, iso, iso_h, iso_h_nz, iso_local, iso_local_h, iso_local_h_nz, iso_local_m, iso_local_m_nz, iso_local_ms, iso_local_ms_nz, iso_local_nz, iso_m, iso_m_nz, iso_ms, iso_ms_nz, iso_nz, iso_utc, iso_utc_fz, iso_utc_h, iso_utc_h_nz, iso_utc_m, iso_utc_m_nz, iso_utc_ms, iso_utc_ms_nz, iso_utc_nz, item_cycle, item_parity, item_parity_cap,\n",
    "j_string, join, js_string, json_string,\n",
    "keep_after, keep_after_last, keep_before, keep_before_last, keys,\n",
    "last, last_index_of, left_pad, length, long, lower_abc, lower_case,\n",
    "map, markup_string, matches, max, min,\n",
    "namespace, new, next_sibling, no_esc, node_name, node_namespace, node_type, number, number_to_date, number_to_datetime, number_to_time,\n",
    "parent, previous_sibling,\n",
    "remove_beginning, remove_ending, replace, reverse, right_pad, root, round, rtf,\n",
    "seq_contains, seq_index_of, seq_last_index_of, sequence, short, size, sort, sort_by, split, starts_with, string, substring, switch,\n",
    "take_while, then, time, time_if_unknown, trim, trim_to_null, truncate, truncate_c, truncate_c_m, truncate_m, truncate_w, truncate_w_m,\n",
    "uncap_first, upper_abc, upper_case, url, url_path,\n",
    "values,\n",
    "web_safe, with_args, with_args_last, word_list,\n",
    "xhtml, xml",
);

/// 内建名是否合法（Java BUILT_INS_BY_NAME 的 legacy 视图 + 本引擎扩展内建）
pub(super) fn is_known_builtin(name: &str) -> bool {
    // 直接查清单文本（性能：解析期一次线性扫描即可；清单为静态文本）
    if JAVA_BUILTIN_LIST
        .split([',', '\n'])
        .any(|n| n.trim() == name)
    {
        return true;
    }
    // Rust 扩展内建（不在 Java 2.3.34 清单中；放行到求值期）
    matches!(
        name,
        "has_previous"
            | "is_even"
            | "is_lambda"
            | "is_nothing"
            | "is_odd"
            | "iso_fz"
            | "replace_re"
    )
}

/// 内建/内置变量名 camelCase → legacy 蛇形归一化（对应 Java
/// `_CoreStringUtils.toFTLLegacyNamingConvention`：`capFirst` → `cap_first`、
/// `templateName` → `template_name`；全大写名保持原样，求值期报 Unknown built-in）
pub(super) fn camel_to_snake(name: &str) -> String {
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
pub(super) fn builtin_var_of(name: &str) -> Option<BuiltinVar> {
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
        "caller_template_name" => BuiltinVar::CallerTemplateName,
        "node" | "current_node" => BuiltinVar::Node,
        "error" => BuiltinVar::Error,
        "output_encoding" => BuiltinVar::OutputEncoding,
        "output_format" => BuiltinVar::OutputFormat,
        "auto_esc" => BuiltinVar::AutoEsc,
        "url_escaping_charset" => BuiltinVar::UrlEscapingCharset,
        "version" => BuiltinVar::Version,
        "incompatible_improvements" => BuiltinVar::IncompatibleImprovements,
        "args" => BuiltinVar::Args,
        "get_optional_template" => BuiltinVar::GetOptionalTemplate,
        _ => return None,
    })
}

/// 数字字面量 → TNumber（契约映射：1→Int、1L→Long、1F→Float、1D→Double、
/// 1.5/1e3→Decimal、超 i64 整数→BigInt；0x 十六进制；L/F/D/B 后缀）
pub(super) fn number_literal(raw: &str) -> Option<TNumber> {
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
pub(super) fn call_target(e: &Expr) -> CallTarget {
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
pub(super) fn call_target_canonical(e: &Expr) -> Option<String> {
    match &e.kind {
        ExprKind::Ident(n) => Some(n.clone()),
        ExprKind::Dot { target, name } => {
            call_target_canonical(target).map(|t| format!("{t}.{name}"))
        }
        _ => None,
    }
}

/// token 的人类可读描述（错误消息）
pub(super) fn tok_desc(t: &Tok) -> String {
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
pub(super) fn find_matching_brace(s: &str) -> Option<(&str, &str)> {
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
pub(super) fn trim_first_line_trailing(text: &mut String, heinous_drop: bool) -> bool {
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
pub(super) fn trim_last_line_leading(text: &mut String, begin_col: u32) {
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
