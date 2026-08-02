//! Java `freemarker.core.LambdaParsingTest` 的 Rust 1:1 实现
//! （对应 Java: LambdaParsingTest —— lambda 表达式优先级）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testPrecedence：lambda 体内 `||` 优先级低于 `==`
#[test]
fn test_precedence() {
    let (c, loader) = test_config();
    assert_output(
        &c,
        &loader,
        "${[1, 2, 3]?filter(it -> it == 1 || it == 3)?join(', ')}",
        "1, 3",
    );
}
