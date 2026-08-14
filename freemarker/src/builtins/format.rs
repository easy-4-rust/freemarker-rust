//! 数字格式化 —— 对应 Java `freemarker.core.CTemplateNumberFormat`（?c/?cn 的 C 格式，
//! IcI ≥ 2.3.32 默认 `JavaScriptOrJSONCFormat`）+ `JavaTemplateNumberFormat`（?string(pattern)
//! 的 DecimalFormat 子集）。文档见 docs/08 §2。
//!
//! v1 范围：
//! - C 格式（"c"/"computer"/?c/?cn）：整数 plain、BigDecimal stripTrailingZeros、
//!   Double/Float 用 Java 最短表示（指数 E 大写、范围对齐 Double.toString）；
//! - c_format 变体（StandardCFormats：JavaScript or JSON/JavaScript/JSON/Java/XS/legacy）：
//!   字符串转义与 Infinity/NaN 符号按变体分派（JavaCFormat.java:61/XSCFormat.java:73）；
//! - DecimalFormat 子集：`0`/`#`/`.`/`,`（分组）/`'...'`（引号字面量），HALF_EVEN 舍入，
//!   locale 相关小数点与分组符（en/fr/de/es/tr）；完整模式（E、%、‰、货币等）属 P4。

use crate::core::built_ins_for_strings_encoding::{java_string_enc, js_string_enc};
use crate::core::Environment;
use crate::error::{Result, TemplateError};
use crate::value::TNumber;
use bigdecimal::{BigDecimal, RoundingMode};
use std::str::FromStr;

/// C 格式变体 —— 对应 Java `freemarker.core.StandardCFormats` 注册表
/// （JavaScriptOrJSONCFormat/JavaScriptCFormat/JSONCFormat/JavaCFormat/XSCFormat/
/// LegacyCFormat，名字见各类的 NAME 常量）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CFormatKind {
    /// 默认（ICI ≥ 2.3.32）：JavaScript 或 JSON 兼容
    #[default]
    JavaScriptOrJson,
    JavaScript,
    Json,
    Java,
    Legacy,
    /// XML Schema 风格：字符串不转义、null → ""
    Xs,
}

impl CFormatKind {
    /// 按 StandardCFormats 注册名解析（Java c_format 设置值）
    pub fn parse(name: &str) -> Option<CFormatKind> {
        Some(match name {
            "JavaScript or JSON" => CFormatKind::JavaScriptOrJson,
            "JavaScript" => CFormatKind::JavaScript,
            "JSON" => CFormatKind::Json,
            "Java" => CFormatKind::Java,
            "legacy" => CFormatKind::Legacy,
            "XS" => CFormatKind::Xs,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            CFormatKind::JavaScriptOrJson => "JavaScript or JSON",
            CFormatKind::JavaScript => "JavaScript",
            CFormatKind::Json => "JSON",
            CFormatKind::Java => "Java",
            CFormatKind::Legacy => "legacy",
            CFormatKind::Xs => "XS",
        }
    }
}

/// ?c/?cn 的字符串输出（Java CFormat.formatString）：
/// - JavaScript or JSON / JSON / legacy：jsStringEnc(JS_OR_JSON, QUOTATION_MARK)
///   （LegacyCFormat.java:89-91 与默认同转义；数字符号才不同）
/// - JavaScript：jsStringEnc(JS, QUOTATION_MARK)（' 不转义，\x 2 位 hex）
/// - Java：javaStringEnc(s, true)（JavaCFormat.java:61-63）
/// - XS：原样（XSCFormat.java:73-74：假定已有 XML 自动转义）
///
/// 注：QUOTATION_MARK 语义 = 双引号包裹（StringUtil.jsStringEnc :1438 开头
/// sb.append('"')，:1524 结尾补 '"'）——?c 输出为 `"..."` 形式
pub fn format_c_string(s: &str, kind: CFormatKind) -> String {
    match kind {
        CFormatKind::JavaScriptOrJson | CFormatKind::Json | CFormatKind::Legacy => {
            format!("\"{}\"", js_string_enc(s, true))
        }
        CFormatKind::JavaScript => {
            // jsStringEnc(JS, QUOTATION_MARK)：' 不转义（StringUtil :1461-1463
            // quotation==QUOTATION_MARK → NO_ESC）；Rust js_string_enc(s,false)
            // 会转义 '——还原之，仅保留 \x 2 位 hex 差异
            format!("\"{}\"", js_string_enc(s, false).replace("\\'", "'"))
        }
        CFormatKind::Java => format!("\"{}\"", java_string_enc(s)),
        CFormatKind::Xs => s.to_string(),
    }
}

/// Infinity/NaN 符号（Java CFormat.getTemplateNumberFormat 的 CTemplateNumberFormat
/// 构造参数；JavaCFormat.java:38-40 / XSCFormat.java:44-46）
fn inf_nan_symbols(
    kind: CFormatKind,
    is_float: bool,
) -> (&'static str, &'static str, &'static str) {
    match kind {
        CFormatKind::Java => (
            if is_float {
                "Float.POSITIVE_INFINITY"
            } else {
                "Double.POSITIVE_INFINITY"
            },
            if is_float {
                "Float.NEGATIVE_INFINITY"
            } else {
                "Double.NEGATIVE_INFINITY"
            },
            if is_float { "Float.NaN" } else { "Double.NaN" },
        ),
        CFormatKind::Xs => ("INF", "-INF", "NaN"),
        _ => ("Infinity", "-Infinity", "NaN"),
    }
}

/// ?c/?cn 与 number_format="c"/"computer" 的 C 格式输出
/// （Java CTemplateNumberFormat.formatToPlainText；Infinity/NaN 符号按
/// c_format 变体分派）
pub fn format_c_number(n: &TNumber, kind: CFormatKind) -> String {
    match n {
        TNumber::Int(v) => v.to_string(),
        TNumber::Long(v) => v.to_string(),
        TNumber::BigInt(v) => v.to_string(),
        TNumber::Double(v) => format_c_double(*v, kind),
        TNumber::Float(v) => format_c_float(*v, kind),
        TNumber::Decimal(d) => format_c_big_decimal(d),
    }
}

fn format_c_big_decimal(d: &BigDecimal) -> String {
    // Java BigDecimal.stripTrailingZeros()：去掉尾零
    let stripped = d.normalized();
    if stripped.as_bigint_and_scale().1 <= 0 {
        // 整数（或 scale<0 的 E 表示）→ 避免指数形式（Java :142-151 toPlainString）
        stripped.to_plain_string()
    } else {
        // 其余按 BigDecimal.toString（小数值 scale>0 为普通十进制；scale=0/负为指数）
        stripped.to_string()
    }
}

fn format_c_double(n: f64, kind: CFormatKind) -> String {
    let (pos_inf, neg_inf, nan) = inf_nan_symbols(kind, false);
    if n == f64::INFINITY {
        return pos_inf.to_string();
    }
    if n == f64::NEG_INFINITY {
        return neg_inf.to_string();
    }
    if n.is_nan() {
        return nan.to_string();
    }
    if n.floor() == n {
        // 整数且 |n| <= 2^53（MAX_INCREMENT_1_DOUBLE）→ Long.toString
        if n.abs() <= 9_007_199_254_740_992.0 {
            return (n as i64).to_string();
        }
        // 超出则落入指数形式（Java 注释：ulp>1 不做整数化）
    } else {
        let abs = n.abs();
        // Double.toString 已用指数而 BigDecimal.toString 还未用的区间 → BigDecimal 值
        if abs < 1e-3 && abs > 1e-7 {
            return BigDecimal::from_str(&java_float_string(n))
                .map(|b| b.to_string())
                .unwrap_or_else(|_| java_float_string(n));
        }
        // 大数避免指数形式（Java :113-117）
        if abs >= 1e7 {
            return BigDecimal::from_str(&java_float_string(n))
                .map(|b| b.to_plain_string())
                .unwrap_or_else(|_| java_float_string(n));
        }
    }
    remove_redundant_dot0(&java_float_string(n))
}

fn format_c_float(n: f32, kind: CFormatKind) -> String {
    let (pos_inf, neg_inf, nan) = inf_nan_symbols(kind, true);
    if n == f32::INFINITY {
        return pos_inf.to_string();
    }
    if n == f32::NEG_INFINITY {
        return neg_inf.to_string();
    }
    if n.is_nan() {
        return nan.to_string();
    }
    if n.floor() == n {
        // |n| <= 2^24（MAX_INCREMENT_1_FLOAT）→ Long.toString
        if n.abs() <= 16_777_216.0 {
            return (n as i64).to_string();
        }
    } else {
        let abs = n.abs();
        if abs < 1e-3 && abs > 1e-7 {
            // Java：new BigDecimal(num.toString()).toString()
            // 注意用 f32 自身最短表示（不能先扩为 f64：1.2f → 1.2000000476837158d）
            let s = java_float_string_f32(n);
            return BigDecimal::from_str(&s).map(|b| b.to_string()).unwrap_or(s);
        }
        // float 无 absN >= 1E7 分支（Java 注释：那些数字对 float 而言都是整数）
    }
    remove_redundant_dot0(&java_float_string_f32(n))
}

/// 去掉冗余的 ".0"（Java `CTemplateNumberFormat.removeRedundantDot0`）：
/// 科学计数法中移除 mantissa 尾部的 ".0"（"1.0E-16" → "1E-16"）；
/// 普通十进制中移除末尾的 ".0"
fn remove_redundant_dot0(s: &str) -> String {
    if let Some(e_idx) = s.find('E') {
        let mantissa = &s[..e_idx];
        if let Some(rest) = mantissa.strip_suffix(".0") {
            return format!("{rest}{}", &s[e_idx..]);
        }
        s.to_string()
    } else if let Some(rest) = s.strip_suffix(".0") {
        rest.to_string()
    } else {
        s.to_string()
    }
}

/// Java `Double.toString`/`Float.toString` 的最短表示（指数用大写 E，范围对齐 Java）：
/// - `-3 <= exp <= 6` → 普通十进制（Java 对 10^-3 ≤ |x| < 10^7 用十进制）；
/// - 其余 → `d.ddddE±xx` 科学计数。
///   Rust `{:e}` 也是最短往返表示，只需把指数形态转换为 Java 风格。
fn java_float_string(n: f64) -> String {
    java_float_string_impl(&format!("{:e}", n))
}

/// f32 版（同 f64 逻辑）：先扩为 f64 再 `{:e}` 会得到 f64 的最短表示而非 f32 的
/// （`1.2f` → `1.2000000476837158`），故直接对 f32 用 `{:e}`（Rust 对 f32 同样
/// 输出可往返的最短表示，`1.2f` → `"1.2e0"`）。
fn java_float_string_f32(n: f32) -> String {
    java_float_string_impl(&format!("{:e}", n))
}

/// `{:e}` 输出 → Java 风格十进制/科学计数
/// （先剥符号再拼位数，避免负数时把 `-` 当数字位切分，如 `-1.2` → `-.12` 的 bug）
fn java_float_string_impl(s: &str) -> String {
    let (mant, exp) = s.split_once('e').expect("Rust {:e} always has exponent");
    let exp: i32 = exp.parse().expect("exponent parses");
    let (sign, mantissa) = match mant.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mant),
    };
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let int_digits = mantissa
        .split_once('.')
        .map_or(mantissa.len(), |(i, _)| i.len());
    // Java：exponent < -3 或 >= 7 → 科学计数
    if !(-3..7).contains(&exp) {
        // 去掉整数的 ".0"（"1.0E-16" → "1E-16"）：Java Double.toString 对整 mantissa 输出 "1.0"，
        // removeRedundantDot0 后续统一处理；这里保持与 Java toString 一致（含 .0）
        let m = if int_digits == digits.len() {
            format!("{}.0", &digits[..int_digits])
        } else {
            format!("{}.{}", &digits[..int_digits], &digits[int_digits..])
        };
        format!("{sign}{m}E{exp}")
    } else if exp < 0 {
        // 0.00...ddd
        let zeros = "0".repeat((-(exp + 1)) as usize);
        format!("{sign}0.{zeros}{digits}")
    } else {
        let point = (exp + 1) as usize;
        if point >= digits.len() {
            format!("{sign}{digits}{}.0", "0".repeat(point - digits.len()))
        } else {
            format!("{sign}{}.{}", &digits[..point], &digits[point..])
        }
    }
}

/// 十进制分组分隔符（locale 相关；对应 Java DecimalFormatSymbols，JDK 19+ 分组符）
fn group_separator(locale: &str) -> char {
    match locale.split('_').next().unwrap_or("en") {
        "fr" => '\u{202F}', // NARROW NO-BREAK SPACE
        "de" | "es" | "tr" | "it" | "pt" | "nl" | "sv" | "cs" | "pl" | "hu" | "ro" | "ru"
        | "uk" | "bg" | "el" | "fi" | "da" | "no" | "sk" | "sl" | "hr" | "lt" | "lv" | "et"
        | "id" | "vi" | "th" => '.',
        _ => ',',
    }
}

/// 十进制小数点分隔符（locale 相关）
fn decimal_separator(locale: &str) -> char {
    match locale.split('_').next().unwrap_or("en") {
        "fr" | "de" | "es" | "tr" | "it" | "pt" | "nl" | "sv" | "cs" | "pl" | "hu" | "ro"
        | "ru" | "uk" | "bg" | "el" | "fi" | "da" | "no" | "sk" | "sl" | "hr" | "lt" | "lv"
        | "et" | "id" | "vi" | "th" => ',',
        _ => '.',
    }
}

/// DecimalFormat 子集模式（Java ExtendedDecimalFormatParser 的 v1 基础版：
/// `0`/`#`/`.`/`,`（分组）/`'...'`（引号字面量））
pub struct DecimalFmt {
    pub prefix: String,
    pub suffix: String,
    pub min_int: usize,
    pub grouping: bool,
    pub min_frac: usize,
    pub max_frac: usize,
    pub decimal_sep: char,
    pub group_sep: char,
}

/// 解析 DecimalFormat 子集；模式中不含 0/# 时视为纯字面量（如 `'df'`）
pub fn parse_decimal_format(pattern: &str, locale: &str) -> Result<DecimalFmt> {
    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut int_part = String::new();
    let mut frac_part = String::new();
    let mut min_int = 0usize;
    let mut max_int = 0usize;
    let mut min_frac = 0usize;
    let mut max_frac = 0usize;
    let mut grouping = false;
    let mut seen_decimal = false;

    let chars: Vec<char> = pattern.chars().collect();
    // 先分离前缀/后缀字面量（第一个 0/# 之前、最后一个 0/# 之后；含引号内文本）
    // Java DecimalFormat：前缀 = 第一个有效位前的文本，后缀 = 最后有效位后的文本
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            // 引号字面量（含两个连续 '' 表示一个 '）
            let mut lit = String::new();
            i += 1;
            while i < chars.len() {
                if chars[i] == '\'' {
                    if i + 1 < chars.len() && chars[i + 1] == '\'' {
                        lit.push('\'');
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                lit.push(chars[i]);
                i += 1;
            }
            prefix.push_str(&lit);
            continue;
        }
        // 模式字符（0/#/./,/ 等）结束前缀
        if c == '0' || c == '#' || c == '.' || c == ',' {
            break;
        }
        prefix.push(c);
        i += 1;
    }
    // 数字区
    while i < chars.len() {
        let c = chars[i];
        match c {
            '0' | '#' => {
                if seen_decimal {
                    frac_part.push(c);
                    if c == '0' {
                        min_frac += 1;
                    }
                    max_frac += 1;
                } else {
                    int_part.push(c);
                    if c == '0' {
                        min_int += 1;
                    }
                    max_int += 1;
                }
            }
            '.' => {
                if seen_decimal {
                    return Err(TemplateError::misc(format!(
                        "Invalid number format pattern: {pattern}"
                    )));
                }
                seen_decimal = true;
            }
            ',' => grouping = true,
            '\'' => {
                // 数字区中的引号 → 之后都是后缀
                let mut lit = String::new();
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\'' {
                        if i + 1 < chars.len() && chars[i + 1] == '\'' {
                            lit.push('\'');
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    lit.push(chars[i]);
                    i += 1;
                }
                suffix.push_str(&lit);
                i += 1;
                while i < chars.len() {
                    suffix.push(chars[i]);
                    i += 1;
                }
                break;
            }
            _ => {
                // 数字区后的任意字符 → 后缀
                suffix.push(c);
            }
        }
        i += 1;
    }
    // 剩余为后缀
    if i < chars.len() {
        while i < chars.len() {
            suffix.push(chars[i]);
            i += 1;
        }
    }
    if min_int == 0 && max_int == 0 && min_frac == 0 && max_frac == 0 {
        // 纯字面量模式（如 "'df'"、"short"）
        return Ok(DecimalFmt {
            prefix,
            suffix,
            min_int: 0,
            grouping: false,
            min_frac: 0,
            max_frac: 0,
            decimal_sep: decimal_separator(locale),
            group_sep: group_separator(locale),
        });
    }
    Ok(DecimalFmt {
        prefix,
        suffix,
        min_int,
        grouping,
        min_frac,
        max_frac,
        decimal_sep: decimal_separator(locale),
        group_sep: group_separator(locale),
    })
}

/// 用 DecimalFormat 子集格式化数字（Java DecimalFormat.format；HALF_EVEN 舍入）
pub fn format_decimal(fmt: &DecimalFmt, n: &TNumber) -> String {
    // 整数快路径（Int/Long 值 BigDecimal 转换精确、无小数舍入——结果与慢路径逐字节一致，
    // 避免每次输出的 BigDecimal 构造 + to_plain_string 分配）
    match n {
        TNumber::Int(v) => return format_integer_decimal(fmt, *v as i64),
        TNumber::Long(v) => return format_integer_decimal(fmt, *v),
        _ => {}
    }
    // Java DecimalFormat 对 Float/Double 走快路径（JDK FastDecimalFormat）：
    // 以**最短往返表示**为基准（Float 先加宽为 Double，如 1.01f → 1.0099999904632568）
    // 再按 max_frac 舍入 —— 与慢路径（BigDecimal 精确值）结果不同
    // （如 2147483647.099999904632568 → 最短 "2147483647.1" → 直接输出）。
    // 其余类型（Int/Long/BigInt/Decimal）用精确值慢路径。
    let mut bd = match n {
        TNumber::Float(v) => BigDecimal::from_str(&format!("{}", *v as f64)).unwrap_or_default(),
        TNumber::Double(v) => BigDecimal::from_str(&format!("{v}")).unwrap_or_default(),
        _ => n.as_big_decimal(),
    };
    // 舍入到 max_frac（HALF_EVEN —— Java DecimalFormat 默认舍入模式）
    if fmt.max_frac == 0 {
        bd = bd.with_scale_round(0, RoundingMode::HalfEven);
    } else if bd.as_bigint_and_scale().1 > fmt.max_frac as i64 {
        bd = bd.with_scale_round(fmt.max_frac as i64, RoundingMode::HalfEven);
    }
    let (int_digits, frac_digits) = split_digits(&bd);
    // 整数部分：补齐 min_int（Java '0' 强制位）
    let mut int_s = int_digits;
    while int_s.len() < fmt.min_int {
        int_s.insert(0, '0');
    }
    if int_s.is_empty() {
        int_s.push('0');
    }
    // 分组（每 3 位一组，从右往左；负号不参与分组——Java DecimalFormat 只对数字位）
    if fmt.grouping && int_s.len() > 3 {
        let (sign, digits) = match int_s.strip_prefix('-') {
            Some(d) => ("-", d),
            None => ("", int_s.as_str()),
        };
        if digits.len() > 3 {
            let chars: Vec<char> = digits.chars().collect();
            let n = chars.len();
            let first = n % 3;
            let mut out = String::new();
            let mut idx = 0;
            if first > 0 {
                out.extend(&chars[..first]);
                idx = first;
            }
            while idx < n {
                if !out.is_empty() {
                    out.push(fmt.group_sep);
                }
                out.extend(&chars[idx..idx + 3]);
                idx += 3;
            }
            int_s = format!("{sign}{out}");
        }
    }
    // 小数部分：舍入后补齐 min_frac，并剥除超出 min_frac 的尾零
    // （Java DecimalFormat：'#' 是可选位 —— 2.500 → "2.5"；'0' 是强制位）
    let mut frac_s = frac_digits;
    while frac_s.len() > fmt.min_frac && frac_s.ends_with('0') {
        frac_s.pop();
    }
    while frac_s.len() < fmt.min_frac {
        frac_s.push('0');
    }
    if frac_s.len() > fmt.max_frac {
        frac_s.truncate(fmt.max_frac);
    }
    let mut out = String::new();
    out.push_str(&fmt.prefix);
    out.push_str(&int_s);
    if !frac_s.is_empty() {
        out.push(fmt.decimal_sep);
        out.push_str(&frac_s);
    }
    out.push_str(&fmt.suffix);
    out
}

/// 整数格式化快路径（与 format_decimal 慢路径对 Int/Long 的结果逐字节一致：
/// 精确 BigDecimal 无小数位、无舍入；补齐 min_int、按 grouping 分组、
/// 小数部分补齐 min_frac 个 '0'）
fn format_integer_decimal(fmt: &DecimalFmt, v: i64) -> String {
    let mut int_s = v.to_string();
    while int_s.len() < fmt.min_int {
        int_s.insert(0, '0');
    }
    // 分组（每 3 位一组，从右往左；负号不参与分组——与慢路径一致）
    if fmt.grouping && int_s.len() > 3 {
        let (sign, digits) = match int_s.strip_prefix('-') {
            Some(d) => ("-", d),
            None => ("", int_s.as_str()),
        };
        if digits.len() > 3 {
            let chars: Vec<char> = digits.chars().collect();
            let n = chars.len();
            let first = n % 3;
            let mut out = String::new();
            let mut idx = 0;
            if first > 0 {
                out.extend(&chars[..first]);
                idx = first;
            }
            while idx < n {
                if !out.is_empty() {
                    out.push(fmt.group_sep);
                }
                out.extend(&chars[idx..idx + 3]);
                idx += 3;
            }
            int_s = format!("{sign}{out}");
        }
    }
    // 无前缀/后缀/小数位 → 直接返回（避免第二次 String 构造）
    if fmt.prefix.is_empty() && fmt.suffix.is_empty() && fmt.min_frac == 0 {
        return int_s;
    }
    let mut out = String::new();
    out.push_str(&fmt.prefix);
    out.push_str(&int_s);
    if fmt.min_frac > 0 {
        out.push(fmt.decimal_sep);
        for _ in 0..fmt.min_frac {
            out.push('0');
        }
    }
    out.push_str(&fmt.suffix);
    out
}

/// 数字 → 整/小数字符串（BigDecimal.toPlainString 拆开）
fn split_digits(bd: &BigDecimal) -> (String, String) {
    let s = bd.to_plain_string();
    match s.split_once('.') {
        Some((i, f)) => (i.to_string(), f.to_string()),
        None => (s, String::new()),
    }
}

/// 按 number_format 设置格式化数字（Java `env.getTemplateNumberFormat`；v1 子集）：
/// - "number" → Java `NumberFormat.getNumberInstance(locale)`（JavaTemplateNumberFormatFactory
///   :51-53）—— en_US 等为 `#,##0.###`（分组 + 至多 3 位小数、HALF_EVEN 舍入）；
/// - "c"/"computer" → C 格式
/// - 其余 → DecimalFormat 子集
pub fn format_number(env: &Environment, n: &TNumber) -> Result<String> {
    let fmt = env.settings.number_format.as_str();
    let locale = env.settings.locale.as_str();
    if let Some(name) = custom_format_name(fmt) {
        return Err(TemplateError::misc(format!(
            "No custom number format was defined with name {}",
            j_quote(&name)
        )));
    }
    if fmt == "number" || fmt.is_empty() {
        // 默认模式解析结果缓存（首次解析后复用，热路径避免每次模式解析；
        // 键为 (number_format, locale)，`<#setting>` 改动后自动失效）
        let mut cache = env.number_fmt_cache.borrow_mut();
        let df = match &*cache {
            Some((f, l, df)) if f == fmt && l == locale => df.clone(),
            _ => {
                let parsed = match parse_decimal_format("#,##0.###", locale) {
                    Ok(df) => df,
                    Err(_) => return Ok(n.to_plain_string()),
                };
                let rc = std::rc::Rc::new(parsed);
                *cache = Some((fmt.to_string(), locale.to_string(), rc.clone()));
                rc
            }
        };
        Ok(format_decimal(&df, n))
    } else if fmt == "c" || fmt == "computer" {
        Ok(format_c_number(n, CFormatKind::JavaScriptOrJson))
    } else {
        match parse_decimal_format(fmt, locale) {
            Ok(df) => Ok(format_decimal(&df, n)),
            Err(_) => Ok(n.to_plain_string()),
        }
    }
}

/// 与 format_number 相同，但显式指定格式串（?string('pattern') 用；
/// Java ?string 的格式串同样经 getTemplateNumberFormat → `@` 检查）
pub fn format_number_with(fmt: &str, locale: &str, n: &TNumber) -> Result<String> {
    if let Some(name) = custom_format_name(fmt) {
        return Err(TemplateError::misc(format!(
            "No custom number format was defined with name {}",
            j_quote(&name)
        )));
    }
    if fmt == "number" || fmt.is_empty() {
        // Java NumberFormat.getNumberInstance(locale) 的 v1 复刻：`#,##0.###`
        // （分组 + 至多 3 位小数；1/2 → "0.5"、123456/7 → "17,636.571"）
        match parse_decimal_format("#,##0.###", locale) {
            Ok(df) => Ok(format_decimal(&df, n)),
            Err(_) => Ok(n.to_plain_string()),
        }
    } else if fmt == "c" || fmt == "computer" {
        Ok(format_c_number(n, CFormatKind::JavaScriptOrJson))
    } else {
        match parse_decimal_format(fmt, locale) {
            Ok(df) => Ok(format_decimal(&df, n)),
            Err(_) => Ok(n.to_plain_string()),
        }
    }
}

/// 自定义格式名解析（Java Environment.java:1637-1641（number）/ :2325-2328（date）：
/// `@` 开头 + 长度>1 + 第二字符为字母 才走自定义格式分支；`@@0` 的 @ 转义、
/// `@0` 的字面量模式、`@` 单独均不匹配 → None）。name 取到首个空格/下划线前
/// （Java findParamsStart :1648-1657）。v1 无自定义格式注册机制（Java
/// isIcI2324OrLater 恒真）→ 匹配即视为未定义。
pub(crate) fn custom_format_name(format_string: &str) -> Option<String> {
    let rest = format_string.strip_prefix('@')?;
    if !rest.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let name = rest
        .split([' ', '_'])
        .next()
        .unwrap_or_default()
        .to_string();
    Some(name)
}

/// Java `StringUtil.jQuote`：双引号包裹 + `\ " \n \r \t` 转义
pub(crate) fn j_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_format_integers_and_decimals() {
        assert_eq!(
            format_c_number(&TNumber::Int(1), CFormatKind::JavaScriptOrJson),
            "1"
        );
        assert_eq!(
            format_c_number(&TNumber::Long(-5), CFormatKind::JavaScriptOrJson),
            "-5"
        );
        assert_eq!(
            format_c_number(&TNumber::from_i64(1234567), CFormatKind::JavaScriptOrJson),
            "1234567"
        );
        assert_eq!(
            format_c_number(
                &TNumber::Decimal(BigDecimal::from_str("1.5").unwrap()),
                CFormatKind::JavaScriptOrJson
            ),
            "1.5"
        );
        assert_eq!(
            format_c_number(
                &TNumber::Decimal(BigDecimal::from_str("1.50").unwrap()),
                CFormatKind::JavaScriptOrJson
            ),
            "1.5"
        );
        // bigDecimal2 = valueOf(1, 16) = 1E-16 → toString → "1E-16"
        assert_eq!(
            format_c_number(
                &TNumber::Decimal(BigDecimal::from_str("1E-16").unwrap()),
                CFormatKind::JavaScriptOrJson
            ),
            "1E-16"
        );
    }

    #[test]
    fn c_format_double_java_style() {
        assert_eq!(
            format_c_number(&TNumber::Double(1e-16), CFormatKind::JavaScriptOrJson),
            "1E-16"
        );
        assert_eq!(
            format_c_number(&TNumber::Double(-1e-16), CFormatKind::JavaScriptOrJson),
            "-1E-16"
        );
        assert_eq!(
            format_c_number(&TNumber::Double(0.05), CFormatKind::JavaScriptOrJson),
            "0.05"
        );
        assert_eq!(
            format_c_number(&TNumber::Double(100000.5), CFormatKind::JavaScriptOrJson),
            "100000.5"
        );
        assert_eq!(
            format_c_number(&TNumber::Double(1.0), CFormatKind::JavaScriptOrJson),
            "1"
        );
        // 整数但超出 2^53 → Double.toString 指数形式（Java CTemplateNumberFormat）
        assert_eq!(
            format_c_number(&TNumber::Double(1e21), CFormatKind::JavaScriptOrJson),
            "1E21"
        );
        assert_eq!(
            format_c_number(
                &TNumber::Double(f64::INFINITY),
                CFormatKind::JavaScriptOrJson
            ),
            "Infinity"
        );
        assert_eq!(
            format_c_number(
                &TNumber::Double(f64::NEG_INFINITY),
                CFormatKind::JavaScriptOrJson
            ),
            "-Infinity"
        );
        assert_eq!(
            format_c_number(&TNumber::Double(f64::NAN), CFormatKind::JavaScriptOrJson),
            "NaN"
        );
    }

    /// c_format 变体（StandardCFormats）：字符串转义 + Infinity/NaN 符号 + XS null
    #[test]
    fn c_format_variants() {
        // 字符串转义：Java（双引号 + \uXXXX）vs JS_OR_JSON（' 不转义）
        assert_eq!(
            format_c_string("a'b\"c", CFormatKind::JavaScriptOrJson),
            "\"a'b\\\"c\""
        );
        // JavaScript 变体（QUOTATION_MARK）：' 不转义（StringUtil :1461-1463），
        // 仅 \x 2 位 hex 与默认不同
        assert_eq!(format_c_string("a'b", CFormatKind::JavaScript), "\"a'b\"");
        assert_eq!(format_c_string("<x>", CFormatKind::Java), "\"<x>\"");
        // XS：原样（假定已有 XML 自动转义）
        assert_eq!(format_c_string("<x>", CFormatKind::Xs), "<x>");
        // Infinity/NaN 符号
        let d = TNumber::Double(f64::INFINITY);
        assert_eq!(
            format_c_number(&d, CFormatKind::JavaScriptOrJson),
            "Infinity"
        );
        assert_eq!(
            format_c_number(&d, CFormatKind::Java),
            "Double.POSITIVE_INFINITY"
        );
        assert_eq!(format_c_number(&d, CFormatKind::Xs), "INF");
        let f = TNumber::Float(f32::NAN);
        assert_eq!(format_c_number(&f, CFormatKind::JavaScriptOrJson), "NaN");
        assert_eq!(format_c_number(&f, CFormatKind::Java), "Float.NaN");
        // 注册名解析
        assert_eq!(CFormatKind::parse("Java"), Some(CFormatKind::Java));
        assert_eq!(CFormatKind::parse("XS"), Some(CFormatKind::Xs));
        assert_eq!(CFormatKind::parse("legacy"), Some(CFormatKind::Legacy));
        assert_eq!(
            CFormatKind::parse("JavaScript or JSON"),
            Some(CFormatKind::JavaScriptOrJson)
        );
        assert_eq!(CFormatKind::parse("bogus"), None);
        assert_eq!(CFormatKind::JavaScriptOrJson.name(), "JavaScript or JSON");
    }

    #[test]
    fn decimal_format_subset() {
        let fmt = parse_decimal_format("0.00", "en_US").unwrap();
        assert_eq!(format_decimal(&fmt, &TNumber::Int(1)), "1.00");
        assert_eq!(
            format_decimal(&fmt, &TNumber::from_i64(1234567)),
            "1234567.00"
        );
        assert_eq!(
            format_decimal(
                &fmt,
                &TNumber::Decimal(BigDecimal::from_str("1234567.886").unwrap())
            ),
            "1234567.89"
        );
        let fmt = parse_decimal_format(",##0.##", "fr_FR").unwrap();
        assert_eq!(format_decimal(&fmt, &TNumber::Int(1)), "1");
        assert_eq!(
            format_decimal(
                &fmt,
                &TNumber::Decimal(BigDecimal::from_str("1234567.886").unwrap())
            ),
            "1\u{202f}234\u{202f}567,89"
        );
        let fmt = parse_decimal_format(",000.##", "fr_FR").unwrap();
        assert_eq!(
            format_decimal(
                &fmt,
                &TNumber::Decimal(BigDecimal::from_str("100000.5").unwrap())
            ),
            "100\u{202f}000,5"
        );
        let fmt = parse_decimal_format("'f'#", "en_US").unwrap();
        assert_eq!(format_decimal(&fmt, &TNumber::Int(1)), "f1");
    }
}
