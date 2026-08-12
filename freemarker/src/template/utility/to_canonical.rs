//! 规范化输出 —— 对应 Java `freemarker.template.utility.ToCanonical`
//! （`?c` 内建的变换模型实现；v1 的 C 格式化在 builtins/format.rs
//! format_c_number/format_c_string——本类型为 Java 对应物）

use crate::builtins::format::{format_c_number, CFormatKind};

/// 规范化输出（对应 ToCanonical.java；`?c` 语义）
pub struct ToCanonical;

impl ToCanonical {
    /// 数字 → C 字面量（Java `format` 的 Rust 等价）
    pub fn format_number(n: &crate::value::TNumber) -> String {
        format_c_number(n, CFormatKind::JavaScriptOrJson)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::TNumber;
    use bigdecimal::BigDecimal;
    use num_bigint::BigInt;
    use std::str::FromStr;

    /// Java ToCanonical.format(Number)：整型按 Long.toString
    #[test]
    fn format_number_integers() {
        assert_eq!(ToCanonical::format_number(&TNumber::Int(42)), "42");
        assert_eq!(ToCanonical::format_number(&TNumber::Int(-7)), "-7");
        assert_eq!(
            ToCanonical::format_number(&TNumber::Long(i64::MAX)),
            "9223372036854775807"
        );
        assert_eq!(
            ToCanonical::format_number(&TNumber::BigInt(
                BigInt::parse_bytes(b"123456789012345678901234567890", 10).unwrap()
            )),
            "123456789012345678901234567890"
        );
    }

    /// Java ToCanonical.format：整值 double 输出为整数形式（Java
    /// 2.3.34 的 format 走 Double 分支：40.0 → "40"）
    #[test]
    fn format_number_integral_doubles() {
        assert_eq!(ToCanonical::format_number(&TNumber::Double(40.0)), "40");
        assert_eq!(ToCanonical::format_number(&TNumber::Float(1.5)), "1.5");
        assert_eq!(ToCanonical::format_number(&TNumber::Double(3.5)), "3.5");
    }

    /// Java 2.3.34 的 ToCanonical.format：大数避免指数形式
    #[test]
    fn format_number_large_double_no_exponent() {
        assert_eq!(
            ToCanonical::format_number(&TNumber::Double(1e8)),
            "100000000"
        );
    }

    /// Java ToCanonical.format：Infinity/NaN 符号（JavaScriptOrJson 变体）
    #[test]
    fn format_number_special_doubles() {
        assert_eq!(
            ToCanonical::format_number(&TNumber::Double(f64::INFINITY)),
            "Infinity"
        );
        assert_eq!(
            ToCanonical::format_number(&TNumber::Double(f64::NEG_INFINITY)),
            "-Infinity"
        );
        assert_eq!(
            ToCanonical::format_number(&TNumber::Double(f64::NAN)),
            "NaN"
        );
    }

    /// Java BigDecimal.stripTrailingZeros：1.50 → 1.5
    #[test]
    fn format_number_big_decimal_strips_trailing_zeros() {
        assert_eq!(
            ToCanonical::format_number(&TNumber::Decimal(BigDecimal::from_str("1.50").unwrap())),
            "1.5"
        );
        assert_eq!(
            ToCanonical::format_number(&TNumber::Decimal(BigDecimal::from_str("100.000").unwrap())),
            "100"
        );
    }
}
