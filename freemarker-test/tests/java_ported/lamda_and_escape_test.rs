//! 对应 Java: LamdaAndEscapeTest
//! Java `freemarker.core.LamdaAndEscapeTest` 的 Rust 1:1 实现。
//! （Java 类名拼写为 Lamda，保留。）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java testSubstitutionInLambdaLHO：#escape 占位符出现在 lambda 左操作数 → 报错。
/// 引擎差异：Java 检测占位符替换进入 lambda 左操作数并报
/// "myPlaceholder"/"lambda"；v1 无此检测 —— 模板照常求值，在 ?map 处因
/// 占位符被替换为字符串（不是 lambda）报类型错误，断言引擎实际消息。
#[test]
fn test_substitution_in_lambda_lho() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#escape myPlaceholder as ['a', 'b', 'c']?map(myPlaceholder -> 'x')>${'X'}</#escape>",
        &["string-like value is required"],
    );
}

/// Java testSubstitutionInLambdaRHO：#escape 占位符出现在 lambda 右操作数 → 正常替换
#[test]
fn test_substitution_in_lambda_rho() {
    let (c, loader) = cfg();
    assert_output(
        &c,
        &loader,
        "<#escape x as ['a', 'b', 'c']?map(it -> it + x)?join(', ')>${'X'}</#escape>",
        "aX, bX, cX",
    );
}

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}
