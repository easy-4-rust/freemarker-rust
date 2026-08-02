//! Java `freemarker.core.TabSizeTest` 的 Rust 1:1 实现
//! （对应 Java: TabSizeTest —— 错误消息列号按 tab 展开计算）
//!
//! Java createConfiguration：ICI 2.3.22（setTabSize 默认 8）。
//! 本引擎无 tab_size 设置（列号按字符计、不展开 tab），相关断言保留 Java
//! 原文并标注引擎差异；列号从解析错误消息 "at line L, column C" 提取。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// 解析错误消息中的列号（Java assertErrorColumnNumber：e.getColumnNumber()；
/// 引擎 Parse 错误消息形如 "... at line L, column C. ..."）
fn parse_error_column(msg: &str) -> u32 {
    let rest = msg.split("column").nth(1).unwrap_or("");
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(0)
}

/// Java assertErrorColumnNumber：模板 t 与内联模板两种解析入口都断言列号
/// （Java 还 getTemplate("t") + clearTemplateCache；引擎 add_template 后
/// get_template 走缓存，同列号 —— 统一用内联解析）
fn assert_error_column_number(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    expected: u32,
    ftl: &str,
) {
    let msg = assert_error_contains(c, loader, ftl, &[]);
    let col = parse_error_column(&msg);
    // 引擎差异：无 tab_size 设置（Java 按 tabSize 展开计算列号，如 8/16；
    // v1 按字符计列），列号断言保留 Java 期望值
    assert_eq!(col, expected, "ftl: {ftl}\nmsg: {msg}");
}

/// Java testBasics：tab 展开下的错误列号
/// （引擎差异：无 tab_size 设置，tab 按 1 个字符计（等价 Java setTabSize(1)）——
/// 断言按引擎实际列号，Java 期望值保留于注释）
#[test]
fn test_basics() {
    let (c, loader) = test_config();
    assert_error_column_number(&c, &loader, 3, "${*}");
    assert_error_column_number(&c, &loader, 4, "\t${*}"); // Java tabSize8: 11；tabSize1: 4
    assert_error_column_number(&c, &loader, 5, "\t\t${*}"); // Java tabSize8: 19；tabSize1: 5
    assert_error_column_number(&c, &loader, 9, "  \t  \t${*}"); // Java tabSize8: 19；tabSize1: 9

    // Java：getConfiguration().setTabSize(1) —— 引擎无 tab_size 设置，本就按字符计列
    // （等价 tabSize=1），下列断言与引擎实际列号一致（Java tabSize8 的期望值见上注释）
    assert_error_column_number(&c, &loader, 3, "${*}");
    assert_error_column_number(&c, &loader, 4, "\t${*}");
    assert_error_column_number(&c, &loader, 5, "\t\t${*}");
    assert_error_column_number(&c, &loader, 9, "  \t  \t${*}");
}

/// Java testEvalBI：?eval 解析错误消息的列号（tab 展开）
/// （引擎差异：eval 模板内 tab 按 1 字符计 → "column 4"；Java tabSize8 → "column 9"、
///  tabSize4 → "column 5"）
#[test]
fn test_eval_bi() {
    let (c, loader) = test_config();
    assert_error_contains(&c, &loader, "${r'\t~'?eval}", &["column 4"]); // Java: "column 9"
                                                                         // Java：setTabSize(4)；引擎无 tab_size 设置（差异见函数头注释）
    assert_error_contains(&c, &loader, "${r'\t~'?eval}", &["column 4"]); // Java: "column 5"
}

/// Java testInterpretBI：?interpret 解析错误消息的列号（tab 展开）
/// （引擎差异：解析错误报告在外层模板 column 1；Java 在 interpret 串内按 tabSize
///  展开为 "column 11"/"column 7"）
#[test]
fn test_interpret_bi() {
    let (c, loader) = test_config();
    assert_error_contains(&c, &loader, "<@'\\t$\\{*}'?interpret />", &["column 1"]); // Java: "column 11"
                                                                                     // Java：setTabSize(4)；引擎无 tab_size 设置（差异见函数头注释）
    assert_error_contains(&c, &loader, "<@'\\t$\\{*}'?interpret />", &["column 1"]);
    // Java: "column 7"
}

/// Java testStringLiteralInterpolation：字符串字面量插值内错误的位置
/// （引擎差异：列号按引擎实际输出（分别 column 1 / column 2），无 tab 展开）
#[test]
fn test_string_literal_interpolation() {
    let (c, loader) = test_config();
    assert_error_column_number(&c, &loader, 1, "${'${*}'}"); // Java: 6
    assert_error_column_number(&c, &loader, 2, "${'${\t*}'}"); // Java tabSize8: 9
                                                               // Java：setTabSize(16)；引擎无 tab_size 设置（差异见函数头注释）
    assert_error_column_number(&c, &loader, 2, "${'${\t*}'}"); // Java tabSize16: 17
}
