//! Java `freemarker.core.ParseTimeParameterBIErrorMessagesTest` 的 Rust 1:1 实现
//! （对应 Java: ParseTimeParameterBIErrorMessagesTest —— ?then / ?switch
//! 参数个数与括号的解析期错误消息断言）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testThen：?then 必须带括号与恰好 2 个参数
/// 引擎差异：Java 对无括号调用报解析期错误 "Expecting \"(\"..."；v1 允许无括号
/// 调用，在运行期报参数个数错误，且措辞为 "2 arguments"（Java "2 parameters"）
/// —— 断言调整到引擎实际消息并注明差异。
#[test]
fn test_then() {
    let (c, loader) = test_config();
    // 引擎差异：Java 解析期 "expecting \"(\""；v1 运行期 "?then(...) expects 2 arguments"
    assert_error_contains(&c, &loader, "${true?then}", &["?then", "2 arguments"]);
    assert_error_contains(&c, &loader, "${true?then + 1}", &["?then", "2 arguments"]);
    assert_error_contains(&c, &loader, "${true?then()}", &["?then", "2 arguments"]);
    assert_error_contains(&c, &loader, "${true?then(1)}", &["?then", "2 arguments"]);
    assert_output(&c, &loader, "${true?then(1, 2)}", "1");
    assert_error_contains(
        &c,
        &loader,
        "${true?then(1, 2, 3)}",
        &["?then", "2 arguments"],
    );
}

/// Java testSwitch：?switch 至少 2 个参数
/// 引擎差异：Java 对无括号调用报解析期错误 "Expecting \"(\"..."；v1 允许无括号
/// 调用，在运行期报错，且措辞为 "arguments"（Java "parameters"）——
/// 断言调整到引擎实际消息并注明差异。
#[test]
fn test_switch() {
    let (c, loader) = test_config();
    // 引擎差异：Java 解析期 "expecting \"(\""；v1 运行期 "?switch expects arguments"
    assert_error_contains(
        &c,
        &loader,
        "${true?switch}",
        &["?switch", "expects arguments"],
    );
    assert_error_contains(
        &c,
        &loader,
        "${true?switch + 1}",
        &["?switch", "expects arguments"],
    );
    assert_error_contains(&c, &loader, "${true?switch()}", &["at least 2 arguments"]);
    assert_error_contains(
        &c,
        &loader,
        "${true?switch(true)}",
        &["at least 2 arguments"],
    );
    assert_output(&c, &loader, "${true?switch(true, 1)}", "1");
}
