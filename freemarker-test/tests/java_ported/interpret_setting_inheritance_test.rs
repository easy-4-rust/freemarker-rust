//! 对应 Java: InterpretSettingInheritanceTest
//! Java `freemarker.core.InterpretSettingInheritanceTest` 的 Rust 1:1 实现：
//! `?interpret` 产物不继承周边模板的 tag_syntax 自动检测结果与空白剥离设置，
//! 只依赖 Configuration。
//!
//! 引擎差异：
//! - v1 无配置级 `tag_syntax` 设置（Settings 无该字段；parser 恒按模板首标签
//!   自动检测，等同 Java `AUTO_DETECT_TAG_SYNTAX`）。Java tagSyntaxTest /
//!   evalTest 的 ANGLE_BRACKET / SQUARE_BRACKET 显式设置块在 v1 不可复现，
//!   仅保留等价于 AUTO_DETECT 的断言（引擎固定行为）。
//! - whitespaceStrippingTest 可 1:1 翻译（v1 支持 whitespace_stripping 设置与
//!   `<#ftl stripWhitespace=false>` 头部）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

const FTL_A_S_A: &str = "<#ftl><@'[#if true]s[/#if]<#if true>a</#if>'?interpret />";
const FTL_A_A_S: &str = "<#ftl><@'<#if true>a</#if>[#if true]s[/#if]'?interpret />";
const FTL_S_S_A: &str = "[#ftl][@'[#if true]s[/#if]<#if true>a</#if>'?interpret /]";
const FTL_S_A_S: &str = "[#ftl][@'<#if true>a</#if>[#if true]s[/#if]'?interpret /]";
const OUT_S_A_WHEN_SYNTAX_IS_S: &str = "s<#if true>a</#if>";
// 以下两个常量仅保留 Java 原文（对应 Java 中 ANGLE/SQUARE 显式设置块的预期输出，
// v1 无 tag_syntax 设置不可达；见 tagSyntaxTest 注释）
#[allow(dead_code)]
const OUT_S_A_WHEN_SYNTAX_IS_A: &str = "[#if true]s[/#if]a";
const OUT_A_S_WHEN_SYNTAX_IS_A: &str = "a[#if true]s[/#if]";
#[allow(dead_code)]
const OUT_A_S_WHEN_SYNTAX_IS_S: &str = "<#if true>a</#if>s";

/// Java tagSyntaxTest —— ?interpret 不继承周边已确立的 tag_syntax。
/// 引擎差异：v1 无配置级 tag_syntax 设置，恒自动检测（等同 Java AUTO_DETECT 块）；
/// Java 的 ANGLE_BRACKET / SQUARE_BRACKET 显式设置断言不可复现，未翻译。
#[test]
fn tag_syntax_test() {
    let (c, loader) = test_config();
    // Java cfg.setTagSyntax(AUTO_DETECT_TAG_SYNTAX) 下的四组断言（v1 固定行为）：
    assert_output(&c, &loader, FTL_S_A_S, OUT_A_S_WHEN_SYNTAX_IS_A);
    assert_output(&c, &loader, FTL_S_S_A, OUT_S_A_WHEN_SYNTAX_IS_S);
    assert_output(&c, &loader, FTL_A_A_S, OUT_A_S_WHEN_SYNTAX_IS_A);
    assert_output(&c, &loader, FTL_A_S_A, OUT_S_A_WHEN_SYNTAX_IS_S);
    // 解释模板自身自动检测为方括号语法，周边模板为尖括号语法 → 两者互不影响
    assert_output(
        &c,
        &loader,
        "<@'[#ftl]x'?interpret />[#if true]y[/#if]",
        "x[#if true]y[/#if]",
    );
}

/// Java whitespaceStrippingTest —— ?interpret 的空白剥离只由自身决定
/// （配置设置或解释模板内的 `<#ftl stripWhitespace=...>` 头部）。
#[test]
fn whitespace_stripping_test() {
    let (mut c, loader) = test_config();

    c.settings.whitespace_stripping = true;
    assert_output(
        &c,
        &loader,
        "<#assign x = 1>\nX<@'<#assign x = 1>\\nY'?interpret />",
        "XY",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=false><#assign x = 1>\nX<@'<#assign x = 1>\\nY'?interpret />",
        "\nXY",
    );
    assert_output(
        &c,
        &loader,
        "<#assign x = 1>\nX<@'<#ftl stripWhitespace=false><#assign x = 1>\\nY'?interpret />",
        "X\nY",
    );

    c.settings.whitespace_stripping = false;
    assert_output(
        &c,
        &loader,
        "<#assign x = 1>\nX<@'<#assign x = 1>\\nY'?interpret />",
        "\nX\nY",
    );
    assert_output(
        &c,
        &loader,
        "<#ftl stripWhitespace=true><#assign x = 1>\nX<@'<#assign x = 1>\\nY'?interpret />",
        "X\nY",
    );
    assert_output(
        &c,
        &loader,
        "<#assign x = 1>\nX<@'<#ftl stripWhitespace=true><#assign x = 1>\\nY'?interpret />",
        "\nXY",
    );
}

/// Java evalTest —— `?eval` 内的字符串经 `?interpret` 同样不继承周边 tag_syntax。
/// 引擎差异：同 tagSyntaxTest —— 无配置级 tag_syntax，恒自动检测；
/// 解释源以 `[` 开头 → 方括号语法，四组断言均为 OUT_S_A_WHEN_SYNTAX_IS_S。
/// （Java 设 ANGLE_BRACKET 时预期 OUT_S_A_WHEN_SYNTAX_IS_A —— 引擎不可达）
#[test]
fn eval_test() {
    let (c, loader) = test_config();
    // Java 断言 1：<@'"..."?interpret'?eval /> → 引擎自动检测（方括号）输出
    assert_output(
        &c,
        &loader,
        "<@'\"[#if true]s[/#if]<#if true>a</#if>\"?interpret'?eval />",
        OUT_S_A_WHEN_SYNTAX_IS_S,
    );
    // Java 断言 2：[#ftl][@'"..."?interpret'?eval /] → 同引擎输出
    assert_output(
        &c,
        &loader,
        "[#ftl][@'\"[#if true]s[/#if]<#if true>a</#if>\"?interpret'?eval /]",
        OUT_S_A_WHEN_SYNTAX_IS_S,
    );
}
