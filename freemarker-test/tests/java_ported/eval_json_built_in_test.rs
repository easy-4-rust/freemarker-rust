//! 对应 Java: EvalJsonBuiltInTest
//! Java `freemarker.core.EvalJsonBuiltInTest` 的 Rust 1:1 实现。
//!
//! 引擎差异：`?eval_json`/`?evalJson` 内建在 v1 **未实现**（builtins 注册表与
//! eval.rs 均无）→ 所有断言渲染报 "Unknown built-in: ?eval_json"，
//! Java 断言值全部无法达到（断言原样保留）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java test
#[test]
#[ignore = "引擎差异：?eval_json/?evalJson 内建未实现（v1 报 Unknown built-in），断言保留 Java 原文"]
fn test() {
    let (c, loader) = cfg();
    // 引擎差异：?eval_json 未实现（v1 报 Unknown built-in）。
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
