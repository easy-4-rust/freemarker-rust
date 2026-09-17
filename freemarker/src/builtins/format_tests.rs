//! 测试 —— 自 format.rs 拆出（#[cfg(test)] 模块；由主文件
//! #[cfg(test)] #[path] 声明，仅测试构建时编译）。

use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Java 实测基线（freemarker 2.3.29 jar，2026-08-16）：
    /// `?string.currency` / `?string.percent` 为预定义格式名，
    /// `?string.integer` 非预定义 → DecimalFormat 模式字面量（integer1235）。
    #[test]
    fn currency_percent_java_baseline() {
        let n = TNumber::Double(1234.56);
        // currency（符号位置/小数位随 locale）
        assert_eq!(
            format_number_with("currency", "en_US", &n).unwrap(),
            "$1,234.56"
        );
        assert_eq!(
            format_number_with("currency", "de_DE", &n).unwrap(),
            "1.234,56 €"
        );
        assert_eq!(
            format_number_with("currency", "zh_CN", &n).unwrap(),
            "¥1,234.56"
        );
        assert_eq!(
            format_number_with("currency", "ja_JP", &n).unwrap(),
            "￥1,235"
        );
        assert_eq!(
            format_number_with("currency", "fr_FR", &n).unwrap(),
            "1\u{202F}234,56 €"
        );
        // percent（值×100、0 位小数；de/fr 数值与 % 间空格）
        assert_eq!(
            format_number_with("percent", "en_US", &n).unwrap(),
            "123,456%"
        );
        assert_eq!(
            format_number_with("percent", "de_DE", &n).unwrap(),
            "123.456 %"
        );
        assert_eq!(
            format_number_with("percent", "fr_FR", &n).unwrap(),
            "123\u{202F}456 %"
        );
        assert_eq!(
            format_number_with("percent", "zh_CN", &n).unwrap(),
            "123,456%"
        );
        // 非预定义名 → 模式字面量（与 Java 一致）
        assert_eq!(
            format_number_with("integer", "en_US", &n).unwrap(),
            "integer1235"
        );
    }

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
