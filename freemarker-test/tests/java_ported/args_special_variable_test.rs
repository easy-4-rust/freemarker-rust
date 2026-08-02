//! 对应 Java: ArgsSpecialVariableTest
//! Java `freemarker.core.ArgsSpecialVariableTest` 的 Rust 1:1 实现。
//!
//! 引擎差异：`.args` 特殊变量（宏/函数实参哈希）在 v1 **不支持**
//! （eval.rs BuiltinVar::Args → "The .args special variable (macro arguments hash)
//! is not supported by this implementation."）→ 本文件所有渲染断言无法达到，
//! Java 断言值原样保留。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java macroSimpleTest
#[test]
fn macro_simple_test() {
    let (c, loader) = cfg();
    let macro_def = "<#macro m a b><#list .args as k, v>${k}=${v}<#sep>, </#list></#macro>";
    let expected_output = "a=11, b=22";
    // 引擎差异：.args 不支持（v1 报错）。
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m a=11 b=22 />"),
        expected_output,
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m 11 22 />"),
        expected_output,
    );
}

/// Java macroZeroArgsTest
#[test]
fn macro_zero_args_test() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "<#macro m>${.args?size}</#macro><@m />", "0");
    assert_output(
        &c,
        &loader,
        "<#macro m others...>${.args?size}</#macro><@m />",
        "0",
    );
}

/// Java macroWithDefaultsTest
#[test]
fn macro_with_defaults_test() {
    let (c, loader) = cfg();
    let macro_def = "<#macro m a b c=3><#list .args as k, v>${k}=${v}<#sep>, </#list></#macro>";
    let expected_output = "a=11, b=22, c=33; a=11, b=22, c=3";
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m a=11 b=22 c=33 />; <@m a=11 b=22 />"),
        expected_output,
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m 11 22 33 />; <@m 11 22 />"),
        expected_output,
    );
}

/// Java macroWithMultiPassDefaultsTest
#[test]
fn macro_with_multi_pass_defaults_test() {
    let (c, loader) = cfg();
    let macro_def = "<#macro m a=c b=c c=b><#list .args as k, v>${k}=${v}<#sep>, </#list></#macro>";
    let expected_output = "a=33, b=33, c=33; a=22, b=22, c=22; a=11, b=33, c=33; a=11, b=22, c=22";
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m c=33 />; <@m b=22 />; <@m a=11 c=33 />; <@m a=11 b=22 />"),
        expected_output,
    );
    assert_output(&c, &loader, &format!("{macro_def}<@m null, null, 33 />; <@m null, 22, null />; <@m 11, null, 33 />; <@m 11, 22, null />"), expected_output);
}

/// Java macroWithCatchAllTest
#[test]
fn macro_with_catch_all_test() {
    let (c, loader) = cfg();
    let macro_def =
        "<#macro m a b=2 others...><#list .args as k, v>${k}=${v}<#sep>, </#list></#macro>";
    assert_output(&c, &loader, &format!("{macro_def}<@m a=11 b=22 c=33 d=44 />; <@m a=11 b=22 />; <@m a=11 />; <@m a=11 c=33 />"),
        "a=11, b=22, c=33, d=44; a=11, b=22; a=11, b=2; a=11, b=2, c=33");

    assert_output(&c, &loader, &format!("{macro_def}<@m 1, 2 />"), "a=1, b=2");
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m 1, 2, 3 />"),
        &[".args", "catch-all"],
    );
}

/// Java functionSimpleTest
#[test]
fn function_simple_test() {
    let (c, loader) = cfg();
    let function_def = "<#function f a b><#return .args?join(', ')></#function>";
    let expected_output = "11, 22";
    assert_output(
        &c,
        &loader,
        &format!("{function_def}${{f(11, 22)}}"),
        expected_output,
    );
}

/// Java functionZeroArgsTest
#[test]
fn function_zero_args_test() {
    let (c, loader) = cfg();
    assert_output(
        &c,
        &loader,
        "<#function f><#return .args?size></#function>${f()}",
        "0",
    );
    assert_output(
        &c,
        &loader,
        "<#function f others...><#return .args?size></#function>${f()}",
        "0",
    );
}

/// Java functionWithDefaultsTest
#[test]
fn function_with_defaults_test() {
    let (c, loader) = cfg();
    let function_def = "<#function f a b c=3><#return .args?join(', ')></#function>";
    let expected_output = "11, 22, 33; 11, 22, 3";
    assert_output(
        &c,
        &loader,
        &format!("{function_def}${{f(11, 22, 33)}}; ${{f(11, 22)}}"),
        expected_output,
    );
}

/// Java functionWithMultiPassDefaultsTest
#[test]
fn function_with_multi_pass_defaults_test() {
    let (c, loader) = cfg();
    let function_def = "<#function f a=c b=c c=b><#return .args?join(', ')></#function>";
    assert_output(&c, &loader, &format!("{function_def}${{f(null, null, 33)}}; ${{f(null, 22)}}; ${{f(11, null, 33)}}; ${{f(11, 22)}}"),
        "33, 33, 33; 22, 22, 22; 11, 33, 33; 11, 22, 22");
    assert_output(
        &c,
        &loader,
        &format!("{function_def}${{f(11, 22)}}; ${{f(11, 22, 33)}}"),
        "11, 22, 22; 11, 22, 33",
    );
}

/// Java functionWithCatchAllTest
#[test]
fn function_with_catch_all_test() {
    let (c, loader) = cfg();
    assert_output(&c, &loader,
        "<#function f a b=2 others...><#return .args?join(', ')></#function>${f(11, 22, 33, 44)}; ${f(11, 22)}; ${f(11)}; ${f(11, null, 33)}",
        "11, 22, 33, 44; 11, 22; 11, 2; 11, 2, 33");
}

/// Java usedInWrongContextTest
#[test]
fn used_in_wrong_context_test() {
    let (c, loader) = cfg();
    // 引擎差异：Java 在宏外使用 .args 报 "args"/"macro"/"function" 提示；
    // v1 一律报 "not supported by this implementation"，Java 子串保留。
    assert_error_contains(&c, &loader, "${.args}", &["args", "macro", "function"]);
    assert_error_contains(
        &c,
        &loader,
        "<#macro m>${'.args'?eval}</#macro><@m />",
        &["args", "macro", "function"],
    );
}
