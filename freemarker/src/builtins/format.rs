//! 数字格式化 —— 对应 Java `freemarker.core.CTemplateNumberFormat`（?c/?cn 的 C 格式，
//! IcI ≥ 2.3.32 默认 `JavaScriptOrJSONCFormat`）+ `JavaTemplateNumberFormat`（?string(pattern)
//! 的 DecimalFormat 子集）。文档见 docs/08 §2。
//!
//! v1 范围：
//! - C 格式（"c"/"computer"/?c/?cn）：整数 plain、BigDecimal stripTrailingZeros、
//!   Double/Float 用 Java 最短表示（指数 E 大写、范围对齐 Double.toString）；
//! - DecimalFormat 子集：`0`/`#`/`.`/`,`（分组）/`'...'`（引号字面量），HALF_EVEN 舍入，
//!   locale 相关小数点与分组符（en/fr/de/es/tr）；完整模式（E、%、‰、货币等）属 P4。

use crate::core::Environment;
use crate::error::{Result, TemplateError};
use crate::value::TNumber;
use bigdecimal::{BigDecimal, RoundingMode};
use std::str::FromStr;

/// ?c/?cn 与 number_format="c"/"computer" 的 C 格式输出
/// （Java CTemplateNumberFormat.formatToPlainText；JavaScriptOrJSONCFormat 的
/// "Infinity"/"NaN" 符号，对应 JavaC20 的 Double.toString 语义）
pub fn format_c_number(n: &TNumber) -> String {
    match n {
        TNumber::Int(v) => v.to_string(),
        TNumber::Long(v) => v.to_string(),
        TNumber::BigInt(v) => v.to_string(),
        TNumber::Double(v) => format_c_double(*v),
        TNumber::Float(v) => format_c_float(*v),
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

fn format_c_double(n: f64) -> String {
    if n == f64::INFINITY {
        return "Infinity".to_string();
    }
    if n == f64::NEG_INFINITY {
        return "-Infinity".to_string();
    }
    if n.is_nan() {
        return "NaN".to_string();
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

fn format_c_float(n: f32) -> String {
    if n == f32::INFINITY {
        return "Infinity".to_string();
    }
    if n == f32::NEG_INFINITY {
        return "-Infinity".to_string();
    }
    if n.is_nan() {
        return "NaN".to_string();
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
            let s = java_float_string(n as f64);
            return BigDecimal::from_str(&s).map(|b| b.to_string()).unwrap_or(s);
        }
        // float 无 absN >= 1E7 分支（Java 注释：那些数字对 float 而言都是整数）
    }
    remove_redundant_dot0(&java_float_string(n as f64))
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
    let s = format!("{:e}", n);
    let (mant, exp) = s.split_once('e').expect("Rust {:e} always has exponent");
    let exp: i32 = exp.parse().expect("exponent parses");
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let int_digits = mant.split_once('.').map_or(mant.len(), |(i, _)| i.len());
    // Java：exponent < -3 或 >= 7 → 科学计数
    if !(-3..7).contains(&exp) {
        // 去掉整数的 ".0"（"1.0E-16" → "1E-16"）：Java Double.toString 对整 mantissa 输出 "1.0"，
        // removeRedundantDot0 后续统一处理；这里保持与 Java toString 一致（含 .0）
        let m = if int_digits == digits.len() {
            format!("{}.0", &digits[..int_digits])
        } else {
            format!("{}.{}", &digits[..int_digits], &digits[int_digits..])
        };
        format!("{}E{}", m, exp)
    } else if exp < 0 {
        // 0.00...ddd
        let zeros = "0".repeat((-(exp + 1)) as usize);
        format!("0.{}{}", zeros, digits)
    } else {
        let point = (exp + 1) as usize;
        if point >= digits.len() {
            format!("{}{}.0", digits, "0".repeat(point - digits.len()))
        } else {
            format!("{}.{}", &digits[..point], &digits[point..])
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
pub fn format_number(env: &Environment, n: &TNumber) -> String {
    let fmt = env.settings.number_format.as_str();
    let locale = env.settings.locale.as_str();
    format_number_with(fmt, locale, n)
}

/// 与 format_number 相同，但显式指定格式串（?string('pattern') 用）
pub fn format_number_with(fmt: &str, locale: &str, n: &TNumber) -> String {
    if fmt == "number" || fmt.is_empty() {
        // Java NumberFormat.getNumberInstance(locale) 的 v1 复刻：`#,##0.###`
        // （分组 + 至多 3 位小数；1/2 → "0.5"、123456/7 → "17,636.571"）
        match parse_decimal_format("#,##0.###", locale) {
            Ok(df) => format_decimal(&df, n),
            Err(_) => n.to_plain_string(),
        }
    } else if fmt == "c" || fmt == "computer" {
        format_c_number(n)
    } else {
        match parse_decimal_format(fmt, locale) {
            Ok(df) => format_decimal(&df, n),
            Err(_) => n.to_plain_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_format_integers_and_decimals() {
        assert_eq!(format_c_number(&TNumber::Int(1)), "1");
        assert_eq!(format_c_number(&TNumber::Long(-5)), "-5");
        assert_eq!(format_c_number(&TNumber::from_i64(1234567)), "1234567");
        assert_eq!(
            format_c_number(&TNumber::Decimal(BigDecimal::from_str("1.5").unwrap())),
            "1.5"
        );
        assert_eq!(
            format_c_number(&TNumber::Decimal(BigDecimal::from_str("1.50").unwrap())),
            "1.5"
        );
        // bigDecimal2 = valueOf(1, 16) = 1E-16 → toString → "1E-16"
        assert_eq!(
            format_c_number(&TNumber::Decimal(BigDecimal::from_str("1E-16").unwrap())),
            "1E-16"
        );
    }

    #[test]
    fn c_format_double_java_style() {
        assert_eq!(format_c_number(&TNumber::Double(1e-16)), "1E-16");
        assert_eq!(format_c_number(&TNumber::Double(-1e-16)), "-1E-16");
        assert_eq!(format_c_number(&TNumber::Double(0.05)), "0.05");
        assert_eq!(format_c_number(&TNumber::Double(100000.5)), "100000.5");
        assert_eq!(format_c_number(&TNumber::Double(1.0)), "1");
        // 整数但超出 2^53 → Double.toString 指数形式（Java CTemplateNumberFormat）
        assert_eq!(format_c_number(&TNumber::Double(1e21)), "1E21");
        assert_eq!(format_c_number(&TNumber::Double(f64::INFINITY)), "Infinity");
        assert_eq!(
            format_c_number(&TNumber::Double(f64::NEG_INFINITY)),
            "-Infinity"
        );
        assert_eq!(format_c_number(&TNumber::Double(f64::NAN)), "NaN");
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
