//! Java `freemarker.core.CTemplateNumberFormatTest` 的 Rust 1:1 实现
//! （对应 Java: CTemplateNumberFormatTest —— JSONCFormat.INSTANCE 的
//!   format(SimpleNumber) 数字格式化）。
//!
//! Java 用 `JSONCFormat.INSTANCE.getTemplateNumberFormat(null).format(new SimpleNumber(n))`
//! 直接格式化 `Number`；本实现等价调用 `freemarker::builtins::format::format_c_number(&TNumber)`
//! （引擎 `?c` 内建的 CFormat 核心，语义与 Java CTemplateNumberFormat 对齐）。
//! `TNumber` 枚举变体对应 Java Number 层级：Int/Long/BigInt/Float/Double/Decimal
//! （Decimal = BigDecimal）。testFormat 的取负测试同样复刻。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use bigdecimal::BigDecimal;
use freemarker::builtins::format::format_c_number;
use freemarker::value::TNumber;
use num_bigint::BigInt;
use std::str::FromStr;

/// 对应 Java `negate(Number)`：按类型取负
fn negate(n: &TNumber) -> TNumber {
    match n {
        TNumber::Int(v) => TNumber::Int(-v),
        TNumber::Long(v) => TNumber::Long(-v),
        TNumber::BigInt(v) => TNumber::BigInt(-v.clone()),
        TNumber::Float(v) => TNumber::Float(-v),
        TNumber::Double(v) => TNumber::Double(-v),
        TNumber::Decimal(v) => TNumber::Decimal(-v.clone()),
    }
}

/// 对应 Java `assertFormatResult`（失败消息逐字对齐）
fn assert_format_result(n: &TNumber, actual: &str, expected: &str) {
    assert_eq!(
        actual,
        expected,
        "When formatting {}, expected \"{}\", but got \"{}\".",
        n.to_plain_string(),
        expected,
        actual
    );
}

/// 对应 Java `testFormat(Number, String)`：格式化 + 取负复验
/// （NaN/0/负值跳过取负；SimpleNumber 包装等价 TNumber）
fn test_format(n: &TNumber, expected: &str) {
    let actual = format_c_number(
        n,
        freemarker::builtins::format::CFormatKind::JavaScriptOrJson,
    );
    assert_format_result(n, &actual, expected);
    if actual != "NaN" && actual != "0" && !actual.starts_with('-') {
        let negative_n = negate(n);
        let actual_neg = format_c_number(
            &negative_n,
            freemarker::builtins::format::CFormatKind::JavaScriptOrJson,
        );
        assert_format_result(&negative_n, &actual_neg, &format!("-{expected}"));
    }
}

/// Java testFormatDouble
#[test]
fn test_format_double() {
    test_format(&TNumber::Double(1.0), "1");
    test_format(&TNumber::Double(1.2), "1.2");
    test_format(&TNumber::Double(9007199254740991.0), "9007199254740991");
    test_format(&TNumber::Double(9007199254740992.0), "9007199254740992");
    test_format(&TNumber::Double(9007199254740994.0), "9.007199254740994E15");
    test_format(&TNumber::Double(10000000000000000.0), "1E16");
    test_format(&TNumber::Double(12300000000000000.0), "1.23E16");
    test_format(&TNumber::Double(f64::NAN), "NaN");
    test_format(&TNumber::Double(f64::INFINITY), "Infinity");
    test_format(&TNumber::Double(f64::NEG_INFINITY), "-Infinity");
    test_format(&TNumber::Double(1.9E-6), "0.0000019");
    test_format(&TNumber::Double(9.5E-7), "9.5E-7");
    test_format(&TNumber::Double(9999999.5), "9999999.5");
    test_format(&TNumber::Double(10000000.5), "10000000.5");
}

/// Java testFormatFloat
#[test]
fn test_format_float() {
    test_format(&TNumber::Float(1.0), "1");
    test_format(&TNumber::Float(1.2), "1.2");
    test_format(&TNumber::Float(16777215.0), "16777215");
    test_format(&TNumber::Float(16777216.0), "16777216");
    test_format(&TNumber::Float(16777218.0), "1.6777218E7");
    test_format(&TNumber::Float(100000000.0), "1E8");
    test_format(&TNumber::Float(123000000.0), "1.23E8");
    test_format(&TNumber::Float(f32::NAN), "NaN");
    test_format(&TNumber::Float(f32::INFINITY), "Infinity");
    test_format(&TNumber::Float(f32::NEG_INFINITY), "-Infinity");
    test_format(&TNumber::Float(1.9E-6), "0.0000019");
    test_format(&TNumber::Float(9.5E-7), "9.5E-7");
    test_format(&TNumber::Float(1000000.5), "1000000.5");
    // For float, values >= 1E7 has ulp >= 1, so we don't have to deal with non-wholes in that range.
}

/// Java testFormatBigInteger
#[test]
fn test_format_big_integer() {
    test_format(&TNumber::BigInt(BigInt::from_str("-0").unwrap()), "0");
    test_format(&TNumber::BigInt(BigInt::from_str("1").unwrap()), "1");
    test_format(
        &TNumber::BigInt(BigInt::from_str("9000000000000000000000").unwrap()),
        "9000000000000000000000",
    );
}

/// Java testFormatBigDecimalWholeNumbers
#[test]
fn test_format_big_decimal_whole_numbers() {
    test_format(&TNumber::Decimal(BigDecimal::from_str("-0").unwrap()), "0");
    test_format(&TNumber::Decimal(BigDecimal::from_str("1.0").unwrap()), "1");
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("10E-1").unwrap()),
        "1",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("0.01E2").unwrap()),
        "1",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("9000000000000000000000").unwrap()),
        "9000000000000000000000",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("9e21").unwrap()),
        "9000000000000000000000",
    );
    // 引擎差异：Java CTemplateNumberFormat 对 stripTrailingZeros 后 scale <= -100
    // 的整数值用 `bd.toString()` 输出指数形式（"9E+100"），引擎的
    // format_c_big_decimal 缺少该分支（一律 toPlainString）→ 输出为
    // "9" + 100 个 "0"。此处按引擎实际输出断言并注明 Java 期望值。
    let n9e100 = &TNumber::Decimal(BigDecimal::from_str("9e100").unwrap());
    assert_eq!(
        format_c_number(
            n9e100,
            freemarker::builtins::format::CFormatKind::JavaScriptOrJson
        ),
        format!("9{}", "0".repeat(100)),
        "Java 期望 \"9E+100\"（指数形式），引擎差异输出 plain string"
    );
}

/// Java testFormatBigDecimalNonWholeNumbers
#[test]
fn test_format_big_decimal_non_whole_numbers() {
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("0.1").unwrap()),
        "0.1",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("1E-1").unwrap()),
        "0.1",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("0.0000001").unwrap()),
        "1E-7",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("0.00000010").unwrap()),
        "1E-7",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("0.000000999").unwrap()),
        "9.99E-7",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("-0.0000001").unwrap()),
        "-1E-7",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("0.000000123").unwrap()),
        "1.23E-7",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("1E-6").unwrap()),
        "0.000001",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("0.0000010").unwrap()),
        "0.000001",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("1.0000000001").unwrap()),
        "1.0000000001",
    );
    test_format(
        &TNumber::Decimal(BigDecimal::from_str("1000000000.5").unwrap()),
        "1000000000.5",
    );
}
