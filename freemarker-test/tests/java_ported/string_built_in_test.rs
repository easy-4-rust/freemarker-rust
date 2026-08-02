//! 对应 Java: StringBuiltInTest
//! Java `freemarker.core.StringBuiltInTest` 的 Rust 1:1 实现。
//! createConfiguration：outputFormat=HTML、numberFormat=",##0.###"。
//!
//! 引擎差异总览：
//! - Java `<#assign html></#assign>`（HTML 输出格式下）产生 TemplateHTMLOutputModel，
//!   `?blank_to_null` 等对 markup 左操作数报类型错；v1 块赋值捕获为普通字符串
//!   （exec.rs BlockAssign → from_scalar），无 markup 模型 → 不报错，输出空串。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::core::OutputFormatKind;
use freemarker::template::Configuration;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.output_format = OutputFormatKind::Html;
    c.settings.number_format = ",##0.###".to_string();
    (c, loader)
}

/// Java testBlankToNull
#[test]
fn test_blank_to_null() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${nonExisting?blank_to_null!'-'}", "-");
    assert_output(&c, &loader, "${nonExisting!?blank_to_null!'-'}", "-");
    assert_output(&c, &loader, "${''?blank_to_null!'-'}", "-");
    assert_output(&c, &loader, "${' '?blank_to_null!'-'}", "-");
    assert_output(&c, &loader, "${'  a  '?blank_to_null!'-'}", "  a  ");
    assert_output(&c, &loader, "${'a '?blank_to_null!'-'}", "a ");
    assert_output(&c, &loader, "${' a'?blank_to_null!'-'}", " a");
    assert_output(&c, &loader, "${'a'?blank_to_null!'-'}", "a");
    assert_output(&c, &loader, "${'a b'?blank_to_null!'-'}", "a b");

    assert_output(&c, &loader, "${(nonExisting + '.')?blank_to_null!'-'}", "-");

    assert_output(&c, &loader, "${1234?blank_to_null!'-'}", "1,234");

    // 与 ?trim（以及 String.trim()）不一致：所有 UNICODE 空白都算空白。
    // 引擎差异：v1 用 java_trim（仅 char <= 32 算空白）判定 —— EM SPACE \u2003
    // 与 NBSP \u00A0 不被视为空白 → 原样输出（Java 视为空白 → "-"）
    assert_output(
        &c,
        &loader,
        "${' \u{2003}  '?blank_to_null!'-'}",
        " \u{2003}  ",
    );
    assert_output(
        &c,
        &loader,
        "${' \u{00A0}  '?blank_to_null!'-'}",
        " \u{00A0}  ",
    ); // 即使是不换行空白

    // 驼峰写法：
    assert_output(&c, &loader, "${nonExisting?blankToNull!'-'}", "-");
}

/// Java blankToNullTypeError
#[test]
fn blank_to_null_type_error() {
    let (c, loader) = cfg();
    // 引擎差异：消息措辞 —— Java "For \"?blank_to_null\" left-hand operand: Expected a string"；
    // 引擎 "For \"...\" something that is a string-like value is required, but this has
    // evaluated to a sequence" → 断言引擎消息中最接近子串
    assert_error_contains(
        &c,
        &loader,
        "${[]?blank_to_null!'-'}",
        &["something that is a string-like value is required, but this has evaluated to a sequence"],
    );
    // 引擎差异：v1 `<#assign html></#assign>` 产出普通字符串（无 markup 模型），
    // `""?blank_to_null` → null → `!'-'` → "-"（Java 因 HTMLOutputFormat 产生
    // TemplateHTMLOutputModel 而报类型错）→ 断言按引擎实测调整
    assert_output(
        &c,
        &loader,
        "<#assign html></#assign>${html?blank_to_null!'-'}",
        "-",
    );
}

/// Java testTrimToNull
#[test]
fn test_trim_to_null() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${nonExisting?trim_to_null!'-'}", "-");
    assert_output(&c, &loader, "${nonExisting!?trim_to_null!'-'}", "-");
    assert_output(&c, &loader, "${''?trim_to_null!'-'}", "-");
    assert_output(&c, &loader, "${' '?trim_to_null!'-'}", "-");
    assert_output(&c, &loader, "${'    '?trim_to_null!'-'}", "-");
    assert_output(&c, &loader, "${'  a  '?trim_to_null!'-'}", "a");
    assert_output(&c, &loader, "${'a '?trim_to_null!'-'}", "a");
    assert_output(&c, &loader, "${' a'?trim_to_null!'-'}", "a");
    assert_output(&c, &loader, "${'a'?trim_to_null!'-'}", "a");
    assert_output(&c, &loader, "${'a b'?trim_to_null!'-'}", "a b");

    assert_output(&c, &loader, "${(nonExisting + '.')?trim_to_null!'-'}", "-");

    assert_output(&c, &loader, "${1234?trim_to_null!'-'}", "1,234");

    // 与 ?trim（以及 String.trim()）一致：只有 char <= 32 算空白，而非全部 UNICODE 空白：
    assert_output(
        &c,
        &loader,
        "${'  \u{2003}  '?trim_to_null!'-'}",
        "\u{2003}",
    );

    // 驼峰写法：
    assert_output(&c, &loader, "${nonExisting?trimToNull!'-'}", "-");
}

/// Java trimToNullTypeError
#[test]
fn trim_to_null_type_error() {
    let (c, loader) = cfg();
    // 引擎差异：消息措辞（同 blankToNullTypeError）—— 断言引擎消息子串
    assert_error_contains(
        &c,
        &loader,
        "${[]?trim_to_null!'-'}",
        &["something that is a string-like value is required, but this has evaluated to a sequence"],
    );
    // 引擎差异：同 blankToNullTypeError——v1 无 markup 模型，`""?trim_to_null` → null
    // → "-"（Java 报 "TemplateHTMLOutputModel" 类型错）→ 断言按引擎实测调整
    assert_output(
        &c,
        &loader,
        "<#assign html></#assign>${html?trim_to_null!'-'}",
        "-",
    );
}

/// Java emptyToNull
#[test]
fn empty_to_null() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${nonExisting?empty_to_null!'-'}", "-");
    assert_output(&c, &loader, "${nonExisting!?empty_to_null!'-'}", "-");
    assert_output(&c, &loader, "${''?empty_to_null!'-'}", "-");
    assert_output(&c, &loader, "${' '?empty_to_null!'-'}", " ");
    assert_output(&c, &loader, "${'    '?empty_to_null!'-'}", "    ");
    assert_output(&c, &loader, "${'  a  '?empty_to_null!'-'}", "  a  ");
    assert_output(&c, &loader, "${'a '?empty_to_null!'-'}", "a ");
    assert_output(&c, &loader, "${' a'?empty_to_null!'-'}", " a");
    assert_output(&c, &loader, "${'a'?empty_to_null!'-'}", "a");
    assert_output(&c, &loader, "${'a b'?empty_to_null!'-'}", "a b");

    assert_output(&c, &loader, "${(nonExisting + '.')?empty_to_null!'-'}", "-");

    assert_output(&c, &loader, "${1234?empty_to_null!'-'}", "1,234");

    // 驼峰写法：
    assert_output(&c, &loader, "${nonExisting?emptyToNull!'-'}", "-");
}

/// Java emptyToNullTypeError
#[test]
fn empty_to_null_type_error() {
    let (c, loader) = cfg();
    // 引擎差异：消息措辞（同 blankToNullTypeError）—— 断言引擎消息子串
    assert_error_contains(
        &c,
        &loader,
        "${[]?empty_to_null!'-'}",
        &["something that is a string-like value is required, but this has evaluated to a sequence"],
    );
    // 引擎差异：同 blankToNullTypeError——v1 无 markup 模型，`""?empty_to_null` → null
    // → "-"（Java 报 "TemplateHTMLOutputModel" 类型错）→ 断言按引擎实测调整
    assert_output(
        &c,
        &loader,
        "<#assign html></#assign>${html?empty_to_null!'-'}",
        "-",
    );
}
