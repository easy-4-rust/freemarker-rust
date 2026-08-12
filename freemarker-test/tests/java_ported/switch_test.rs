//! Java `freemarker.core.SwitchTest` 的 Rust 1:1 实现
//! （对应 Java: SwitchTest —— #switch/#case/#default/#on 的行为与错误）
//!
//! 引擎差异总注：v1 的 #switch 只支持 #case/#default（fall-through 与
//! 源码序 default 均实现）；`<#on>`（Java 2.3.28+）未实现 —— #on 相关断言
//! 保留 Java 原文。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testCaseBasics：case 匹配 + fall-through + 可选 default
#[test]
fn test_case_basics() {
    test_case_basics_impl(true);
    test_case_basics_impl(false);
}

fn test_case_basics_impl(has_default: bool) {
    let (c, loader) = test_config();
    for i in 1..=6 {
        let expected = if i < 6 {
            format!(
                "[Case {}]",
                if i < 4 {
                    i.to_string()
                } else {
                    "4 or 5".to_string()
                }
            )
        } else if has_default {
            "[D]".to_string()
        } else {
            "[]".to_string()
        };
        let default_part = if has_default { "<#default>D" } else { "" };
        assert_output(
            &c,
            &loader,
            &format!(
                "[<#switch {i}>\n<#case 3>Case 3<#break><#case 1>Case 1<#break><#case 4><#case 5>Case 4 or 5<#break><#case 2>Case 2<#break>{default_part}</#switch>]"
            ),
            &expected,
        );
    }
}

/// Java testCasesWithOddlyPlacedDefault：向后兼容的"default 排在 case 前"写法
#[test]
fn test_cases_with_oddly_placed_default() {
    let (c, loader) = test_config();
    assert_output(
        &c,
        &loader,
        "<#list 1..3 as i><#switch i><#case 1>1<#default>D<#case 3>3</#switch>;</#list>",
        "1D3;D;3;",
    );
}

/// Java testDefaultOnly：只有 default
#[test]
fn test_default_only() {
    let (c, loader) = test_config();
    assert_output(&c, &loader, "<#switch 1><#default>D</#switch>", "D");
    assert_output(
        &c,
        &loader,
        "<#list 1..2 as i><#switch 1><#default>D<#break>unreachable</#switch></#list>",
        "DD",
    );
}

/// Java testCaseWhitespace：case 块间的空白剥离
#[test]
fn test_case_whitespace() {
    let (c, loader) = test_config();
    assert_output(
        &c,
        &loader,
        "<#list 1..3 as i>\n[\n  <#switch i>\n    <#case 1>C1\n    <#case 2>C2<#break>\n    <#default>D\n  </#switch>\n]\n</#list>",
        "[\nC1\n    C2]\n[\nC2]\n[\nD\n]\n",
    );
}

/// Java testOn：#on 匹配
/// （引擎差异：v1 未实现 `<#on>`（Java 2.3.28+ 的 #switch case 形式），
/// 模板解析报 "Unexpected directive <#on>"——断言保留 Java 原文）
#[test]
fn test_on() {
    test_on_basics_impl(true);
    test_on_basics_impl(false);
}

fn test_on_basics_impl(has_default: bool) {
    let (c, loader) = test_config();
    for i in 1..=6 {
        let expected = if i < 6 {
            format!(
                "[On {}]",
                if i < 4 {
                    i.to_string()
                } else {
                    "4 or 5".to_string()
                }
            )
        } else if has_default {
            "[D]".to_string()
        } else {
            "[]".to_string()
        };
        let default_part = if has_default { "<#default>D" } else { "" };
        // 引擎差异：`<#on>` 未实现（Java 2.3.28+ 的 #switch case 形式）——
        // 本模板解析报错而非按 #on 匹配
        assert_output(
            &c,
            &loader,
            &format!(
                "[<#switch {i}>\n<#on 3>On 3<#on 1>On 1<#on 4, 5>On 4 or 5<#on 2>On 2{default_part}</#switch>]"
            ),
            &expected,
        );
    }
}

/// Java testOnParsingErrors：#on 的解析期错误
/// （引擎差异：v1 未实现 `<#on>` —— 断言保留 Java 原文）
#[test]
fn test_on_parsing_errors() {
    let (c, loader) = test_config();
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#on 1>On 1<#default>D<#on 2>On 2</#switch>",
        &["#on after #default"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#on 1>On 1<#case 2>On 2</#switch>",
        &["can't use both #on, and #case", "already had an #on"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#case 1>On 1<#on 2>On 2</#switch>",
        &["can't use both #on, and #case", "already had a #case"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#on 1>On 1<#default>D<#case 2>On 2</#switch>",
        &["can't use both #on, and #case", "already had an #on"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#on 1>On 1<#default>D1<#default>D2</#switch>",
        &["already had a #default"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#case 1>On 1<#default>D1<#default>D2</#switch>",
        &["already had a #default"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#on 1>On 1<#default>D<#on 2>On 2</#switch>",
        &["#on after #default"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#switch x><#default>D<#on 2>On 2</#switch>",
        &["#on after #default"],
    );
}

/// Java testOnWhitespace：#on 块间的空白剥离
/// （引擎差异：v1 未实现 `<#on>` —— 断言保留 Java 原文）
#[test]
fn test_on_whitespace() {
    let (c, loader) = test_config();
    assert_output(
        &c,
        &loader,
        "<#list 1..3 as i>\n[\n  <#switch i>\n    <#on 1>C1\n    <#on 2>C2\n    <#default>D\n  </#switch>\n]\n</#list>",
        "[\nC1\n    ]\n[\nC2\n    ]\n[\nD\n]\n",
    );
    assert_output(
        &c,
        &loader,
        "<#list 1..3 as i>\n[\n  <#switch i>\n    <#on 1>C1<#t>\n    <#on 2>C2<#t>\n    <#default>D<#t>\n  </#switch>\n]\n</#list>",
        "[\nC1]\n[\nC2]\n[\nD]\n",
    );
    assert_output(
        &c,
        &loader,
        "<#list 1..3 as i>\n[\n  <#switch i>\n    <#on 1>\n      C1\n    <#on 2>\n      C2\n    <#default>\n      D\n  </#switch>\n]\n</#list>",
        "[\n      C1\n]\n[\n      C2\n]\n[\n      D\n]\n",
    );
}
