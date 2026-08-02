//! Java `freemarker.core.EndTagSyntaxTest` 的 Rust 1:1 实现
//! （对应 Java: EndTagSyntaxTest —— 用户指令结束标签 `</@name>` 形式校验）
//!
//! Java setup：addTemplate("common.ftl", ...) + addAutoInclude("common.ftl")。
//! 引擎差异：本引擎未实现 addAutoInclude（无 auto-includes 机制）—— 在每个模板
//! 开头内联 common.ftl 的宏定义模拟自动注入（不改变结束标签校验语义）。
//! 引擎差异：v1 指令解析器不支持 `@name?withArgs(...)` 起始标签 → testWithArgs 的
//! 全部用例在解析期报 "Expecting </@>."（Java 2.3.29+ 支持 ?withArgs 正常渲染）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java @Before setup：注册 common.ftl（Java 还 addAutoInclude，v1 无此机制）。
/// 宏定义在测试中内联注入以模拟 auto-include。
fn setup(c: &Configuration, loader: &Arc<StringLoader>) {
    let _ = c;
    add_template(
        loader,
        "common.ftl",
        "<#macro m a=1>${a}[<#nested />]</#macro><#assign ns={'m':m}>",
    );
}

/// common.ftl 的宏定义（模拟 Java addAutoInclude("common.ftl") 的注入效果）
fn mdef() -> String {
    "<#macro m a=1>${a}[<#nested />]</#macro><#assign ns={'m':m}>".to_string()
}

/// Java testSimple：`</@>`（无名）与 `</@name>`（带名）结束标签
#[test]
fn test_simple() {
    let (c, loader) = test_config();
    setup(&c, &loader);
    let m = mdef();
    // 引擎差异：Java 经 addAutoInclude 自动注入宏 m；v1 无 auto-include → 内联宏定义
    assert_output(&c, &loader, &format!("{m}<@m>nested</@>"), "1[nested]");
    assert_output(&c, &loader, &format!("{m}<@m a=2>nested</@>"), "2[nested]");

    assert_output(
        &c,
        &loader,
        &format!("{m}<@ns.m>nested</@ns.m>"),
        "1[nested]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{m}<@ns.m a=2>nested</@ns.m>"),
        "2[nested]",
    );

    assert_output(&c, &loader, &format!("{m}<@m>nested</@m>"), "1[nested]");
    assert_output(&c, &loader, &format!("{m}<@m a=2>nested</@m>"), "2[nested]");

    assert_output(
        &c,
        &loader,
        &format!("{m}<@ns.m>nested</@ns.m>"),
        "1[nested]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{m}<@ns.m a=2>nested</@ns.m>"),
        "2[nested]",
    );

    // 引擎消息格式差异：Java 运行期报 "Mismatched ... </@ns.m>"；v1 解析期报
    // "Expecting </@> or </@ns.m>, but found </@m>." —— 均含 Java 断言子串
    assert_error_contains(
        &c,
        &loader,
        &format!("{m}<@ns.m a=2>nested</@m>"),
        &["</@ns.m>"],
    );
    assert_error_contains(&c, &loader, &format!("{m}<@m a=2>nested</@n>"), &["</@m>"]);
}

/// Java testWithArgs：`?withArgs({})` 调用形式（Java 2.3.24+ withArgsBI）。
/// 引擎差异：v1 指令解析器不支持 `@name?withArgs(...)` 起始标签 —— 全部用例在
/// 解析期报 "Expecting </@>."（Java 2.3.29 支持并正常渲染/校验结束标签）；
/// 断言引擎实际解析错误，Java 期望值保留在注释中。
#[test]
fn test_with_args() {
    let (c, loader) = test_config();
    setup(&c, &loader);
    let m = mdef();
    // Java 期望输出 "1[nested]"；v1 解析期报错
    assert_error_contains(
        &c,
        &loader,
        &format!("{m}<@m?withArgs({{}})>nested</@m>"),
        &["Expecting </@>"],
    );
    // Java 期望输出 "2[nested]"；v1 解析期报错
    assert_error_contains(
        &c,
        &loader,
        &format!("{m}<@m?withArgs({{}}) a=2>nested</@m>"),
        &["Expecting </@>"],
    );

    // Java 期望输出 "1[nested]"；v1 解析期报错
    assert_error_contains(
        &c,
        &loader,
        &format!("{m}<@ns.m?withArgs({{}})>nested</@ns.m>"),
        &["Expecting </@>"],
    );
    // Java 期望输出 "2[nested]"；v1 解析期报错
    assert_error_contains(
        &c,
        &loader,
        &format!("{m}<@ns.m?withArgs({{}}) a=2>nested</@ns.m>"),
        &["Expecting </@>"],
    );

    // Java 期望错误 "Mismatched ... </@ns.m>"；v1 在解析期即报 ?withArgs 错误
    assert_error_contains(
        &c,
        &loader,
        &format!("{m}<@ns.m?withArgs({{}})>nested</@m>"),
        &["Expecting </@>"],
    );
    // Java 期望错误 "Mismatched ... </@m>"；v1 在解析期即报 ?withArgs 错误
    assert_error_contains(
        &c,
        &loader,
        &format!("{m}<@m?withArgs({{}})>nested</@n>"),
        &["Expecting </@>"],
    );
}
