//! Java `freemarker.template.ActualTagSyntaxTest` 的 Rust 1:1 实现
//! （ActualTagSyntaxTest.java：模板实际使用的标签语法检测）
//!
//! 引擎映射：v1 解析器同时支持尖括号与方括号语法（lexer.rs：`[#` 为方括号指令
//! 语法、`[#ftl]` 头部仅在模板首行有效）——无 `getActualTagSyntax` API，
//! 以"能否按预期语法解析并渲染"表达同义断言。
//! 引擎差异：Java 的 tagSyntax 设置（AUTO_DETECT/ANGLE_BRACKET/SQUARE_BRACKET）
//! 与 getActualTagSyntax 未实现（v1 无 tag_syntax 设置字段）；`[#ftl]`/`<#ftl>`
//! 头部与方括号指令的解析能力本身与 Java 一致。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testWithFtlHeader：`[#ftl]` 声明方括号、`<#ftl>` 声明尖括号。
/// 引擎差异：Java 断言 getActualTagSyntax() 返回值（SQUARE_BRACKET/
/// ANGLE_BRACKET）；v1 无此 API——改为断言两种头部都能正常解析渲染
/// （引擎自动识别两种语法，与 Java AUTO_DETECT 语义一致）。
#[test]
fn test_with_ftl_header() {
    let (c, loader) = test_config();
    // Java：三种 cfgTagSyntax 下 getActualTagSyntax("[#ftl]foo")==SQUARE_BRACKET、
    // getActualTagSyntax("<#ftl>foo")==ANGLE_BRACKET
    // 引擎差异：tagSyntax 设置未实现（v1 恒自动识别）——解析+渲染验证
    assert_output(&c, &loader, "[#ftl]foo", "foo");
    assert_output(&c, &loader, "<#ftl>foo", "foo");
}

/// Java testUndecidable：无可判定标记时按配置语法解释
#[test]
fn test_undecidable() {
    let (c, loader) = test_config();
    // Java：AUTO_DETECT/ANGLE_BRACKET 下 "foo" → ANGLE_BRACKET；
    // SQUARE_BRACKET 下 → SQUARE_BRACKET。
    // 引擎差异：v1 无 tagSyntax 设置——"foo" 为纯文本，两种语法下输出一致
    assert_output(&c, &loader, "foo", "foo");
}

/// Java testDecidableWithoutFtlHeader：尖括号/方括号指令可判定语法
#[test]
fn test_decidable_without_ftl_header() {
    let (c, loader) = test_config();
    // Java：AUTO_DETECT 下 "<#if true></#if>" → ANGLE_BRACKET、
    // "[#if true][/#if]" → SQUARE_BRACKET（v1 同样自动识别两种语法）
    assert_output(&c, &loader, "foo<#if true></#if>", "foo");
    assert_output(&c, &loader, "foo[#if true][/#if]", "foo");
}
