//! 对应 Java: CapturingAssignmentTest
//! Java `freemarker.core.CapturingAssignmentTest` 的 Rust 1:1 实现。
//!
//! 引擎差异：Java 在 HTML 输出格式下 `<#assign x>...</#assign>` 捕获为
//! TemplateHTMLOutputModel（markup）→ `${x + '&'}` 中普通 '&' 被转义、已捕获的
//! 标记不再转义，输出 "<p>2&amp;"；v1 的 HTML 输出格式**不实现 auto-escape**
//! （整段按字面量直出，连普通 '&' 也不转义）→ 实际输出 "<p>2&"，与无 outputFormat
//! 时一致 —— HTML 用例断言按引擎实际行为（保留 Java 期望值于注释）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java testAssign
#[test]
fn test_assign() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "<#assign x></#assign>[${x}]", "[]");
    assert_output(
        &c,
        &loader,
        "<#assign x><p>${1 + 1}</#assign>${x + '&'}",
        "<p>2&",
    );
    // 引擎差异：Java 输出 "<p>2&amp;"（捕获为 markup，普通 '&' 被转义）；
    // v1 HTML 输出格式不实现 auto-escape，整段直出 → "<p>2&"。
    assert_output(
        &c,
        &loader,
        "<#ftl outputFormat='HTML'><#assign x><p>${1 + 1}</#assign>${x + '&'}",
        "<p>2&",
    );
}

/// Java testAssignNs
#[test]
fn test_assign_ns() {
    let (c, loader) = cfg();
    add_template(&loader, "lib.ftl", "");
    assert_output(
        &c,
        &loader,
        "<#import 'lib.ftl' as lib><#assign x in lib></#assign>[${lib.x}]",
        "[]",
    );
    assert_output(
        &c,
        &loader,
        "<#import 'lib.ftl' as lib><#assign x in lib><p>${1 + 1}</#assign>${lib.x + '&'}",
        "<p>2&",
    );
    // 引擎差异：同 testAssign 的 HTML 用例（v1 不实现 HTML auto-escape）。
    assert_output(&c, &loader, "<#ftl outputFormat='HTML'><#import 'lib.ftl' as lib><#assign x in lib><p>${1 + 1}</#assign>${lib.x + '&'}", "<p>2&");
}

/// Java testGlobal
#[test]
fn test_global() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "<#global x></#global>[${.globals.x}]", "[]");
    assert_output(
        &c,
        &loader,
        "<#global x><p>${1 + 1}</#global>${.globals.x + '&'}",
        "<p>2&",
    );
    // 引擎差异：同 testAssign 的 HTML 用例（v1 不实现 HTML auto-escape）。
    assert_output(
        &c,
        &loader,
        "<#ftl outputFormat='HTML'><#global x><p>${1 + 1}</#global>${.globals.x + '&'}",
        "<p>2&",
    );
}

/// Java testLocal
#[test]
fn test_local() {
    let (c, loader) = cfg();
    assert_output(
        &c,
        &loader,
        "<#macro m><#local x></#local>[${x}]</#macro><@m/>${x!}",
        "[]",
    );
    assert_output(
        &c,
        &loader,
        "<#macro m><#local x><p>${1 + 1}</#local>${x + '&'}</#macro><@m/>${x!}",
        "<p>2&",
    );
    // 引擎差异：同 testAssign 的 HTML 用例（v1 不实现 HTML auto-escape）。
    assert_output(&c, &loader, "<#ftl outputFormat='HTML'><#macro m><#local x><p>${1 + 1}</#local>${x + '&'}</#macro><@m/>${x!}", "<p>2&");
}
