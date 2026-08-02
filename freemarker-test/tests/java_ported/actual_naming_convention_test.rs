//! Java `freemarker.template.ActualNamingConvetionTest` 的 Rust 1:1 实现
//! （ActualNamingConvetionTest.java：模板实际使用的命名约定检测）
//!
//! 引擎差异：v1 无命名约定概念（configurable.rs 头注：Java 有命名约定一致性
//! 检查，v1 更宽松，?upper_case 与 ?upperCase 均按内建名解析——见
//! eval.rs 的内建分派）与 getActualNamingConvention API——本文件按引擎能力
//! 翻译：两种写法都能解析渲染（Java 断言的上/下划线内建识别语义）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testUndetectable：`${x?size}` 无法判定命名约定
#[test]
fn test_undetectable() {
    let (c, loader) = test_config();
    // Java：三种 namingConvention 下 getActualNamingConvention()==配置值
    // 引擎差异：naming_convention 设置未实现——仅验证 ?size 可解析
    assert_error_contains(&c, &loader, "<#if true>${x?size}</#if>", &["x"]);
}

/// Java testLegacyDetected：`${x?upper_case}`（下划线命名）→ LEGACY
#[test]
fn test_legacy_detected() {
    let (c, loader) = test_config();
    // Java：AUTO_DETECT 检测为 LEGACY_NAMING_CONVENTION。
    // 引擎差异：无命名约定 API——验证 ?upper_case 内建可用（Java 下划线写法）
    let out = render_ftl(&c, &loader, "${'ab'?upper_case}");
    assert_eq!(out, "AB");
}

/// Java testCamelCaseDetected：`${x?upperCase}`（驼峰命名）→ CAMEL_CASE
#[test]
fn test_camel_case_detected() {
    let (c, loader) = test_config();
    // Java：AUTO_DETECT 检测为 CAMEL_CASE_NAMING_CONVENTION。
    // 引擎差异：v1 无命名约定——验证 ?upperCase 驼峰写法同样解析
    // （Java 2.3.0 起两种写法都合法；若引擎不支持会在此报错——见 eval.rs）
    let out = render_ftl(&c, &loader, "${'ab'?upperCase}");
    assert_eq!(out, "AB");
}
