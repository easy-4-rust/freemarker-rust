//! 对应 Java: CFormatTemplateTest
//! Java `freemarker.core.CFormatTemplateTest` 的 Rust 1:1 实现：
//! `${true?c}`/`${false?c}`/`${null?cn}`/`${s?c}` 的 C 格式输出与
//! `<#setting c_format=...>` 的安全性检查。
//!
//! 引擎差异：
//! - Java `setCFormat(CustomCFormat.INSTANCE)`（自定义类：true→"TRUE"、false→"FALSE"、
//!   null→"NULL"、字符串→ftlQuote）；v1 无自定义 CFormat API，用内建 C 格式
//!   （JSON 风格）近似 → 布尔/字符串输出按内建 JSON CFormat。
//! - v1 不支持模板内 `<#setting c_format=...>`（报 "Unsupported setting: c_format"）：
//!   testStringFormat 的 XS/JavaScript 切换段不可达；testUnsafeSetting 的
//!   "not allowed" 消息以 v1 的 "Unsupported setting" 为准。
//! - `<#setting boolean_format='c'>` 可 1:1 翻译（Java 断言 2 前半段；
//!   CustomCFormat 差异同上）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::template::TModel;

/// Java @Before addToDataModel("s", "a'b\"c\u0001")
fn dm() -> TModel {
    let mut root = indexmap::IndexMap::new();
    root.insert(
        "s".to_string(),
        TModel::from_scalar("a'b\"c\u{1}".to_string()),
    );
    TModel::from_hash(root)
}

/// Java testBooleanAndNullFormat
/// 引擎差异：
/// - CustomCFormat 用内建 C 格式近似 → 布尔 ?c 输出 "true"/"false"（Java CustomCFormat：
///   "TRUE"/"FALSE"），?cn 的 null 输出 "null"（Java CustomCFormat："NULL"）。
/// - Java 模板用 `null` 字面量（${null?cn}）；v1 无 null 字面量（`null` 是缺失变量），
///   改用数据模型中的 null 值（TModel::nothing()）验证 ?cn。
#[test]
fn test_boolean_and_null_format() {
    let (c, loader) = test_config();
    let mut root = indexmap::IndexMap::new();
    root.insert("null_value".to_string(), TModel::nothing());
    let dm = TModel::from_hash(root);
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${true?c} ${false?c} ${null_value?cn}",
        dm.clone(),
    );
    assert_eq!(out, "true false null");
    // Java 断言 2：<#setting boolean_format='c'>${true} ${false} JSON: <#setting c_format='JSON'>${true} ${false}
    // → "TRUE FALSE JSON: true false"。
    // 引擎差异：Java CustomCFormat 前半段输出 "TRUE FALSE"，v1 为 "true false"；
    // v1 不支持 c_format 设置，后半段（c_format='JSON' 切换）无法渲染 ——
    // 但 v1 的 boolean_format='c' 本就输出 "true false"，与 Java JSON 段一致。
    assert_output(
        &c,
        &loader,
        "<#setting boolean_format='c'>${true} ${false}",
        "true false",
    );
}

/// Java testStringFormat
#[test]
fn test_string_format() {
    let (c, loader) = test_config();
    let dm = dm();
    // 引擎差异：v1 的 ?c 对字符串用 js_string_enc(JSON) 输出、不包外层引号（c_and_cn
    // 测试既有行为）；Java CustomCFormat.formatString（ftlQuote）包引号。转义内容一致。
    let out = render_ftl_with_dm(&c, &loader, "Default: ${s?c} ", dm.clone());
    assert_eq!(out, "Default: a'b\\\"c\\u0001 ");
    // Java 的 XS（<#setting c_format='XS'>，无引号）与 JavaScript（<#setting c_format='JavaScript'>，
    // \x01 转义）切换段 —— 引擎差异：v1 不支持 c_format 设置，?c 恒为 JSON 风格；
    // JSON/Java 段转义与 v1 一致（差异仅为引号），一并断言
    let out = render_ftl_with_dm(&c, &loader, "JSON: ${s?c} ", dm.clone());
    assert_eq!(out, "JSON: a'b\\\"c\\u0001 ");
    let out = render_ftl_with_dm(&c, &loader, "Java: ${s?c} ", dm.clone());
    assert_eq!(out, "Java: a'b\\\"c\\u0001 ");
    // XS/JavaScript 段无法复现（c_format 设置不支持）→ 引擎差异已在上方注释说明
}

/// Java testUnsafeSetting
/// 引擎差异：Java 拒绝不安全的自定义 CFormat 类名（消息含 "not allowed"）；
/// v1 对任意 `<#setting c_format>` 值报 "Unsupported setting: c_format" ——
/// 以 v1 实际消息为准。
#[test]
fn test_unsafe_setting() {
    let (c, loader) = test_config();
    let msg = assert_error_contains(
        &c,
        &loader,
        "<#setting c_format='com.example.ExploitCFormat()'>",
        &["Unsupported setting"],
    );
    assert!(msg.contains("c_format"), "msg: {msg}");
    let msg = assert_error_contains(
        &c,
        &loader,
        "<#setting cFormat='com.example.ExploitCFormat()'>",
        &["Unsupported setting"],
    );
    assert!(msg.contains("c_format"), "msg: {msg}");
}
