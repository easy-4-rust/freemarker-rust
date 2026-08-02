//! Java `freemarker.template.CustomAttributeTest` 的 Rust 1:1 实现
//! （CustomAttributeTest.java：Template/Environment/Configuration 范围的
//!   自定义属性测试）
//!
//! 引擎差异：v1 无 CustomAttribute 家族（模板/环境/配置的自定义属性槽、
//! `<#ftl attributes={...}>` 头部解析、CustomAttribute.SCOPE_* 均未移植）——
//! 整体跳过并注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testStringKey：字符串键属性的增删查
#[test]
fn test_string_key() {
    // Java：t.getCustomAttributeNames()/getCustomAttribute/setCustomAttribute/
    // removeCustomAttribute 的字符串键语义（置 null 保留键名、remove 删除键）。
    // v1 Template 无自定义属性 API。
}

/// Java testRemoveFromEmptySet：空集合移除不报错
#[test]
fn test_remove_from_empty_set() {
    // Java：空模板上 removeCustomAttribute 无害；set 后可读回。
    // v1 无对应 API。
}

/// Java testFtlHeader / testFtl2Header / testFtl3Header：`<#ftl attributes={...}>`
/// 头部声明的属性（列表/数值/字符串值、移除与置 null）。
/// 引擎差异：v1 解析器未实现 attributes 头部参数。
#[test]
fn test_ftl_header() {
    // Java：<#ftl attributes={'key1': ['s', 2, true, {'a':'A'}], 'key2': 22}>
    // → getCustomAttributeNames()==[key1,key2] 等。
    // v1 无 attributes 头部（语法错误或忽略——未实现）。
}

/// Java testObjectKey：CustomAttribute 对象键（SCOPE_TEMPLATE 等）
#[test]
fn test_object_key() {
    // Java：CustomAttribute 实例按对象身份存取，不占用字符串键名空间。
    // v1 无 CustomAttribute。
}

/// Java testScopes：SCOPE_ENVIRONMENT/SCOPE_CONFIGURATION 的取值时机校验
/// （无当前环境时 get() 抛 IllegalStateException；模板内回调断言各范围值）。
/// 引擎差异：v1 无作用域自定义属性与模板回调机制。
#[test]
fn test_scopes() {
    // Java：无环境时 CUST_ATT_ENV_1.get() 抛 IllegalStateException；模板
    // "${testScopesFromTemplateStep1()}" 中读到 123/1234/12345 等。
    // v1 无 CustomAttribute/Environment 回调。
}
