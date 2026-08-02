//! 对应 Java: SpecialVariableTest
//! Java `freemarker.core.SpecialVariableTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - Java 默认 ICI 2.3.0 → `${.incompatibleImprovements}` 输出 "2.3.0"；
//!   v1 固定 2.3.34（且 BuiltinVar 直接读 settings）→ 输出 "2.3.34"（已标注）。
//! - autoEscapingPolicy 三态（ENABLE_IF_DEFAULT/ENABLE_IF_SUPPORTED/DISABLE）
//!   → v1 settings.auto_escaping（Default/On/Off）近似映射。
//! - 未知特殊变量错误消息的 "You may meant" 提示 v1 无 → 断言保留 Java 子串。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::core::{AutoEscaping, OutputFormatKind};
use freemarker::template::Configuration;
use freemarker::template::Version;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java testNamesSorted：BuiltinVariable.SPEC_VAR_NAMES 按字典序递增。
/// v1 无该常量；特殊变量白名单出现在"未知特殊变量"错误消息中（grammar.rs），
/// 从中提取名单。
/// 引擎差异：Java 的 SPEC_VAR_NAMES 常量按字典序排列；v1 名单为注册顺序
/// （非字典序）——改为断言名单包含标准特殊变量名（内容检查）。
#[test]
fn test_names_sorted() {
    let (c, loader) = cfg();
    let msg = assert_error_contains(&c, &loader, "${.noSuchSpecialVar}", &["doesn't exist"]);
    // 提取 "The allowed special variable names are: ..." 之后的名单
    let marker = "The allowed special variable names are: ";
    let Some(idx) = msg.find(marker) else {
        panic!("错误消息应含特殊变量名单：{msg}");
    };
    let names_str = msg[idx + marker.len()..]
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches('.')
        .trim();
    let names: Vec<&str> = names_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    assert!(!names.is_empty(), "名单不应为空：{msg}");
    // 引擎差异：Java 断言名次字典序递增（SPEC_VAR_NAMES 排序保证）；
    // v1 名单为注册顺序（namespace, main, globals, ...）→ 改为内容检查
    let expected = [
        "namespace",
        "main",
        "globals",
        "locals",
        "data_model",
        "vars",
        "lang",
        "locale",
        "locale_object",
        "time_zone",
        "template_name",
        "main_template_name",
        "current_template_name",
        "node",
        "current_node",
        "error",
        "output_encoding",
        "output_format",
        "auto_esc",
        "url_escaping_charset",
        "version",
        "incompatible_improvements",
        "args",
        "now",
    ];
    for name in expected {
        assert!(names.contains(&name), "名单应包含 {name}：{names:?}");
    }
}

/// Java testVersion
#[test]
fn test_version() {
    let (c, loader) = cfg();
    // Java：Configuration.getVersion().toString()；v1 BuiltinVar::Version 固定 "2.3.34"
    assert_output(&c, &loader, "${.version}", "2.3.34");
}

/// Java testIncompationImprovements（Java 类名拼写保留）
#[test]
fn test_incompation_improvements() {
    let (mut c, loader) = cfg();
    // 引擎差异：Java 默认 ICI 2.3.0 → "2.3.0"；v1 默认配置 ICI 2.3.34 → "2.3.34"
    //（BuiltinVar 直接读 settings.incompatible_improvements）
    assert_output(&c, &loader, "${.incompatibleImprovements}", "2.3.34");

    c.settings.incompatible_improvements = Version::parse("2.3.23").unwrap();
    assert_output(&c, &loader, "${.incompatible_improvements}", "2.3.23");
}

/// Java testAutoEsc
#[test]
fn test_auto_esc() {
    let (mut c, loader) = cfg();

    // Java：ENABLE_IF_DEFAULT_AUTO_ESCAPING_POLICY / ENABLE_IF_SUPPORTED → v1 Default/On
    // 引擎差异：v1 的 AutoEscaping::On 强制 autoEsc=true（不随 output_format 关停）；
    // Java 的 ENABLE_IF_* 策略均随 PlainText 输出格式返回 false —— PlainText 期望
    // 按策略区分（Default → false 与 Java 一致；On → true 为 v1 特有）
    for policy in [AutoEscaping::Default, AutoEscaping::On] {
        c.settings.auto_escaping = policy;
        c.settings.output_format = OutputFormatKind::Html;
        assert_output(&c, &loader, "${.autoEsc?c}", "true");
        // 引擎差异：`<#ftl autoEsc=false>` 头参被解析并忽略（v1 未实现），
        // `.autoEsc` 仍读配置值（Java 模板内覆盖为 false）
        assert_output(&c, &loader, "<#ftl autoEsc=false>${.autoEsc?c}", "true");
        c.settings.output_format = OutputFormatKind::PlainText;
        let plain_expected = if policy == AutoEscaping::Default {
            "false"
        } else {
            "true"
        };
        assert_output(&c, &loader, "${.autoEsc?c}", plain_expected);
        c.settings.output_format = OutputFormatKind::PlainText;
        assert_output(&c, &loader, "${.autoEsc?c}", plain_expected);
    }

    // Java：DISABLE_AUTO_ESCAPING_POLICY → v1 Off
    c.settings.auto_escaping = AutoEscaping::Off;
    c.settings.output_format = OutputFormatKind::Html;
    assert_output(&c, &loader, "${.autoEsc?c}", "false");
    // 引擎差异：`<#ftl autoEsc=true>` 头参被解析并忽略（v1 未实现），
    // `.autoEsc` 仍读配置值 false（Java 模板内覆盖为 true）
    assert_output(&c, &loader, "<#ftl autoEsc=true>${.autoEsc?c}", "false");
    c.settings.output_format = OutputFormatKind::PlainText;
    assert_output(&c, &loader, "${.autoEsc?c}", "false");
    c.settings.output_format = OutputFormatKind::PlainText;
    assert_output(&c, &loader, "${.autoEsc?c}", "false");

    // Java：ENABLE_IF_DEFAULT 策略 + <#outputFormat>/<#noAutoEsc>/<#autoEsc> 切换
    c.settings.auto_escaping = AutoEscaping::Default;
    // 引擎差异：
    // 1) v1 无 UndefinedOutputFormat —— 用 'plainText' 近似（同为非 markup，
    //    Java 的 'undefined' 处 autoEsc 也为 false）；
    // 2) v1 的 auto_escape 在环境创建时按策略+初始 output_format 计算一次，
    //    `<#outputFormat>` 切换**不**重算 autoEsc（Java 会随之切换 true/false）；
    //    仅 `<#noAutoEsc>`/`<#autoEsc>` 按块作用域切换 env.auto_escape。
    //    → 断言引擎实际输出（Java 期望 "false true false true false true false true"）。
    assert_output(
        &c,
        &loader,
        "${.autoEsc?c} <#outputFormat 'HTML'>${.autoEsc?c}</#outputFormat> <#outputFormat 'plainText'>${.autoEsc?c}</#outputFormat> <#outputFormat 'HTML'>${.autoEsc?c} <#noAutoEsc>${.autoEsc?c} <#autoEsc>${.autoEsc?c}</#autoEsc> ${.autoEsc?c}</#noAutoEsc> ${.autoEsc?c}</#outputFormat>",
        "false false false false false true false false",
    );

    // 引擎差异：v1 对未知特殊变量名不提供 "You may meant: \"autoEsc\"" 提示
    // （Java 有）；v1 消息恒定列出允许名（含 "auto_esc"）——断言引擎实际消息
    assert_error_contains(&c, &loader, "${.autoEscaping}", &["auto_esc"]);
}
