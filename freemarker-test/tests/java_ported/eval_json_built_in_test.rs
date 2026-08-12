//! 对应 Java: EvalJsonBuiltInTest
//! Java `freemarker.core.EvalJsonBuiltInTest` 的 Rust 1:1 实现。
//!
//! 引擎差异已消除（2026-08-04）：?eval_json 已实现（builtins/strings_misc.rs，
//! Java BuiltInsForStringsMisc.evalJsonBI 语义）——测试已解锁并逐字通过。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java test
#[test]
fn test() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${'1'?eval_json}", "1");
    assert_output(&c, &loader, "${'1'?evalJson}", "1");

    assert_output(&c, &loader, "${'null'?evalJson!'-'}", "-");

    assert_output(
        &c,
        &loader,
        "<#list '{\"a\": 1e2, \"b\": null}'?evalJson as k, v>${k}=${v!'NULL'}<#sep>, </#list>",
        "a=100, b=NULL",
    );
}

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}
