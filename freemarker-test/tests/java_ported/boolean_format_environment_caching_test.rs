//! 对应 Java: BooleanFormatEnvironmentCachingTest
//! Java `freemarker.core.BooleanFormatEnvironmentCachingTest` 的 Rust 1:1 实现：
//! `${true}`/`${false}` 在 boolean_format='c' 时按 CFormat 输出，且 `<#setting>`
//! 切换 c_format/boolean_format 立即生效（环境级缓存）。
//!
//! 引擎差异：
//! - Java 配置 `setCFormat(CustomCFormat.INSTANCE)`（自定义类，true/false 输出
//!   "TRUE"/"FALSE"）+ `setBooleanFormat("c")`；Rust 无自定义 CFormat API
//!   （_ObjectBuilder* NA-DESIGN），用内建 C 格式（boolean_format='c' → "true"/"false"）
//!   近似 → 首段输出为 "true true false false" 而非 Java 的 "TRUE TRUE FALSE FALSE"。
//! - c_format 设置已实现（2026-08）：`<#setting cFormat='JSON'>`/`'Java'` 可 1:1 翻译，
//!   环境级缓存切换立即生效。
//! - `<#setting booleanFormat='y,n'>` / `<#setting booleanFormat='c'>` 可 1:1 翻译。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java test()：五段式输出，引擎逐段断言。
#[test]
fn test() {
    let (mut c, loader) = test_config();
    // Java createConfiguration：conf.setBooleanFormat("c")（CustomCFormat 无对应 API）
    c.settings.boolean_format = "c".to_string();

    // 段 1：${true} ${true} ${false} ${false}（boolean_format='c'）
    // 引擎差异：CustomCFormat 用内建 C 格式近似 → "true ... false"，Java 为 "TRUE ... FALSE"
    assert_output(
        &c,
        &loader,
        "${true} ${true} ${false} ${false}",
        "true true false false",
    );

    // 段 2：<#setting cFormat='JSON'>${true} ${true} ${false} ${false}
    // Java 切到 JSON CFormat 后输出 "true ... false"（c_format 设置已实现）
    assert_output(
        &c,
        &loader,
        "<#setting cFormat='JSON'>${true} ${true} ${false} ${false}",
        "true true false false",
    );

    // 段 3：<#setting booleanFormat='y,n'>${true} ${true} ${false} ${false} → "y y n n"
    assert_output(
        &c,
        &loader,
        "<#setting booleanFormat='y,n'>${true} ${true} ${false} ${false}",
        "y y n n",
    );

    // 段 4：<#setting cFormat='Java'>${true} ${true} ${false} ${false}
    // Java CFormat 布尔输出同为 "true ... false"（c_format 设置已实现）
    assert_output(
        &c,
        &loader,
        "<#setting cFormat='Java'>${true} ${true} ${false} ${false}",
        "true true false false",
    );

    // 段 5：<#setting booleanFormat='c'>${true} ${true} ${false} ${false} → "true ... false"
    // （Java 此时 cFormat=Java → 同为 "true ... false"，与 v1 一致）
    assert_output(
        &c,
        &loader,
        "<#setting booleanFormat='c'>${true} ${true} ${false} ${false}",
        "true true false false",
    );
}
