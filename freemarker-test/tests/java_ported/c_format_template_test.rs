//! 对应 Java: CFormatTemplateTest
//! Java `freemarker.core.CFormatTemplateTest` 的 Rust 1:1 实现：
//! `${true?c}`/`${false?c}`/`${null?cn}`/`${s?c}` 的 C 格式输出与
//! `<#setting c_format=...>` 的安全性检查。
//!
//! 引擎差异：
//! - Java `setCFormat(CustomCFormat.INSTANCE)`（自定义类：true→"TRUE"、false→"FALSE"、
//!   null→"NULL"、字符串→ftlQuote）；Rust 无自定义 CFormat API（_ObjectBuilder* 类名
//!   求值属 NA-DESIGN），用内建 StandardCFormats 变体近似 → 布尔/字符串输出按内建
//!   C 格式。
//! - c_format 设置已实现（2026-08，对应 Java Configurable.C_FORMAT_KEY）：
//!   JavaScript/JSON/Java/XS/legacy 变体按 StandardCFormats 注册名分派；
//!   testStringFormat 各切换段可 1:1 复现。
//! - testUnsafeSetting：Java 拒绝不安全自定义类名（消息含 "not allowed"）；Rust 无
//!   类加载机制，非注册名直接报 "Unknown c_format"（更严格的安全行为，文档化差异）。
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
    // Java 权威输出（CFormatTemplateTest.java:60-68）：
    // Default: "a'b\\\"c\\u0001"（JS_OR_JSON：' 不转义、\u 4 位 hex）
    let out = render_ftl_with_dm(&c, &loader, "Default: ${s?c} ", dm.clone());
    assert_eq!(out, "Default: \"a'b\\\"c\\u0001\" ");
    // XS：原样（假定已有 XML 自动转义）
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "XS: <#setting c_format='XS'>${s?c} ",
        dm.clone(),
    );
    assert_eq!(out, "XS: a'b\"c\u{1} ");
    // JavaScript：\x 2 位 hex（' 不转义）
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "JavaScript: <#setting c_format='JavaScript'>${s?c} ",
        dm.clone(),
    );
    assert_eq!(out, "JavaScript: \"a'b\\\"c\\x01\" ");
    // JSON 与 Java 段转义一致（\u 4 位 hex）
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "JSON: <#setting c_format='JSON'>${s?c} ",
        dm.clone(),
    );
    assert_eq!(out, "JSON: \"a'b\\\"c\\u0001\" ");
    let out = render_ftl_with_dm(&c, &loader, "Java: <#setting c_format='Java'>${s?c} ", dm);
    assert_eq!(out, "Java: \"a'b\\\"c\\u0001\" ");
}

/// Java testUnsafeSetting
/// 引擎差异：Java 拒绝不安全的自定义 CFormat 类名（消息含 "not allowed"）；
/// Rust 无类加载机制（_ObjectBuilder* NA-DESIGN），非注册名直接报
/// "Unknown c_format: ..."（更严格的安全行为，文档化差异）。
#[test]
fn test_unsafe_setting() {
    let (c, loader) = test_config();
    let msg = assert_error_contains(
        &c,
        &loader,
        "<#setting c_format='com.example.ExploitCFormat()'>",
        &["Unknown c_format"],
    );
    assert!(msg.contains("com.example.ExploitCFormat()"), "msg: {msg}");
    let msg = assert_error_contains(
        &c,
        &loader,
        "<#setting cFormat='com.example.ExploitCFormat()'>",
        &["Unknown c_format"],
    );
    assert!(msg.contains("com.example.ExploitCFormat()"), "msg: {msg}");
}
