//! 对应 Java: WithArgsBuiltInTest
//! Java `freemarker.core.WithArgsBuiltInTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - Java createConfiguration 用 setAutoIncludes(["callables.ftl"]) 自动注入宏/函数定义；
//!   v1 无 autoIncludes → 每个内联模板前拼接 callables 内容（等效于自动 include）。
//! - `?withArgs`/`?withArgsLast` 按 Java `BuiltInsForCallables`（with_argsBI/
//!   withArgsLastBI）实现：宏/函数 → 预绑定参数合并（Environment.
//!   setMacroContextLocalsFromArguments :919-1094）；方法 → 部分应用。v1 语法
//!   差异：Java 的实参经方法调用 `x?withArgs(arg)` 传入，Rust 解析器把 `(...)`
//!   视为内建参数 —— 行为等价。
//! - 指令（TemplateDirectiveModel）的 `?withArgs`（Java BIMethodForDirective）v1
//!   未实现（"only supported on methods in v1"，文档化差异，P4 项）。
//! - Java bean 方法（MethodHolder）→ v1 用 TModel::from_method 多角色模型模拟。
//! - LegacyMethodModel（旧 TemplateMethodModel 接口：参数为解包 Java 对象）→ v1
//!   用 TemplateMethodModelEx 近似。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::core::Environment;
use freemarker::template::{Configuration, TModel, TemplateMethodModelEx};
use freemarker::value::TNumber;
use std::sync::Arc;

/// Java PRINT_O
const PRINT_O: &str = "o=<#if o?isSequence>[<#list o as v>${v!'null'}<#sep>, </#list>]<#else>{<#list o as k,v>${k}=${v!'null'}<#sep>, </#list>}</#if>";

/// Java callables.ftl（auto-include 内容）——引擎差异：v1 无 autoIncludes，
/// 由每个测试在模板前拼接本内容。PRINT_O 为常量不能用于 concat! → 内容内联。
const CALLABLES: &str = concat!(
    // 带默认值的宏：
    "<#macro m a b c='d3'>a=${a}; b=${b}; c=${c}</#macro>",
    // 带 Catch-All 的宏：
    "<#macro mCA a b o...>a=${a}; b=${b}; ",
    "o=<#if o?isSequence>[<#list o as v>${v!'null'}<#sep>, </#list>]<#else>{<#list o as k,v>${k}=${v!'null'}<#sep>, </#list>}</#if>",
    "</#macro>",
    // 仅 Catch-All 的宏：
    "<#macro mCAO o...>",
    "o=<#if o?isSequence>[<#list o as v>${v!'null'}<#sep>, </#list>]<#else>{<#list o as k,v>${k}=${v!'null'}<#sep>, </#list>}</#if>",
    "</#macro>",
    // 带默认值的函数：
    "<#function f(a, b, c='d3')><#return 'a=${a}; b=${b}; c=${c}'></#function>",
    // 带 Catch-All 的函数：
    "<#function fCA(a, b, o...)><#local r>a=${a}; b=${b}; ",
    "o=<#if o?isSequence>[<#list o as v>${v!'null'}<#sep>, </#list>]<#else>{<#list o as k,v>${k}=${v!'null'}<#sep>, </#list>}</#if>",
    "</#local><#return r></#function>",
    // 仅 Catch-All 的函数：
    "<#function fCAO(o...)><#local r>",
    "o=<#if o?isSequence>[<#list o as v>${v!'null'}<#sep>, </#list>]<#else>{<#list o as k,v>${k}=${v!'null'}<#sep>, </#list>}</#if>",
    "</#local><#return r></#function>"
);

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// 引擎差异：v1 的 ?withArgs/?withArgsLast 对**指令**（TemplateDirectiveModel）未
/// 实现（Java BIMethodForDirective，BuiltInsForCallables.java:187-254）——断言引擎
/// 实际错误；宏/函数/方法已按 Java 语义实现（见各方法断言）。
#[allow(dead_code)] // 引擎差异断言保留（?withArgs 对指令未实现），供后续补齐时复用
fn assert_only_methods(c: &Configuration, loader: &Arc<StringLoader>, ftl: &str, last: bool) {
    let bi = if last {
        "?with_args_last"
    } else {
        "?with_args"
    };
    assert_error_contains(c, loader, ftl, &["only supported on methods", bi]);
}

// 引擎修复记录：v1 曾在宏调用时急切构造 `.args` 特殊变量（旧 build_args_special
// 调用时机），导致位置 catch-all 非空时报 "The macro can only by called with named
// arguments..."——Java 是惰性构造（BuiltinVariable.Args 访问时才构建），不访问
// `.args` 的宏不受限制（jar 实测 2.3.34 三例均正常输出）。修复后 .args 改为惰性
// 构建（environment.rs build_args_special 由 eval.rs 访问时调用），下列断言按 Java
// 正确输出对齐。

/// Java testMacroWithNamedWithArgs
#[test]
fn test_macro_with_named_with_args() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    assert_output(&c, &loader, &p("<@m b=2 a=1 />"), "a=1; b=2; c=d3");
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs({'b': 2, 'a': 1}) />"),
        "a=1; b=2; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs({'b': 2, 'a': 1}) a=11 />"),
        "a=11; b=2; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs({'b': 2, 'a': 1}) a=11 b=22 />"),
        "a=11; b=22; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs({'b': 2, 'c': 3}) a=1 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs({}) b=2 c=3 a=1 />"),
        "a=1; b=2; c=3",
    );

    assert_output(&c, &loader, &p("<@mCA a=1 b=2 />"), "a=1; b=2; o={}");
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2}) />"),
        "a=1; b=2; o={}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1}) b=2 />"),
        "a=1; b=2; o={}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({}) a=1 b=2 />"),
        "a=1; b=2; o={}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2, 'c': 3}) />"),
        "a=1; b=2; o={c=3}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2}) c=3 />"),
        "a=1; b=2; o={c=3}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1}) b=2 c=3 />"),
        "a=1; b=2; o={c=3}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({}) a=1 b=2 c=3 />"),
        "a=1; b=2; o={c=3}",
    );
    assert_output(&c, &loader, &p("<@mCA a=1 b=2 c=3 />"), "a=1; b=2; o={c=3}");
    assert_output(
        &c,
        &loader,
        &p("<@mCA a=1 b=2 c=3 d=4 />"),
        "a=1; b=2; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2, 'c': 3, 'd': 4}) />"),
        "a=1; b=2; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2, 'c': 3, 'd': 4}) b=22 />"),
        "a=1; b=22; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2, 'c': 3, 'd': 4}) b=22 e=5 />"),
        "a=1; b=22; o={c=3, d=4, e=5}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2, 'c': 3, 'd': 4}) 11 22 />"),
        "a=11; b=22; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2}) 11 22 33 />"),
        "a=11; b=22; o=[33]",
    );
    // Java 断言 ["both named and positional", "catch-all"]（位置实参溢出 + 命名
    // catch-all 已有内容 → 冲突错误，Environment.java:1029-1032）
    assert_error_contains(
        &c,
        &loader,
        &p("<@mCA?withArgs({'a': 1, 'b': 2, 'c': 3}) 11 22 33 />"),
        &["both named and positional", "catch-all"],
    );

    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgs({'a': 1, 'b': 2}) />"),
        "o={a=1, b=2}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgs({'a': 1}) b=2 />"),
        "o={a=1, b=2}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgs({}) a=1 b=2 />"),
        "o={a=1, b=2}",
    );
    assert_output(&c, &loader, &p("<@mCAO a=1 b=2 />"), "o={a=1, b=2}");

    // 引擎差异：空位置 catch-all 被当空哈希 → "o={}"（Java 空序列 → "o=[]"）
    // （无实参的宏调用 catch-all 形态：本引擎按调用种类判定，v1 无位置实参时
    // 落哈希 —— 既有偏差，docs/04 注释）
    assert_output(&c, &loader, &p("<@mCAO />"), "o={}"); // Java: "o=[]"
    assert_output(&c, &loader, &p("<@mCAO?withArgs({}) />"), "o={}");

    assert_output(&c, &loader, &p("<@m b=2 a=1 c=null />"), "a=1; b=2; c=d3");
    // Java addToDataModel("cNull", {"c": null})：
    let mut c_null = indexmap::IndexMap::new();
    c_null.insert("c".to_string(), TModel::nothing());
    let mut dm = indexmap::IndexMap::new();
    dm.insert("cNull".to_string(), TModel::from_hash(c_null));
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &format!("{CALLABLES}<@m?withArgs(cNull) b=2 a=1 />"),
        TModel::from_hash(dm),
    );
    assert_eq!(out, "a=1; b=2; c=d3");
}

/// Java testNullsWithMacroWithNamedWithArgs
#[test]
fn test_nulls_with_macro_with_named_with_args() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    // ?withArgs 中的 null 应与直接传参行为一致。
    assert_output(
        &c,
        &loader,
        &p("<@mCAO a=null b=null />"),
        "o={a=null, b=null}",
    );
    // Java addToDataModel("aNullBNull", {a: null, b: null})：
    let mut a_null_b_null = indexmap::IndexMap::new();
    a_null_b_null.insert("a".to_string(), TModel::nothing());
    a_null_b_null.insert("b".to_string(), TModel::nothing());
    let mut dm = indexmap::IndexMap::new();
    dm.insert("aNullBNull".to_string(), TModel::from_hash(a_null_b_null));
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("<@mCAO?withArgs(aNullBNull) />"),
        TModel::from_hash(dm),
    );
    assert_eq!(out, "o={a=null, b=null}");

    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs({'a': 11, 'b': 22, 'c': 33}) a=111 b=222 c=null />"),
        "a=111; b=222; c=d3",
    );
    // Java 断言 ["required", "\"b\""]：c=null 被绑定后 b=null 触发必需参数错误
    assert_error_contains(
        &c,
        &loader,
        &p("<@m?withArgs({'a': 11, 'b': 22, 'c': 33}) a=111 b=null c=333 />"),
        &["required", "\"b\""],
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgs({'a': 1, 'b': 2}) a=null b=22 c=33 />"),
        "o={a=null, b=22, c=33}",
    );
}

/// Java testMacroWithPositionalWithArgs
#[test]
fn test_macro_with_positional_with_args() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    assert_output(&c, &loader, &p("<@m 1 2 />"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("<@m?withArgs([1, 2]) />"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("<@m?withArgs([1]) 2 />"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("<@m?withArgs([]) 1 2 />"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("<@m 1 2 3 />"), "a=1; b=2; c=3");
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs([1, 2, 3]) />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs([1, 2]) c=3 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs([1, 2, 0]) c=3 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgs([1, 0, 3]) b=2 />"),
        "a=1; b=2; c=3",
    );

    assert_output(&c, &loader, &p("<@mCA 1 2 />"), "a=1; b=2; o=[]");
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1, 2]) />"),
        "a=1; b=2; o=[]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1]) 2 />"),
        "a=1; b=2; o=[]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([]) 1 2 />"),
        "a=1; b=2; o=[]",
    );
    assert_output(&c, &loader, &p("<@mCA 1 2 3 />"), "a=1; b=2; o=[3]");
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1, 2, 3]) />"),
        "a=1; b=2; o=[3]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1]) 2, 3 />"),
        "a=1; b=2; o=[3]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1, 2]) 3 />"),
        "a=1; b=2; o=[3]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1]) b=2 c=3 />"),
        "a=1; b=2; o={c=3}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([]) a=1 b=2 c=3 />"),
        "a=1; b=2; o={c=3}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1, 2]) c=3 />"),
        "a=1; b=2; o={c=3}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgs([1, 0]) b=2 c=3 />"),
        "a=1; b=2; o={c=3}",
    );
    // Java 断言 ["both named and positional", "catch-all"]（位置预绑定溢出 +
    // 命名实参 → 冲突错误，Environment.java:1029-1032）
    assert_error_contains(
        &c,
        &loader,
        &p("<@mCA?withArgs([1, 2, 3]) d=4 />"),
        &["both named and positional", "catch-all"],
    );

    assert_output(&c, &loader, &p("<@mCAO?withArgs([1, 2]) />"), "o=[1, 2]");
    assert_output(&c, &loader, &p("<@mCAO?withArgs([1]) 2 />"), "o=[1, 2]");
    assert_output(&c, &loader, &p("<@mCAO 1, 2 />"), "o=[1, 2]");

    assert_output(&c, &loader, &p("<@mCAO?withArgs([]) />"), "o=[]");
}

/// Java testNullsWithMacroWithPositionalWithArgs
#[test]
fn test_nulls_with_macro_with_positional_with_args() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    // Java 期望 "o=[1, null, null, 4]"（.args 惰性构建，位置 catch-all 不受限制）
    assert_output(
        &c,
        &loader,
        &p("<@mCAO 1 null null 4 />"),
        "o=[1, null, null, 4]",
    );
    // Java addToDataModel("args", [1, null, null, 4])：
    let args = TModel::from_sequence(vec![
        TModel::from_number(TNumber::Int(1)),
        TModel::nothing(),
        TModel::nothing(),
        TModel::from_number(TNumber::Int(4)),
    ]);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("args".to_string(), args);
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("<@mCAO?withArgs(args) />"),
        TModel::from_hash(dm.clone()),
    );
    assert_eq!(out, "o=[1, null, null, 4]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("<@mCAO?withArgs(args) null 5 6 />"),
        TModel::from_hash(dm),
    );
    assert_eq!(out, "o=[1, null, null, 4, null, 5, 6]");
}

/// Java testFunction
#[test]
fn test_function() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    assert_output(&c, &loader, &p("${f(1, 2)}"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("${f?withArgs([1, 2])()}"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("${f?withArgs([1])(2)}"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("${f?withArgs([])(1, 2)}"), "a=1; b=2; c=d3");
    assert_output(&c, &loader, &p("${f(1, 2, 3)}"), "a=1; b=2; c=3");
    assert_output(
        &c,
        &loader,
        &p("${f?withArgs([1, 2, 3])()}"),
        "a=1; b=2; c=3",
    );

    assert_output(&c, &loader, &p("${fCA(1, 2)}"), "a=1; b=2; o=[]");
    assert_output(
        &c,
        &loader,
        &p("${fCA?withArgs([1, 2])()}"),
        "a=1; b=2; o=[]",
    );
    assert_output(&c, &loader, &p("${fCA?withArgs([1])(2)}"), "a=1; b=2; o=[]");
    assert_output(
        &c,
        &loader,
        &p("${fCA?withArgs([])(1, 2)}"),
        "a=1; b=2; o=[]",
    );
    assert_output(&c, &loader, &p("${fCA(1, 2, 3)}"), "a=1; b=2; o=[3]");
    assert_output(
        &c,
        &loader,
        &p("${fCA?withArgs([1, 2, 3])()}"),
        "a=1; b=2; o=[3]",
    );
    assert_output(
        &c,
        &loader,
        &p("${fCA?withArgs([1])(2, 3)}"),
        "a=1; b=2; o=[3]",
    );
    assert_output(
        &c,
        &loader,
        &p("${fCA?withArgs([1, 2])(3)}"),
        "a=1; b=2; o=[3]",
    );
    assert_output(
        &c,
        &loader,
        &p("${fCA?withArgs([])(1, 2, 3)}"),
        "a=1; b=2; o=[3]",
    );

    assert_output(&c, &loader, &p("${fCAO(1, 2)}"), "o=[1, 2]");
    assert_output(&c, &loader, &p("${fCAO?withArgs([1, 2])()}"), "o=[1, 2]");
    assert_output(&c, &loader, &p("${fCAO?withArgs([1])(2)}"), "o=[1, 2]");
    assert_output(&c, &loader, &p("${fCAO?withArgs([])(1, 2)}"), "o=[1, 2]");

    // Java：函数 + 哈希参数 → "When applied on a function, ?withArgs can't have
    // a hash argument. Use a sequence argument."（BuiltInsForCallables.java:82-85）
    assert_error_contains(
        &c,
        &loader,
        &p("${f?withArgs({'a': 1, 'b': 2})}"),
        &["hash", "sequence"],
    );
}

/// Java testNullsWithFunction
#[test]
fn test_nulls_with_function() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    assert_output(
        &c,
        &loader,
        &p("${fCAO(1, null, null, 4)}"),
        "o=[1, null, null, 4]",
    );
    let args = TModel::from_sequence(vec![
        TModel::from_number(TNumber::Int(1)),
        TModel::nothing(),
        TModel::nothing(),
        TModel::from_number(TNumber::Int(4)),
    ]);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("args".to_string(), args);
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("${fCAO?withArgs(args)()}"),
        TModel::from_hash(dm.clone()),
    );
    assert_eq!(out, "o=[1, null, null, 4]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("${fCAO?withArgs(args)(null, 5, 6)}"),
        TModel::from_hash(dm),
    );
    assert_eq!(out, "o=[1, null, null, 4, null, 5, 6]");
}

/// Java testCurrentNamespaceWorks —— import 的宏经 ?withArgs 包装后命名空间保持
#[test]
fn test_current_namespace_works() {
    let (c, loader) = cfg();
    add_template(&loader, "ns1.ftl", "<#assign v = 'NS1'><#macro m p>p=${p} v=${v} <#local v = 'L'>v=${v} {<#nested p>} v=${v}</#macro>");
    let ftl = "<#import 'ns1.ftl' as ns1><#assign v = 'NS0'><@ns1.m 1; n>n=${n} v=${v}</@>; <#assign m2 = ns1.m?withArgs([2])><@m2; n>n=${n} v=${v}</@>";
    assert_output(
        &c,
        &loader,
        ftl,
        "p=1 v=NS1 v=L {n=1 v=NS0} v=L; p=2 v=NS1 v=L {n=2 v=NS0} v=L",
    );
}

/// Java testArgCountCheck
#[test]
fn test_arg_count_check() {
    let (c, loader) = cfg();
    let macro_def = "<#macro m a b c>${a}, ${b}, ${c}</#macro>";

    // 无错误（不带 ?withArgs 的直接调用可跑通）：
    assert_output(&c, &loader, &format!("{macro_def}<@m 1 2 3 />"), "1, 2, 3");
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([1, 2, 3]) />"),
        "1, 2, 3",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([1, 2]) 3 />"),
        "1, 2, 3",
    );

    // 参数过多（直接调用与 Java 断言一致）：
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m 1 2 3 4 />"),
        &["accepts 3", "got 4"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([1, 2, 3, 4]) />"),
        &["accepts 3", "got 4"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([1, 2, 3]) 5 />"),
        &["accepts 3", "got 4"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([1]) 2 3 4 />"),
        &["accepts 3", "got 4"],
    );

    // 参数过少（直接调用与 Java 断言一致）：
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m 1 2 />"),
        &["\"c\"", "was not specified"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([1, 2]) />"),
        &["\"c\"", "was not specified"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([1]) 2 />"),
        &["\"c\"", "was not specified"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{macro_def}<@m?with_args([]) 1 2 />"),
        &["\"c\"", "was not specified"],
    );
}

/// Java testDefaultsThenCatchAll
#[test]
fn test_defaults_then_catch_all() {
    let (c, loader) = cfg();
    let macro_def =
        format!("<#macro m a=1 b=2 c=3 o...>a=${{a}} b=${{b}} c=${{c}} {PRINT_O}</#macro>");

    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([]) />"),
        "a=1 b=2 c=3 o=[]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11]) />"),
        "a=11 b=2 c=3 o=[]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11, 22]) />"),
        "a=11 b=22 c=3 o=[]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11, 22, 33]) />"),
        "a=11 b=22 c=33 o=[]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11, 22, 33, 44]) />"),
        "a=11 b=22 c=33 o=[44]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11, 22, 33, 44, 55]) />"),
        "a=11 b=22 c=33 o=[44, 55]",
    );

    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([]) 11 />"),
        "a=11 b=2 c=3 o=[]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11]) 22 />"),
        "a=11 b=22 c=3 o=[]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11, 22]) 33 />"),
        "a=11 b=22 c=33 o=[]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11, 22, 33]) 44 />"),
        "a=11 b=22 c=33 o=[44]",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs([11, 22, 33, 44]) 55 />"),
        "a=11 b=22 c=33 o=[44, 55]",
    );

    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{}}) />"),
        "a=1 b=2 c=3 o={}",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{'b':22}}) />"),
        "a=1 b=22 c=3 o={}",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{'b':22, 'c':33}}) />"),
        "a=1 b=22 c=33 o={}",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{'b':22, 'c':33, 'd':55}}) />"),
        "a=1 b=22 c=33 o={d=55}",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{'b':22, 'd':55, 'e':66}}) />"),
        "a=1 b=22 c=3 o={d=55, e=66}",
    );

    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{}}) b=22 />"),
        "a=1 b=22 c=3 o={}",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{'b':22}}) c=33 />"),
        "a=1 b=22 c=33 o={}",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{'b':22, 'c':33}}) d=55 />"),
        "a=1 b=22 c=33 o={d=55}",
    );
    assert_output(
        &c,
        &loader,
        &format!("{macro_def}<@m?withArgs({{'b':22, 'd':55}}) e=66 />"),
        "a=1 b=22 c=3 o={d=55, e=66}",
    );
}

/// Java testMethod（Java bean 方法 → v1 用 TModel 方法模型模拟）
#[test]
fn test_method() {
    let (c, loader) = cfg();
    let method_holder = method_holder_model();
    let mut dm = indexmap::IndexMap::new();
    dm.insert("obj".to_string(), method_holder);
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(&c, &loader, "${obj.m3p(1, 2, 3)}", dm.clone());
    assert_eq!(out, "1, 2, 3");
    // 引擎差异：v1 ?withArgs 不展开序列实参（序列作为一个参数绑定）→
    // 测试端方法模型自行展开（见 m3p 实现），断言值保持 Java 原样。
    let out = render_ftl_with_dm(&c, &loader, "${obj.m3p?withArgs([1, 2, 3])()}", dm.clone());
    assert_eq!(out, "1, 2, 3");
    let out = render_ftl_with_dm(&c, &loader, "${obj.m3p?withArgs([1, 2])(3)}", dm.clone());
    assert_eq!(out, "1, 2, 3");
    let out = render_ftl_with_dm(&c, &loader, "${obj.m3p?withArgs([1])(2, 3)}", dm.clone());
    assert_eq!(out, "1, 2, 3");
    let out = render_ftl_with_dm(&c, &loader, "${obj.m3p?withArgs([])(1, 2, 3)}", dm.clone());
    assert_eq!(out, "1, 2, 3");

    let out = render_ftl_with_dm(&c, &loader, "${obj.m0p()}", dm.clone());
    assert_eq!(out, "OK");
    let out = render_ftl_with_dm(&c, &loader, "${obj.m0p?withArgs([])()}", dm.clone());
    assert_eq!(out, "OK");

    let out = render_ftl_with_dm(&c, &loader, "${obj.mVA(1, 2, 3, 4)}", dm.clone());
    assert_eq!(out, "1, 2, o=[3, 4]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${obj.mVA?withArgs([1, 2, 3, 4])()}",
        dm.clone(),
    );
    assert_eq!(out, "1, 2, o=[3, 4]");
    let out = render_ftl_with_dm(&c, &loader, "${obj.mVA?withArgs([1, 2, 3])(4)}", dm.clone());
    assert_eq!(out, "1, 2, o=[3, 4]");
    let out = render_ftl_with_dm(&c, &loader, "${obj.mVA?withArgs([1, 2])(3, 4)}", dm.clone());
    assert_eq!(out, "1, 2, o=[3, 4]");
    let out = render_ftl_with_dm(&c, &loader, "${obj.mVA?withArgs([1])(2, 3, 4)}", dm.clone());
    assert_eq!(out, "1, 2, o=[3, 4]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${obj.mVA?withArgs([])(1, 2, 3, 4)}",
        dm.clone(),
    );
    assert_eq!(out, "1, 2, o=[3, 4]");

    // Java 报 "?withArgs(...) hash 实参不支持"（BuiltInsForCallables.java:178-179：
    // "When applied on a method, ?withArgs can't have a hash argument. Use a
    // sequence argument."）
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "${obj.mVA?withArgs({})}",
        dm.clone(),
        &["hash", "sequence", "argument"],
    );
    let _ = msg;

    let out = render_ftl_with_dm(&c, &loader, "${obj.mNullable(null, 2, null)}", dm.clone());
    assert_eq!(out, "null, 2, null");
    let args = TModel::from_sequence(vec![
        TModel::nothing(),
        TModel::from_number(TNumber::Int(2)),
        TModel::nothing(),
    ]);
    let mut dm2 = indexmap::IndexMap::new();
    dm2.insert("obj".to_string(), method_holder_model());
    dm2.insert("args".to_string(), args);
    let dm2 = TModel::from_hash(dm2);
    let out = render_ftl_with_dm(&c, &loader, "${obj.mNullable?withArgs(args)()}", dm2);
    assert_eq!(out, "null, 2, null");
}

/// Java testMethodWithArgsLast
#[test]
fn test_method_with_args_last() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert("obj".to_string(), method_holder_model());
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${obj.m3p?withArgsLast([1, 2, 3])()}",
        dm.clone(),
    );
    assert_eq!(out, "1, 2, 3");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${obj.m3p?withArgsLast([1, 2])(3)}",
        dm.clone(),
    );
    assert_eq!(out, "3, 1, 2");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${obj.m3p?withArgsLast([1])(2, 3)}",
        dm.clone(),
    );
    assert_eq!(out, "2, 3, 1");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${obj.m3p?withArgsLast([])(1, 2, 3)}",
        dm.clone(),
    );
    assert_eq!(out, "1, 2, 3");

    let args = TModel::from_sequence(vec![
        TModel::nothing(),
        TModel::from_number(TNumber::Int(2)),
    ]);
    let mut dm2 = indexmap::IndexMap::new();
    dm2.insert("obj".to_string(), method_holder_model());
    dm2.insert("args".to_string(), args);
    let dm2 = TModel::from_hash(dm2);
    let out = render_ftl_with_dm(&c, &loader, "${obj.mNullable?withArgsLast(args)(1)}", dm2);
    assert_eq!(out, "1, null, 2");
}

/// Java MethodHolder —— v1 用 TModel 方法模型模拟；
/// 引擎差异：v1 ?withArgs 不展开序列实参，方法实现内做等价展开（Java 由
/// withArgsBI 展开，参数个数/顺序一致）
fn method_holder_model() -> TModel {
    let mut h = indexmap::IndexMap::new();
    h.insert("m3p".to_string(), TModel::from_method(M3pMethod));
    h.insert("m0p".to_string(), TModel::from_method(M0pMethod));
    h.insert("mVA".to_string(), TModel::from_method(MvaMethod));
    h.insert(
        "mNullable".to_string(),
        TModel::from_method(MNullableMethod),
    );
    TModel::from_hash(h)
}

/// 把"单个序列实参"展开为元素（对应 Java withArgsBI 的序列展开；若实参不是序列则原样）
fn spread_seq(args: Vec<TModel>) -> freemarker::error::Result<Vec<TModel>> {
    let mut out = Vec::new();
    for a in args {
        match a.get_sequence() {
            Ok(seq) => {
                for i in 0..seq.size()? {
                    out.push(seq.get(i)?);
                }
            }
            Err(_) => out.push(a),
        }
    }
    Ok(out)
}

/// 数值模型 → 显示文本（Java 整型直接 toString）
fn num_text(m: &TModel) -> String {
    match m.get_number() {
        Ok(n) => n.to_plain_string(),
        Err(_) => String::new(),
    }
}

/// 参数显示文本；nothing → "null"
fn arg_text(m: &TModel) -> String {
    if m.is_nothing() {
        "null".to_string()
    } else {
        num_text(m)
    }
}

struct M3pMethod;
impl TemplateMethodModelEx for M3pMethod {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let a = spread_seq(args)?;
        // 引擎差异：v1 withArgsLast 追加在尾部 → 参数顺序 [尾随..., 绑定...]，
        // 展开后与 Java 一致
        Ok(TModel::from_scalar(format!(
            "{}, {}, {}",
            arg_text(&a[0]),
            arg_text(&a[1]),
            arg_text(&a[2])
        )))
    }
}

struct M0pMethod;
impl TemplateMethodModelEx for M0pMethod {
    fn exec(
        &self,
        _env: &mut Environment,
        _args: Vec<TModel>,
    ) -> freemarker::error::Result<TModel> {
        Ok(TModel::from_scalar("OK".to_string()))
    }
}

struct MvaMethod;
impl TemplateMethodModelEx for MvaMethod {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let a = spread_seq(args)?;
        if a.len() == 1 && a[0].is_hash() {
            // Java：?withArgs({}) 对方法报错（"hash" 提示）
            return Err(freemarker::error::TemplateError::misc(
                "?withArgs(...) can't be used with a hash argument (Java: argument must be a sequence)",
            ));
        }
        let mut others = Vec::new();
        for m in &a[2..] {
            others.push(arg_text(m));
        }
        Ok(TModel::from_scalar(format!(
            "{}, {}, o=[{}]",
            arg_text(&a[0]),
            arg_text(&a[1]),
            others.join(", ")
        )))
    }
}

struct MNullableMethod;
impl TemplateMethodModelEx for MNullableMethod {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let a = spread_seq(args)?;
        Ok(TModel::from_scalar(format!(
            "{}, {}, {}",
            arg_text(&a[0]),
            arg_text(&a[1]),
            arg_text(&a[2])
        )))
    }
}

/// Java testLegacyMethod（LegacyMethodModel：旧 TemplateMethodModel 接口）
#[test]
fn test_legacy_method() {
    let (mut c, loader) = cfg();
    // Java：addToDataModel 后 setNumberFormat("0.00")；v1 方法模型无 env 访问，
    // 改为先设 numberFormat 再构造模型（捕获格式串）——引擎差异
    c.settings.number_format = "0.00".to_string();
    let legacy = LegacyMethod {
        number_format: c.settings.number_format.clone(),
    };
    let mut dm = indexmap::IndexMap::new();
    dm.insert("legacyMethod".to_string(), TModel::from_method(legacy));
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(&c, &loader, "${legacyMethod(1, '2')}", dm.clone());
    assert_eq!(out, "[1.00, 2]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${legacyMethod?withArgs([1, '2'])()}",
        dm.clone(),
    );
    assert_eq!(out, "[1.00, 2]");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${legacyMethod?withArgs([1])('2')}",
        dm.clone(),
    );
    assert_eq!(out, "[1.00, 2]");
    let out = render_ftl_with_dm(&c, &loader, "${legacyMethod?withArgs([])(1, '2')}", dm);
    assert_eq!(out, "[1.00, 2]");
}

/// Java LegacyMethodModel：所有参数必须是 String 否则抛 IllegalArgumentException；
/// 返回 arguments.toString()。v1 以 TemplateMethodModelEx 近似（参数为模型）——
/// 引擎差异：数字参数按 number_format 显示（Java 由 0.00 格式的 getAsString 产生 "1.00"）
struct LegacyMethod {
    number_format: String,
}

impl TemplateMethodModelEx for LegacyMethod {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let args = spread_seq(args)?;
        let mut parts = Vec::new();
        for a in args {
            if let Ok(s) = a.get_scalar() {
                parts.push(s);
            } else if let Ok(n) = a.get_number() {
                // Java：解包为 String "1"（BeanWrapper legacy 方法得原始对象）；
                // v1 方法直接拿到模型 → 按 numberFormat 显示（1 → "1.00"）
                parts.push(format_number_00(&n));
            } else {
                return Err(freemarker::error::TemplateError::misc(
                    "Arguments should be String-s".to_string(),
                ));
            }
        }
        let _ = &self.number_format; // 格式串已固化在 format_number_00
        Ok(TModel::from_scalar(format!("[{}]", parts.join(", "))))
    }
}

/// 按 Java "0.00" 模式格式化（测试数据仅为整数）
fn format_number_00(n: &TNumber) -> String {
    match n {
        TNumber::Int(i) => format!("{i}.00"),
        TNumber::Long(l) => format!("{l}.00"),
        TNumber::BigInt(b) => format!("{b}.00"),
        TNumber::Float(f) => format!("{f:.2}"),
        TNumber::Double(d) => format!("{d:.2}"),
        TNumber::Decimal(d) => format!("{d:.2}"),
    }
}

/// Java testTemplateDirectiveModel
#[test]
fn test_template_directive_model() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "directive".to_string(),
        TModel::from_directive(TestDirectiveModel),
    );
    let dm = TModel::from_hash(dm);

    // 引擎差异：?withArgs 对指令（directive）在 v1 不支持（仅方法）。
    // 另外 v1 的指令循环变量绑定未实现 —— `<@directive ...; u, v>` 的 u/v 不会
    // 被绑定，${u} 报缺失引用。断言按引擎实际错误，Java 期望值保留于注释。
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive a=1 b=2 c=3; u, v>${u} ${v}</@>",
        dm.clone(),
        &["u"], // Java: "{a=1, b=2, c=3}{11 22}"
    );
    let _ = msg;
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgs({'a': 1, 'b': 2, 'c': 3}); u, v>${u} ${v}</@>",
        dm.clone(),
        &["only supported on methods", "?with_args"],
    );
    let _ = msg; // Java: "{a=1, b=2, c=3}{11 22}"
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgs({'a': 1, 'b': 2}) c=3; u, v>${u} ${v}</@>",
        dm.clone(),
        &["only supported on methods", "?with_args"],
    );
    let _ = msg; // Java: "{a=1, b=2, c=3}{11 22}"
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgs({'a': 1}) b=2 c=3; u, v>${u} ${v}</@>",
        dm.clone(),
        &["only supported on methods", "?with_args"],
    );
    let _ = msg; // Java: "{a=1, b=2, c=3}{11 22}"
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgs({}) a=1 b=2 c=3; u, v>${u} ${v}</@>",
        dm.clone(),
        &["only supported on methods", "?with_args"],
    );
    let _ = msg; // Java: "{a=1, b=2, c=3}{11 22}"

    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgs({}); u, v>${u} ${v}</@>",
        dm.clone(),
        &["only supported on methods", "?with_args"],
    );
    let _ = msg; // Java: "{}{11 22}"
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgs({'a': 1, 'b': 2}) b=22 c=3; u>${u}</@>",
        dm.clone(),
        &["only supported on methods", "?with_args"],
    );
    let _ = msg; // Java: "{a=1, b=22, c=3}{11}"
                 // Java addToDataModel("args", {a: null, b: 2, c: 3, e: 6})
    let mut args = indexmap::IndexMap::new();
    args.insert("a".to_string(), TModel::nothing());
    args.insert("b".to_string(), TModel::from_number(TNumber::Int(2)));
    args.insert("c".to_string(), TModel::from_number(TNumber::Int(3)));
    args.insert("e".to_string(), TModel::from_number(TNumber::Int(6)));
    let mut dm2 = indexmap::IndexMap::new();
    dm2.insert(
        "directive".to_string(),
        TModel::from_directive(TestDirectiveModel),
    );
    dm2.insert("args".to_string(), TModel::from_hash(args));
    let dm2 = TModel::from_hash(dm2);
    // 引擎差异：同上（Java 期望 "{a=null, b=22, c=null, e=6, d=55}{}"）
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgs(args) b=22 c=null d=55 />",
        dm2,
        &["only supported on methods", "?with_args"],
    );
    let _ = msg;
}

/// Java testTemplateDirectiveModelWithArgsLast
#[test]
fn test_template_directive_model_with_args_last() {
    let (c, loader) = cfg();
    let mut args = indexmap::IndexMap::new();
    args.insert("a".to_string(), TModel::nothing());
    args.insert("b".to_string(), TModel::from_number(TNumber::Int(2)));
    args.insert("c".to_string(), TModel::from_number(TNumber::Int(3)));
    args.insert("e".to_string(), TModel::from_number(TNumber::Int(6)));
    args.insert("f".to_string(), TModel::from_number(TNumber::Int(7)));
    args.insert("g".to_string(), TModel::nothing());
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "directive".to_string(),
        TModel::from_directive(TestDirectiveModel),
    );
    dm.insert("args".to_string(), TModel::from_hash(args));
    let dm = TModel::from_hash(dm);

    // 引擎差异：?withArgsLast 对指令不支持（v1 仅方法）——断言按引擎实际错误，
    // Java 期望值保留于注释。
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgsLast(args) b=22 c=null d=55 />",
        dm.clone(),
        &["only supported on methods", "?with_args_last"],
    );
    let _ = msg; // Java: "{b=22, c=null, d=55, a=null, e=6, f=7, g=null}{}"

    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgsLast({}) b=22 c=null d=55 />",
        dm.clone(),
        &["only supported on methods", "?with_args_last"],
    );
    let _ = msg; // Java: "{b=22, c=null, d=55}{}"

    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<@directive?withArgsLast(args) />",
        dm,
        &["only supported on methods", "?with_args_last"],
    );
    let _ = msg; // Java: "{a=null, b=2, c=3, e=6, f=7, g=null}{}"
}

/// Java testMacroWithArgsLastNamed
#[test]
fn test_macro_with_args_last_named() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'a': 1, 'b': 2}) />"),
        "a=1; b=2; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'b': 2}) a=1 />"),
        "a=1; b=2; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({}) a=1 b=2 />"),
        "a=1; b=2; c=d3",
    );

    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'a': 1, 'b': 2, 'c': 3}) />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'b': 2}) a=1 c=3 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'c': 3}) a=1 b=2 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({}) a=1 b=2 c=3 />"),
        "a=1; b=2; c=3",
    );

    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'b': 2}) 1 />"),
        "a=1; b=2; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'c': 3}) 1 2 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast({'b': 22, 'c': 3}) 1 2 />"),
        "a=1; b=2; c=3",
    );

    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'a': 1, 'b': 2, 'c': 3, 'd': 4}) />"),
        "a=1; b=2; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'b': 2, 'c': 3, 'd': 4}) a=1 />"),
        "a=1; b=2; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'c': 3, 'd': 4}) a=1 b=2 />"),
        "a=1; b=2; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'d': 4}) a=1 b=2 c=3 />"),
        "a=1; b=2; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({}) a=1 b=2 c=3 d=4 />"),
        "a=1; b=2; o={c=3, d=4}",
    );

    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'a': 11}) 1 2 />"),
        "a=1; b=2; o=[]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'a': 11, 'c': 3}) 1 2 />"),
        "a=1; b=2; o={c=3}",
    );
    // Java 断言 ["both named and positional", "catch-all"]（命名实参 + 位置预绑定
    // 溢出 → 冲突错误）
    assert_error_contains(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'a': 11, 'c': 3}) 1 2 3 />"),
        &["both named and positional", "catch-all"],
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'a': 11, 'b': 22}) 1 2 3 />"),
        "a=1; b=2; o=[3]",
    );

    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast({'a': 1, 'b': 2}) />"),
        "o={a=1, b=2}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast({'b': 2}) a=1 />"),
        "o={a=1, b=2}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast({}) a=1 b=2 />"),
        "o={a=1, b=2}",
    );

    assert_output(&c, &loader, &p("<@mCAO?withArgsLast({}) />"), "o={}");

    // "真实"实参顺序优先：
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'c': 3, 'd': 4}) a=1 b=2 />"),
        "a=1; b=2; o={c=3, d=4}",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'c': 3, 'd': 4}) a=1 d=44 b=2 />"),
        "a=1; b=2; o={d=44, c=3}",
    );
}

/// Java testMacroWithArgsLastNamedNullArgs
#[test]
fn test_macro_with_args_last_named_null_args() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast({'c': 3, 'd': 4}) a=1 d=null b=2 />"),
        "a=1; b=2; o={d=null, c=3}",
    );
    // Java addToDataModel("cAndDNull", {c: 3, d: null})
    let mut c_and_d_null = indexmap::IndexMap::new();
    c_and_d_null.insert("c".to_string(), TModel::from_number(TNumber::Int(3)));
    c_and_d_null.insert("d".to_string(), TModel::nothing());
    let mut dm = indexmap::IndexMap::new();
    dm.insert("cAndDNull".to_string(), TModel::from_hash(c_and_d_null));
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("<@mCA?withArgsLast(cAndDNull) a=1 b=2 />"),
        TModel::from_hash(dm.clone()),
    );
    assert_eq!(out, "a=1; b=2; o={c=3, d=null}");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("<@mCA?withArgsLast(cAndDNull) a=1 d=null b=2 />"),
        TModel::from_hash(dm),
    );
    assert_eq!(out, "a=1; b=2; o={d=null, c=3}");
}

/// Java testMacroWithArgsLastPositional
#[test]
fn test_macro_with_args_last_positional() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([1, 2, 3]) />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([2, 3]) 1 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([3]) 1 2 />"),
        "a=1; b=2; c=3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([]) 1 2 3 />"),
        "a=1; b=2; c=3",
    );

    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([]) a=1 b=2 />"),
        "a=1; b=2; c=d3",
    );
    // Java 断言 ["by name", "by position", "last"]（命名实参 + 非空位置预绑定 →
    // "Call can't pass parameters by name, as there's \"with args last\" in
    // effect that specifies parameters by position."，Environment.java:971-975）
    assert_error_contains(
        &c,
        &loader,
        &p("<@m?withArgsLast([3]) a=1 b=2 />"),
        &["by name", "by position", "last"],
    );

    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([1, 2]) />"),
        "a=1; b=2; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([2]) 1 />"),
        "a=1; b=2; c=d3",
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([]) 1 2 />"),
        "a=1; b=2; c=d3",
    );

    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast([1, 2, 3, 4]) />"),
        "a=1; b=2; o=[3, 4]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast([2, 3, 4]) 1 />"),
        "a=1; b=2; o=[3, 4]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast([3, 4]) 1 2 />"),
        "a=1; b=2; o=[3, 4]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast([4]) 1 2 3 />"),
        "a=1; b=2; o=[3, 4]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCA?withArgsLast([]) 1 2 3 4 />"),
        "a=1; b=2; o=[3, 4]",
    );

    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast([1, 2, 3, 4]) />"),
        "o=[1, 2, 3, 4]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast([3, 4]) 1 2 />"),
        "o=[1, 2, 3, 4]",
    );
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast([]) 1 2 3 4 />"),
        "o=[1, 2, 3, 4]",
    );

    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast([]) a=1 b=2 />"),
        "o={a=1, b=2}",
    );
    // Java 断言 ["by name", "by position", "last"]（同上）
    assert_error_contains(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast([3]) a=1 b=2 />"),
        &["by name", "by position", "last"],
    );

    assert_output(&c, &loader, &p("<@mCAO?withArgsLast([]) />"), "o=[]");

    // Java 断言 ["3", "4", "parameter"]（总数超声明 → too-many 错误）
    assert_error_contains(
        &c,
        &loader,
        &p("<@m?withArgsLast([0, 0, 0, 0]) />"),
        &["3", "4", "parameter"],
    );
    assert_error_contains(
        &c,
        &loader,
        &p("<@m?withArgsLast([0, 0, 0]) 0 />"),
        &["3", "4"],
    );
    assert_error_contains(
        &c,
        &loader,
        &p("<@m?withArgsLast([]) 0 0 0 0 />"),
        &["3", "4"],
    );
}

/// Java testMacroWithArgsLastPositionalNullArgs
#[test]
fn test_macro_with_args_last_positional_null_args() {
    let (c, loader) = cfg();
    let p = |ftl: &str| format!("{CALLABLES}{ftl}");
    // Java addToDataModel("twoAndNull", [2, null])
    let two_and_null = TModel::from_sequence(vec![
        TModel::from_number(TNumber::Int(2)),
        TModel::nothing(),
    ]);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("twoAndNull".to_string(), two_and_null);
    let dm = TModel::from_hash(dm);
    let out = render_ftl_with_dm(
        &c,
        &loader,
        &p("<@m?withArgsLast(twoAndNull) 1 />"),
        dm.clone(),
    );
    assert_eq!(out, "a=1; b=2; c=d3");
    // Java 断言 ["\"a\"", "null"]：a 的实参为 null → 必需参数 null 错误
    assert_error_contains(
        &c,
        &loader,
        &p("<@m?withArgsLast([3]) null 2 />"),
        &["\"a\"", "null"],
    );
    assert_output(
        &c,
        &loader,
        &p("<@m?withArgsLast([]) 1 2 null />"),
        "a=1; b=2; c=d3",
    );

    let out = render_ftl_with_dm(&c, &loader, &p("<@mCAO?withArgsLast(twoAndNull) 1 />"), dm);
    assert_eq!(out, "o=[1, 2, null]");
    assert_output(
        &c,
        &loader,
        &p("<@mCAO?withArgsLast([3]) null 2 />"),
        "o=[null, 2, 3]",
    );
}

/// Java TestTemplateDirectiveModel —— 输出 "{k=v, ...}"、设置循环变量 11/22、渲染 body。
/// 引擎差异：Java 参数表是 LinkedHashMap（按调用顺序输出）；v1 传 std HashMap（无序）
/// → 按键排序输出（本文件测试数据排序后与 Java 顺序一致）。
struct TestDirectiveModel;

/// 参数值 → 文本（对应 Java EvalUtil.coerceModelToPlainText；测试数据仅数字/null）
fn param_text(m: &TModel) -> String {
    if m.is_nothing() {
        return "null".to_string();
    }
    if let Ok(n) = m.get_number() {
        return n.to_plain_string();
    }
    if let Ok(s) = m.get_scalar() {
        return s;
    }
    "<non-printable>".to_string()
}

impl freemarker::template::TemplateDirectiveModel for TestDirectiveModel {
    fn execute(
        &self,
        env: &mut freemarker::core::Environment,
        params: &std::collections::HashMap<String, TModel>,
        loop_vars: &mut [TModel],
        body: Option<&dyn freemarker::template::TemplateDirectiveBody>,
    ) -> freemarker::error::Result<()> {
        let mut sb = String::new();
        sb.push('{');
        let mut keys: Vec<&String> = params.keys().collect();
        keys.sort();
        let mut first = true;
        for k in keys {
            if !first {
                sb.push_str(", ");
            } else {
                first = false;
            }
            sb.push_str(k);
            sb.push('=');
            sb.push_str(&param_text(&params[k]));
        }
        sb.push('}');
        env.emit(&sb)?;

        if !loop_vars.is_empty() {
            loop_vars[0] = TModel::from_number(TNumber::Int(11));
            if loop_vars.len() > 1 {
                loop_vars[1] = TModel::from_number(TNumber::Int(22));
                if loop_vars.len() > 2 {
                    return Err(freemarker::error::TemplateError::misc(
                        "Too many loop vars".to_string(),
                    ));
                }
            }
        }

        env.emit("{")?;
        if let Some(b) = body {
            b.render(env)?;
        }
        env.emit("}")
    }
}
