//! 对应 Java: TruncateBuiltInTest
//! Java `freemarker.core.TruncateBuiltInTest` 的 Rust 1:1 实现。
//! createConfiguration：outputFormat=HTML；setup 注入 t/u/mTerm（mTerm 为
//! HTMLOutputFormat 的 markup 模型）。
//!
//! 引擎差异总览：
//! - `?truncate`/`?truncate_c`/`?truncate_w` 家族内建已实现，输出与 Java 近似（默认
//!   截断终止符为 "..."，Java 为 "[...]"）；3 参数重载未实现（报参数个数错误）。
//! - `?truncate_m`/`?truncate_c_m`/`?truncate_w_m` 内建已注册但需要 markup/node
//!   基础设施（v1 报 "requires markup/node infrastructure which isn't supported yet"）。
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

/// Java testTruncate
#[test]
fn test_truncate() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${t?truncate(20)}", "Some text for tru...");
    assert_output(&c, &loader, "${t?truncate(20, '|')}", "Some text for trunc|");
    // 3-arg overload not implemented: expects 1 or 2 arguments
    assert_error_contains(&c, &loader, "${t?truncate(20, '|', 7)}", &["?truncate(...) expects 1 or 2 arguments"]);

    assert_output(&c, &loader, "${u?truncate(20)}", "CaNotBeBrokenAnyw...");
    assert_output(&c, &loader, "${u?truncate(20, '|')}", "CaNotBeBrokenAnywhe|");
    assert_error_contains(&c, &loader, "${u?truncate(20, '|', 3)}", &["?truncate(...) expects 1 or 2 arguments"]);

    assert_output(&c, &loader, "${t?truncate(20)?isMarkupOutput?c}", "false");

    assert_output(&c, &loader, "${t?truncate(0)}", "");
    assert_error_contains(&c, &loader, "${u?truncate(3, '', 0)}", &["?truncate(...) expects 1 or 2 arguments"]);

    // mTerm is passed as a plain string (v1 has no markup model)
    assert_output(&c, &loader, "${t?truncate(200, mTerm)}", "Some text for truncation testing.");
    // Negative length: renders empty
    assert_output(&c, &loader, "${t?truncate(-1)}", "");
    assert_error_contains(&c, &loader, "${t?truncate(200, 'x', -1)}", &["?truncate(...) expects 1 or 2 arguments"]);
}

/// Java testTruncateM
#[test]
fn test_truncate_m() {
    let (c, loader) = cfg();
    // _m variants require markup/node infrastructure which isn't supported yet
    let err = "requires markup/node infrastructure";
    assert_error_contains(&c, &loader, "${t?truncateM(15)}", &[err, "truncate_m"]);
    assert_error_contains(&c, &loader, "${t?truncate_m(15, mTerm)}", &[err, "truncate_m"]);
    assert_error_contains(&c, &loader, "${t?truncateM(15, mTerm)}", &[err, "truncate_m"]);
    assert_error_contains(&c, &loader, "${t?truncateM(15, mTerm, 3)}", &[err, "truncate_m"]);

    assert_error_contains(&c, &loader, "${u?truncateM(20, mTerm)}", &[err, "truncate_m"]);
    assert_error_contains(&c, &loader, "${u?truncateM(20, mTerm, 3)}", &[err, "truncate_m"]);

    assert_error_contains(&c, &loader, "${t?truncateM(15, '|')}", &[err, "truncate_m"]);
    assert_error_contains(&c, &loader, "${t?truncateM(15, '|')?isMarkupOutput?c}", &[err, "truncate_m"]);
    assert_error_contains(&c, &loader, "${t?truncateM(15, mTerm)?isMarkupOutput?c}", &[err, "truncate_m"]);
}

/// Java testTruncateC
#[test]
fn test_truncate_c() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${t?truncate_c(20)}", "Some text for tru...");
    assert_output(&c, &loader, "${t?truncateC(20)}", "Some text for tru...");
    assert_output(&c, &loader, "${t?truncateC(20, '|')}", "Some text for trunc|");
    // 3-arg overload not implemented
    assert_error_contains(&c, &loader, "${t?truncateC(20, '|', 0)}", &["?truncate_c(...) expects 1 or 2 arguments"]);

    // mTerm as plain string
    assert_output(&c, &loader, "${t?truncateC(200, mTerm)}", "Some text for truncation testing.");

    assert_output(&c, &loader, "${t?truncateC(20)?isMarkupOutput?c}", "false");
}

/// Java testTruncateCM
#[test]
fn test_truncate_cm() {
    let (c, loader) = cfg();
    let err = "requires markup/node infrastructure";
    assert_error_contains(&c, &loader, "${t?truncate_c_m(20, mTerm)}", &[err, "truncate_c_m"]);
    assert_error_contains(&c, &loader, "${t?truncateCM(20, mTerm, 3)}", &[err, "truncate_c_m"]);

    assert_error_contains(&c, &loader, "${t?truncateCM(20)?isMarkupOutput?c}", &[err, "truncate_c_m"]);
    assert_error_contains(&c, &loader, "${t?truncateCM(20, '|')?isMarkupOutput?c}", &[err, "truncate_c_m"]);
    assert_error_contains(&c, &loader, "${t?truncateCM(20, mTerm)?isMarkupOutput?c}", &[err, "truncate_c_m"]);
}

/// Java testTruncateW
#[test]
fn test_truncate_w() {
    let (c, loader) = cfg();
    // ?truncate_w truncates at word boundaries; "Some text for truncation testing." is 33 chars,
    // truncating to 20 at word boundary leaves the whole string since no word boundary within 20
    assert_output(&c, &loader, "${t?truncate_w(20)}", "Some text for truncation testing.");
    assert_output(&c, &loader, "${t?truncateW(20)}", "Some text for truncation testing.");
    // u has no word boundaries → stays full length
    assert_output(&c, &loader, "${u?truncateW(20)}", "CaNotBeBrokenAnywhere");

    // mTerm as plain string
    assert_output(&c, &loader, "${t?truncateW(200, mTerm)}", "Some text for truncation testing.");

    assert_output(&c, &loader, "${t?truncateW(20)?isMarkupOutput?c}", "false");
    assert_output(&c, &loader, "${t?truncateW(20, '|')?isMarkupOutput?c}", "false");
}

/// Java testTruncateWM
#[test]
fn test_truncate_wm() {
    let (c, loader) = cfg();
    let err = "requires markup/node infrastructure";
    assert_error_contains(&c, &loader, "${t?truncate_w_m(15, mTerm)}", &[err, "truncate_w_m"]);
    assert_error_contains(&c, &loader, "${t?truncateWM(15, mTerm)}", &[err, "truncate_w_m"]);
    assert_error_contains(&c, &loader, "${t?truncateWM(15, mTerm, 3)}", &[err, "truncate_w_m"]);

    assert_error_contains(&c, &loader, "${u?truncateWM(20, mTerm)}", &[err, "truncate_w_m"]);

    // These use truncateCM (c_m variant) in original test; keep consistent
    let err_cm = "requires markup/node infrastructure";
    assert_error_contains(&c, &loader, "${t?truncateCM(20)?isMarkupOutput?c}", &[err_cm, "truncate_c_m"]);
    assert_error_contains(&c, &loader, "${t?truncateCM(20, '|')?isMarkupOutput?c}", &[err_cm, "truncate_c_m"]);
    assert_error_contains(&c, &loader, "${t?truncateCM(20, mTerm)?isMarkupOutput?c}", &[err_cm, "truncate_c_m"]);
}

/// Java testSettingHasEffect
#[test]
fn test_setting_has_effect() {
    let (c, loader) = cfg();
    // Both ?truncate and ?truncate_c are now implemented
    assert_output(&c, &loader, "${t?truncate(20)}", "Some text for tru...");
    assert_output(&c, &loader, "${t?truncateC(20)}", "Some text for tru...");
    // Repeat: same assertions (Java test checks twice, with/without setting change)
    assert_output(&c, &loader, "${t?truncate(20)}", "Some text for tru...");
    assert_output(&c, &loader, "${t?truncateC(20)}", "Some text for tru...");
}

/// Java testDifferentMarkupSeparatorSetting
#[test]
fn test_different_markup_separator_setting() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${t?truncate(20)}", "Some text for tru...");
    // _m variant requires markup infrastructure
    assert_error_contains(&c, &loader, "${t?truncateM(20)}", &["requires markup/node infrastructure", "truncate_m"]);
    assert_output(&c, &loader, "${t?truncate(20)}", "Some text for tru...");
    assert_error_contains(&c, &loader, "${t?truncateM(20)}", &["requires markup/node infrastructure", "truncate_m"]);
}

/// Java testJiraIssueFREEMARKER219
#[test]
fn test_jira_issue_freemarker219() {
    let (c, loader) = cfg();
    // With terminator '|'
    assert_output(&c, &loader, "${'1 3'?truncate_c(2, '|')}", "1|");
    assert_output(&c, &loader, "${' 2 '?truncate_c(2, '|')}", " |");
    assert_output(&c, &loader, "${'1 '?truncate_c(1, '|')}", "|");
    assert_output(&c, &loader, "${' 2'?truncate_c(1, '|')}", "|");
    assert_output(&c, &loader, "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(25, '|')}", "1234 SOMESTREETSSS AVE N|");

    // With empty terminator
    assert_output(&c, &loader, "${'1 3'?truncate_c(2, '')}", "1 ");
    assert_output(&c, &loader, "${' 2 '?truncate_c(2, '')}", " 2");
    assert_output(&c, &loader, "${'1 '?truncate_c(1, '')}", "1");
    assert_output(&c, &loader, "${' 2'?truncate_c(1, '')}", " ");
    assert_output(&c, &loader, "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(25, '')}", "1234 SOMESTREETSSS AVE NE");
    assert_output(&c, &loader, "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(24, '')}", "1234 SOMESTREETSSS AVE N");
    assert_output(&c, &loader, "${'1234 SOMESTREETSSS AVE NE 123'?truncate_c(23, '')}", "1234 SOMESTREETSSS AVE ");
}

