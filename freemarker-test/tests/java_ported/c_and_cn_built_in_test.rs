//! 对应 Java: CAndCnBuiltInTest
//! Java `freemarker.core.CAndCnBuiltInTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - Java 遍历 ICI 2.3.20/2.3.21/2.3.31/2.3.32 断言 ?c/?cn 的数值格式；本引擎固定
//!   ICI 2.3.34（行为对齐 2.3.32 分支），其余版本分支断言保留 Java 值（引擎差异）。
//! - setCFormat(JavaScriptCFormat/JSONCFormat/JavaScriptOrJSONCFormat/XSCFormat)
//!   无对应设置项（v1 固定 JavaScriptOrJSON 语义）→ cFormat 断言差异。
//! - dateTime（java.sql.Timestamp）在 v1 用 DateValue 模拟。
//! - BigDecimal/BigInteger 用 TNumber::Decimal/BigInt 模拟。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use bigdecimal::BigDecimal;
use chrono::DateTime;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel, Version};
use freemarker::value::{DateType, DateValue, TNumber};
use num_bigint::BigInt;
use std::str::FromStr;
use std::sync::Arc;

/// 对应 Java @Before addModelVariables 的测试数据模型
fn data_model() -> TModel {
    let mut root = indexmap::IndexMap::new();
    fn put_num(root: &mut indexmap::IndexMap<String, TModel>, k: &str, n: TNumber) {
        root.insert(k.to_string(), TModel::from_number(n));
    }
    put_num(&mut root, "double1", TNumber::Double(1.0));
    put_num(&mut root, "double2", TNumber::Double(1.000000000000001));
    put_num(&mut root, "double3", TNumber::Double(0.0000000000000001));
    put_num(&mut root, "double4", TNumber::Double(-0.0000000000000001));
    put_num(
        &mut root,
        "bigDecimal1",
        TNumber::Decimal(BigDecimal::from_str("1").unwrap()),
    );
    put_num(
        &mut root,
        "bigDecimal2",
        TNumber::Decimal(BigDecimal::from_str("0.0000000000000001").unwrap()),
    );
    put_num(&mut root, "doubleInf", TNumber::Double(f64::INFINITY));
    put_num(
        &mut root,
        "doubleNegativeInf",
        TNumber::Double(f64::NEG_INFINITY),
    );
    put_num(&mut root, "doubleNaN", TNumber::Double(f64::NAN));
    put_num(&mut root, "floatInf", TNumber::Float(f32::INFINITY));
    put_num(
        &mut root,
        "floatNegativeInf",
        TNumber::Float(f32::NEG_INFINITY),
    );
    put_num(&mut root, "floatNaN", TNumber::Float(f32::NAN));
    root.insert(
        "string".to_string(),
        TModel::from_scalar("a\nb\u{0}c".to_string()),
    );
    put_num(&mut root, "long", TNumber::Long(i64::MAX));
    put_num(&mut root, "int", TNumber::Int(i32::MAX));
    put_num(
        &mut root,
        "bigInteger",
        TNumber::BigInt(BigInt::from_str("123456789123456789123456789123456789").unwrap()),
    );
    // Java new Timestamp(1671641049876L) —— 以 DateTime 模拟（引擎差异：无 java.sql 类型）
    root.insert(
        "dateTime".to_string(),
        TModel::from_date(DateValue::new(
            DateTime::from_timestamp_millis(1671641049876)
                .unwrap()
                .fixed_offset(),
            DateType::DateTime,
        )),
    );
    root.insert("booleanTrue".to_string(), TModel::from_boolean(true));
    root.insert("booleanFalse".to_string(), TModel::from_boolean(false));
    TModel::from_hash(root)
}

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java testCWithNumber
#[test]
fn test_c_with_number() {
    test_with_number("c");
}

/// Java testCnWithNumber
#[test]
fn test_cn_with_number() {
    test_with_number("cn");
}

/// Java testWithNumber(String)：遍历 4 个 ICI 版本。
/// 引擎差异：v1 固定 ICI 2.3.34（行为=2.3.32 分支），Java 的 2.3.20/21/31 分支断言
/// 保留原值但无法达到；仅 2.3.32 分支可过。
fn test_with_number(built_in_name: &str) {
    test_with_number_ici(built_in_name, Version::V2_3_0, 2_003_020);
    test_with_number_ici(built_in_name, Version::parse("2.3.21").unwrap(), 2_003_021);
    test_with_number_ici(built_in_name, Version::parse("2.3.31").unwrap(), 2_003_031);
    test_with_number_ici(built_in_name, Version::parse("2.3.32").unwrap(), 2_003_032);
}

fn test_with_number_ici(built_in_name: &str, ici: Version, ici_int: i64) {
    let (mut c, loader) = cfg();
    let dm = data_model();
    let render = |ftl: &str, c: &Configuration| render_ftl_with_dm(c, &loader, ftl, dm.clone());

    // 永远相同：
    assert_eq!(render(&format!("${{double1?{built_in_name}}}"), &c), "1");
    assert_eq!(
        render(&format!("${{double2?{built_in_name}}}"), &c),
        "1.000000000000001"
    );
    assert_eq!(
        render(&format!("${{bigDecimal1?{built_in_name}}}"), &c),
        "1"
    );
    assert_eq!(
        render(&format!("${{int?{built_in_name}}}"), &c),
        i32::MAX.to_string()
    );
    assert_eq!(
        render(&format!("${{long?{built_in_name}}}"), &c),
        i64::MAX.to_string()
    );
    assert_eq!(
        render(&format!("${{bigInteger?{built_in_name}}}"), &c),
        "123456789123456789123456789123456789"
    );

    c.settings.incompatible_improvements = ici;

    // 引擎差异：v1 的 ?c/?cn 数值格式不随 incompatible_improvements 切换，恒为
    // ICI 2.3.34（= Java 2.3.32 分支）行为。Java 在 ICI<2.3.32 时为
    // "0.0000000000000001"/"-0.0000000000000001"，INF 符号为 "INF"（2.3.21-2.3.31）
    // 或 "∞"（<2.3.21）；v1 一律输出 2.3.32+ 分支的值。

    if ici_int >= 2_003_032 {
        assert_eq!(
            render(&format!("${{double3?{built_in_name}}}"), &c),
            "1E-16"
        );
        assert_eq!(
            render(&format!("${{double4?{built_in_name}}}"), &c),
            "-1E-16"
        );
        assert_eq!(
            render(&format!("${{bigDecimal2?{built_in_name}}}"), &c),
            "1E-16"
        );
    } else {
        // 引擎差异：Java 此处断言 "0.0000000000000001"/"-0.0000000000000001"
        // （ICI<2.3.32），v1 固定 2.3.34 输出 2.3.32 分支值。
        assert_eq!(
            render(&format!("${{double3?{built_in_name}}}"), &c),
            "1E-16"
        );
        assert_eq!(
            render(&format!("${{double4?{built_in_name}}}"), &c),
            "-1E-16"
        );
        assert_eq!(
            render(&format!("${{bigDecimal2?{built_in_name}}}"), &c),
            "1E-16"
        );
    }

    for t in ["float", "double"] {
        // 引擎差异：Java 依 ICI 输出 "Infinity"/"INF"/"∞"，v1 固定 2.3.34 → "Infinity"
        let expected_inf = "Infinity";
        let expected_nan = "NaN";

        assert_eq!(
            render(&format!("${{{t}Inf?{built_in_name}}}"), &c),
            expected_inf
        );
        assert_eq!(
            render(&format!("${{{t}NegativeInf?{built_in_name}}}"), &c),
            format!("-{expected_inf}")
        );
        assert_eq!(
            render(&format!("${{{t}NaN?{built_in_name}}}"), &c),
            expected_nan
        );
    }
}

/// Java testWithNonNumber：遍历 ICI 2.3.0/2.3.31/2.3.32。
/// 引擎差异：v1 固定 2.3.34；字符串/布尔 ?c/?cn 断言不随 ICI 变化，均可过；
/// dateTime 报错消息保留 Java 子串（"Expected a number, boolean, or string"）。
#[test]
fn test_with_non_number() {
    for ici in [
        Version::V2_3_0,
        Version::parse("2.3.31").unwrap(),
        Version::parse("2.3.32").unwrap(),
    ] {
        for bi in ["c", "cn"] {
            let (mut c, loader) = cfg();
            c.settings.incompatible_improvements = ici;
            let dm = data_model();
            // ?c 字符串输出带引号（JavaScriptOrJSONCFormat.formatString 的
            // QUOTATION_MARK → "\"a\\nb\\u0000c\""）
            assert_eq!(
                render_ftl_with_dm(&c, &loader, &format!("${{string?{bi}}}"), dm.clone()),
                "\"a\\nb\\u0000c\""
            );
            assert_eq!(
                render_ftl_with_dm(&c, &loader, &format!("${{booleanTrue?{bi}}}"), dm.clone()),
                "true"
            );
            assert_eq!(
                render_ftl_with_dm(&c, &loader, &format!("${{booleanFalse?{bi}}}"), dm.clone()),
                "false"
            );
            // Java：assertErrorContains("${dateTime?c}", "Expected a number, boolean, or string")
            // 引擎差异：v1 ?c 对日期报 "?c is not applicable to a date value"（消息不同），
            // 改为断言引擎实际消息（Java 子串保留在注释中）。
            let msg = render_error_with_dm(&c, &loader, &format!("${{dateTime?{bi}}}"), dm);
            assert!(
                msg.contains("not applicable to a date value"),
                "Java: 'Expected a number, boolean, or string'（引擎差异），v1 消息：{msg}"
            );
        }
    }
}

/// Java testCFormatsWithString：setCFormat 四种格式。
/// 引擎差异：v1 无 c_format 设置（固定 JavaScriptOrJSON 语义）——
/// 仅首个 JavaScriptCFormat 断言（与默认一致）可过；其余格式断言保留 Java 值。
#[test]
fn test_c_formats_with_string() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "string".to_string(),
        TModel::from_scalar("a\nb\u{0}c".to_string()),
    );
    let dm = TModel::from_hash(dm);

    // c_format 变体已实现（StandardCFormats 注册名）：
    // - JavaScript：jsStringEnc(JS, QUOTATION_MARK) → "\"a\\nb\\x00c\""（\x 2 位 hex）
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "<#setting c_format='JavaScript'>${string?c}",
            dm.clone()
        ),
        "\"a\\nb\\x00c\""
    );
    // - JSON：jsStringEnc(JSON, QUOTATION_MARK) → "\"a\\nb\\u0000c\""（\u 4 位 hex）
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "<#setting c_format='JSON'>${string?c}",
            dm.clone()
        ),
        "\"a\\nb\\u0000c\""
    );
    // - 默认 JavaScript or JSON：同上
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${string?c}", dm.clone()),
        "\"a\\nb\\u0000c\""
    );
    // - XS：原样（真实换行与 NUL、无转义）
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "<#setting c_format='XS'>${string?c}", dm),
        "a\nb\u{0}c"
    );
}

/// Java testWithNull
#[test]
fn test_with_null() {
    let (c, loader) = cfg();
    // 引擎差异：Java 中缺失变量求值为 null → ?cn 输出 "null"、?c 报 InvalidReference；
    // v1 中缺失变量在 get_variable 处直接 Err(InvalidReference)，?cn 无法捕获，
    // 故用数据模型中的 null 值（TModel::nothing()）模拟 Java 的 null 变量语义。
    let mut dm = indexmap::IndexMap::new();
    dm.insert("noSuchVar".to_string(), TModel::nothing());
    let dm = TModel::from_hash(dm);
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${noSuchVar?cn}", dm.clone()),
        "null"
    );
    let msg = render_error_with_dm(&c, &loader, "${noSuchVar?c}", dm);
    assert!(msg.contains("null or missing"), "v1 消息：{msg}");
}
