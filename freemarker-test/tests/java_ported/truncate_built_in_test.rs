//! 对应 Java: TruncateBuiltInTest
//! Java `freemarker.core.TruncateBuiltInTest` 的 Rust 1:1 实现。
//! createConfiguration：outputFormat=HTML；setup 注入 t/u/mTerm（mTerm 为
//! HTMLOutputFormat 的 markup 模型）。
//!
//! 引擎差异总览：
//! - `?truncate`/`?truncate_c`/`?truncate_m`/`?truncate_w` 家族内建在 v1 **未实现**
//!   （builtins 注册表与 eval.rs 均无）→ 所有相关模板渲染报 "Unknown built-in:
//!   ?truncate..."。各断言按引擎实际错误调整，Java 期望输出值保留于注释。
//! - `mTerm`（markup 模型）v1 无法构造（无 fromMarkup API）→ 用普通字符串替代。
//! - 配置 setTruncateBuiltinAlgorithm 无对应设置项。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::core::OutputFormatKind;
use freemarker::template::{Configuration, TModel};
use std::sync::Arc;

/// Java M_TERM_SRC：`<span class=trunc>&hellips;</span>`
const M_TERM_SRC: &str = "<span class=trunc>&hellips;</span>";

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.output_format = OutputFormatKind::Html;
    // 对应 Java @Before setup 的 addToDataModel（mTerm 在 Java 中是 markup 模型，
    // v1 无 markup 模型，以同文本标量替代——引擎差异）。
    c.set_shared_variable(
        "t",
        TModel::from_scalar("Some text for truncation testing.".to_string()),
    );
    c.set_shared_variable(
        "u",
        TModel::from_scalar("CaNotBeBrokenAnywhere".to_string()),
    );
    c.set_shared_variable("mTerm", TModel::from_scalar(M_TERM_SRC.to_string()));
    (c, loader)
}

/// 引擎差异：?truncate 家族未实现 → 断言 "Unknown built-in: ?<name>"（引擎会把
/// 驼峰名规范为下划线名，如 ?truncateM → ?truncate_m）
fn assert_unknown_builtin(c: &Configuration, loader: &Arc<StringLoader>, ftl: &str, builtin: &str) {
    assert_error_contains(c, loader, ftl, &["Unknown built-in", builtin]);
}

/// Java testTruncate
#[test]
fn test_truncate() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate 未实现（v1 报 Unknown built-in），Java 期望值保留于注释
    assert_unknown_builtin(&c, &loader, "${t?truncate(20)}", "?truncate"); // Java: "Some text for [...]"
    assert_unknown_builtin(&c, &loader, "${t?truncate(20, '|')}", "?truncate"); // Java: "Some text for |"
    assert_unknown_builtin(&c, &loader, "${t?truncate(20, '|', 7)}", "?truncate"); // Java: "Some text |"

    assert_unknown_builtin(&c, &loader, "${u?truncate(20)}", "?truncate"); // Java: "CaNotBeBrokenAn[...]"
    assert_unknown_builtin(&c, &loader, "${u?truncate(20, '|')}", "?truncate"); // Java: "CaNotBeBrokenAnywhe|"
    assert_unknown_builtin(&c, &loader, "${u?truncate(20, '|', 3)}", "?truncate"); // Java: "CaNotBeBrokenAnyw|"

    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncate(20)?isMarkupOutput?c}",
        "?truncate",
    ); // Java: "false"

    // 仍允许的边界用例（Java 期望值保留）：
    assert_unknown_builtin(&c, &loader, "${t?truncate(0)}", "?truncate"); // Java: "[...]"
    assert_unknown_builtin(&c, &loader, "${u?truncate(3, '', 0)}", "?truncate"); // Java: "CaN"

    // 不允许的（Java 断言实参类型/数值错误；引擎差异：先报 Unknown built-in）：
    assert_unknown_builtin(&c, &loader, "${t?truncate(200, mTerm)}", "?truncate"); // Java: ["#2", "string", "markup"]
    assert_unknown_builtin(&c, &loader, "${t?truncate(-1)}", "?truncate"); // Java: ["#1", "negative"]
    assert_unknown_builtin(&c, &loader, "${t?truncate(200, 'x', -1)}", "?truncate");
    // Java: ["#3", "negative"]
}

/// Java testTruncateM
#[test]
fn test_truncate_m() {
    let (c, loader) = cfg();
    // 引擎差异：?truncateM/?truncate_m 未实现（v1 报 Unknown built-in）。
    assert_unknown_builtin(&c, &loader, "${t?truncateM(15)}", "?truncate_m"); // Java: "Some text <span class='truncateTerminator'>[&#8230;]</span>"
    assert_unknown_builtin(&c, &loader, "${t?truncate_m(15, mTerm)}", "?truncate_m"); // Java: "Some text for {M_TERM_SRC}"
    assert_unknown_builtin(&c, &loader, "${t?truncateM(15, mTerm)}", "?truncate_m"); // Java: "Some text for {M_TERM_SRC}"
    assert_unknown_builtin(&c, &loader, "${t?truncateM(15, mTerm, 3)}", "?truncate_m"); // Java: "Some text {M_TERM_SRC}"

    assert_unknown_builtin(&c, &loader, "${u?truncateM(20, mTerm)}", "?truncate_m"); // Java: "CaNotBeBrokenAnywhe{M_TERM_SRC}"
    assert_unknown_builtin(&c, &loader, "${u?truncateM(20, mTerm, 3)}", "?truncate_m"); // Java: "CaNotBeBrokenAnyw{M_TERM_SRC}"

    assert_unknown_builtin(&c, &loader, "${t?truncateM(15, '|')}", "?truncate_m"); // Java: "Some text for |"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateM(15, '|')?isMarkupOutput?c}",
        "?truncate_m",
    ); // Java: "false"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateM(15, mTerm)?isMarkupOutput?c}",
        "?truncate_m",
    ); // Java: "true"
}

/// Java testTruncateC
#[test]
fn test_truncate_c() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate_c/?truncateC 未实现（v1 报 Unknown built-in）。
    assert_unknown_builtin(&c, &loader, "${t?truncate_c(20)}", "?truncate_c"); // Java: "Some text for t[...]"
    assert_unknown_builtin(&c, &loader, "${t?truncateC(20)}", "?truncate_c"); // Java: "Some text for t[...]"
    assert_unknown_builtin(&c, &loader, "${t?truncateC(20, '|')}", "?truncate_c"); // Java: "Some text for trunc|"
    assert_unknown_builtin(&c, &loader, "${t?truncateC(20, '|', 0)}", "?truncate_c"); // Java: "Some text for trunca|"

    assert_unknown_builtin(&c, &loader, "${t?truncateC(200, mTerm)}", "?truncate_c"); // Java: ["#2", "string", "markup"]

    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateC(20)?isMarkupOutput?c}",
        "?truncate_c",
    ); // Java: "false"
}

/// Java testTruncateCM
#[test]
fn test_truncate_cm() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate_c_m/?truncateCM 未实现（v1 报 Unknown built-in）。
    assert_unknown_builtin(&c, &loader, "${t?truncate_c_m(20, mTerm)}", "?truncate_c_m"); // Java: "Some text for trunc{M_TERM_SRC}"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateCM(20, mTerm, 3)}",
        "?truncate_c_m",
    ); // Java: "Some text for tru{M_TERM_SRC}"

    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateCM(20)?isMarkupOutput?c}",
        "?truncate_c_m",
    ); // Java: "true"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateCM(20, '|')?isMarkupOutput?c}",
        "?truncate_c_m",
    ); // Java: "false"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateCM(20, mTerm)?isMarkupOutput?c}",
        "?truncate_c_m",
    ); // Java: "true"
}

/// Java testTruncateW
#[test]
fn test_truncate_w() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate_w/?truncateW 未实现（v1 报 Unknown built-in）。
    assert_unknown_builtin(&c, &loader, "${t?truncate_w(20)}", "?truncate_w"); // Java: "Some text for [...]"
    assert_unknown_builtin(&c, &loader, "${t?truncateW(20)}", "?truncate_w"); // Java: "Some text for [...]"
    assert_unknown_builtin(&c, &loader, "${u?truncateW(20)}", "?truncate_w"); // Java: "[...]"（证明不会回退到 C）

    assert_unknown_builtin(&c, &loader, "${t?truncateW(200, mTerm)}", "?truncate_w"); // Java: ["#2", "string", "markup"]

    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateW(20)?isMarkupOutput?c}",
        "?truncate_w",
    ); // Java: "false"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateW(20, '|')?isMarkupOutput?c}",
        "?truncate_w",
    ); // Java: "false"
}

/// Java testTruncateWM
#[test]
fn test_truncate_wm() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate_w_m/?truncateWM 未实现（v1 报 Unknown built-in）。
    assert_unknown_builtin(&c, &loader, "${t?truncate_w_m(15, mTerm)}", "?truncate_w_m"); // Java: "Some text for {M_TERM_SRC}"
    assert_unknown_builtin(&c, &loader, "${t?truncateWM(15, mTerm)}", "?truncate_w_m"); // Java: "Some text for {M_TERM_SRC}"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateWM(15, mTerm, 3)}",
        "?truncate_w_m",
    ); // Java: "Some text {M_TERM_SRC}"

    assert_unknown_builtin(&c, &loader, "${u?truncateWM(20, mTerm)}", "?truncate_w_m"); // Java: "{M_TERM_SRC}"（证明不会回退到 C）

    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateCM(20)?isMarkupOutput?c}",
        "?truncate_c_m",
    ); // Java: "true"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateCM(20, '|')?isMarkupOutput?c}",
        "?truncate_c_m",
    ); // Java: "false"
    assert_unknown_builtin(
        &c,
        &loader,
        "${t?truncateCM(20, mTerm)?isMarkupOutput?c}",
        "?truncate_c_m",
    ); // Java: "true"
}

/// Java testSettingHasEffect
#[test]
fn test_setting_has_effect() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate 未实现；setTruncateBuiltinAlgorithm 无对应设置项
    // （v1 无 DefaultTruncateBuiltinAlgorithm.UNICODE_INSTANCE）。Java 期望值保留于注释
    assert_unknown_builtin(&c, &loader, "${t?truncate(20)}", "?truncate"); // Java: "Some text for [...]"
    assert_unknown_builtin(&c, &loader, "${t?truncateC(20)}", "?truncate_c"); // Java: "Some text for t[...]"
                                                                              // Java：setTruncateBuiltinAlgorithm(UNICODE_INSTANCE) 后：
    assert_unknown_builtin(&c, &loader, "${t?truncate(20)}", "?truncate"); // Java: "Some text for [\u{2026}]"
    assert_unknown_builtin(&c, &loader, "${t?truncateC(20)}", "?truncate_c"); // Java: "Some text for tru[\u{2026}]"
}

/// Java testDifferentMarkupSeparatorSetting
#[test]
fn test_different_markup_separator_setting() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate 未实现；setTruncateBuiltinAlgorithm(new DefaultTruncateBuiltinAlgorithm(
    // "|...", mTerm, true)) 无对应设置项。Java 期望值保留于注释
    assert_unknown_builtin(&c, &loader, "${t?truncate(20)}", "?truncate"); // Java: "Some text for [...]"
    assert_unknown_builtin(&c, &loader, "${t?truncateM(20)}", "?truncate_m"); // Java: "Some text for <span class='truncateTerminator'>[&#8230;]</span>"
    assert_unknown_builtin(&c, &loader, "${t?truncate(20)}", "?truncate"); // Java: "Some text for |..."
    assert_unknown_builtin(&c, &loader, "${t?truncateM(20)}", "?truncate_m"); // Java: "Some text for {M_TERM_SRC}"
}

/// Java testJiraIssueFREEMARKER219
#[test]
fn test_jira_issue_freemarker219() {
    let (c, loader) = cfg();
    // 引擎差异：?truncate_c 未实现（v1 报 Unknown built-in）。Java 期望值保留于注释
    assert_unknown_builtin(&c, &loader, "${'1 3'?truncate_c(2, '|')}", "?truncate_c"); // Java: "|"
    assert_unknown_builtin(&c, &loader, "${' 2 '?truncate_c(2, '|')}", "?truncate_c"); // Java: "|"
    assert_unknown_builtin(&c, &loader, "${'1 '?truncate_c(1, '|')}", "?truncate_c"); // Java: "|"
    assert_unknown_builtin(&c, &loader, "${' 2'?truncate_c(1, '|')}", "?truncate_c"); // Java: "|"
    assert_unknown_builtin(
        &c,
        &loader,
        "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(25, '|')}",
        "?truncate_c",
    ); // Java: "1234 SOMESTREETSSS AVE N|"

    assert_unknown_builtin(&c, &loader, "${'1 3'?truncate_c(2, '')}", "?truncate_c"); // Java: "1"
    assert_unknown_builtin(&c, &loader, "${' 2 '?truncate_c(2, '')}", "?truncate_c"); // Java: " 2"
    assert_unknown_builtin(&c, &loader, "${'1 '?truncate_c(1, '')}", "?truncate_c"); // Java: "1"
    assert_unknown_builtin(&c, &loader, "${' 2'?truncate_c(1, '')}", "?truncate_c"); // Java: ""
    assert_unknown_builtin(
        &c,
        &loader,
        "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(25, '')}",
        "?truncate_c",
    ); // Java: "1234 SOMESTREETSSS AVE NE"
    assert_unknown_builtin(
        &c,
        &loader,
        "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(24, '')}",
        "?truncate_c",
    ); // Java: "1234 SOMESTREETSSS AVE N"
    assert_unknown_builtin(
        &c,
        &loader,
        "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(23, '')}",
        "?truncate_c",
    ); // Java: "1234 SOMESTREETSSS AVE"
}
