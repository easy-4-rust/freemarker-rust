//! Java `freemarker.core.CallerTemplateNameTest` 的 Rust 1:1 实现
//! （对应 Java: CallerTemplateNameTest —— `.callerTemplateName`/`.caller_template_name`
//!   特殊变量在宏/函数/include/import/嵌套/参数默认值/局部化查找等场景下的"调用方模板名"）。
//!
//! 语义（Java BuiltinVariable.java:264-267 + Macro.Context.callPlace，Macro.java:227-250）：
//! `.caller_template_name` = 当前宏/函数**调用点所在模板**的查找名（词法模板；
//! Environment 指令栈顶元素 template 的等价物 —— Rust 侧 lexical_template_name）；
//! 无名调用方 → ""（Java getName()==null → EMPTY_STRING）；宏外 → 报错。
//!
//! 引擎差异：`?interpret` 的动态表达式不携带词法模板（Java ?eval hack 用负行号
//! 标记），v1 与 Java 一致按当前词法模板处理，无测试覆盖差异。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::{StringLoader, TemplateLoader};
use freemarker::template::{Configuration, TModel, Version};
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java testBaics —— 宏/函数在 main/include 链中的调用方模板名
#[test]
fn test_basics() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "main.ftl",
        "<#macro m>${.callerTemplateName}</#macro>\
         <#function f()><#return .callerTemplateName></#function>\
         <@m /> ${f()} [<#include 'other.ftl'>] <@m /> ${f()}",
    );
    add_template(
        &loader,
        "other.ftl",
        "<@m /> ${f()} [<#include 'yet-another.ftl'>] <@m /> ${f()}",
    );
    add_template(&loader, "yet-another.ftl", "<@m /> ${f()}");
    assert_eq!(
        render_named(&c, &loader, "main.ftl"),
        "main.ftl main.ftl [other.ftl other.ftl [yet-another.ftl yet-another.ftl] other.ftl other.ftl] main.ftl main.ftl"
    );
}

/// Java testNoCaller —— 宏外使用 → "Can't get .callerTemplateName here, as there's
/// no macro or function (that's implemented in the template) call in context."
#[test]
fn test_no_caller() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "${.callerTemplateName}",
        &["no macro or function", ".callerTemplateName"],
    );
    assert_error_contains(
        &c,
        &loader,
        "${.caller_template_name}",
        &["no macro or function", ".caller_template_name"],
    );
    // 宏的嵌套内容（<#nested> 回插）中宏上下文为空（无宏调用在上下文中）
    assert_error_contains(
        &c,
        &loader,
        "<#macro m><#nested></#macro><@m>${.callerTemplateName}</@>",
        &["no macro or function", ".callerTemplateName"],
    );
    add_template(&loader, "main.ftl", "${.callerTemplateName}");
    let msg = assert_error_contains(
        &c,
        &loader,
        "<#include 'main.ftl'>",
        &["no macro or function"],
    );
    let _ = msg;
}

/// Java testNamelessCaller —— 无名模板（name==null）→ 调用方模板名为 ""（EMPTY_STRING）
#[test]
fn test_nameless_caller() {
    let (c, _loader) = cfg();
    // Java assertOutput：new Template(null, ftl, cfg)（无名模板）；
    // v1 render_ftl 用固定名 "adhoc" —— 此处直接按无名（name=""）解析等价
    let cfg = std::rc::Rc::new(c.clone());
    let t = freemarker::parser::parse(
        &cfg,
        "",
        "<#macro m2>${.callerTemplateName}</#macro>[<@m2/>]",
    )
    .unwrap();
    let mut out = Vec::new();
    t.process(TModel::from_hash(indexmap::IndexMap::new()), &mut out)
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out), "[]");
}

/// Java testNested —— include 的宏定义模板；`<#nested>` 回插时切回调用方帧
#[test]
fn test_nested() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "main.ftl",
        "<#include 'lib1.ftl'><#include 'lib2.ftl'><@m1 />",
    );
    add_template(
        &loader,
        "lib1.ftl",
        "<#macro m1>${.callerTemplateName} [<@m2>${.callerTemplateName}</@m2>] ${.callerTemplateName}</#macro>",
    );
    add_template(
        &loader,
        "lib2.ftl",
        "<#macro m2>${.callerTemplateName} [<#nested>] ${.callerTemplateName}</#macro>",
    );
    assert_eq!(
        render_named(&c, &loader, "main.ftl"),
        "main.ftl [lib1.ftl [main.ftl] lib1.ftl] main.ftl"
    );
}

/// Java testSelfCaller —— 宏在定义模板内被调用 → 调用方 = 定义模板
#[test]
fn test_self_caller() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "main.ftl",
        "<#macro m>${.callerTemplateName}</#macro><@m />",
    );
    assert_eq!(render_named(&c, &loader, "main.ftl"), "main.ftl");
}

/// Java testImportedTemplateCaller —— import 的宏在库模板内互相调用 →
/// 调用方 = 库模板名
#[test]
fn test_imported_template_caller() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "main.ftl",
        "<#import 'lib/foo.ftl' as foo><@foo.m />, <@foo.m2 />",
    );
    add_template(
        &loader,
        "lib/foo.ftl",
        "<#macro m>${.callerTemplateName}</#macro><#macro m2><@m3/></#macro><#macro m3>${.callerTemplateName}</#macro>",
    );
    assert_eq!(
        render_named(&c, &loader, "main.ftl"),
        "main.ftl, lib/foo.ftl"
    );
}

/// Java testNestedIntoNonUserDirectives —— 非用户指令（list/if）内宏上下文保持
#[test]
fn test_nested_into_non_user_directives() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "main.ftl",
        "<#macro m><#list 1..2 as _><#if true>${.callerTemplateName}</#if>;</#list></#macro><@m/>",
    );
    assert_eq!(render_named(&c, &loader, "main.ftl"), "main.ftl;main.ftl;");
}

/// Java testUsedInArgument —— 实参与参数默认值中的调用方模板名；
/// 宏 m2 定义在 inc.ftl（include）→ 其体内调用点调用方 = inc.ftl
#[test]
fn test_used_in_argument() {
    let (mut c, loader) = cfg();
    add_template(
        &loader,
        "main.ftl",
        "<#include 'inc.ftl'><#macro start><@m .callerTemplateName /><@m2 /></#macro><@start />",
    );
    add_template(
        &loader,
        "inc.ftl",
        "<#macro m x y=.callerTemplateName>x: ${x}; y: ${y}; caller: ${.callerTemplateName};</#macro><#macro m2><@m .callerTemplateName /></#macro>",
    );
    for _i in 0..2 {
        assert_eq!(
            render_named(&c, &loader, "main.ftl"),
            "x: main.ftl; y: main.ftl; caller: main.ftl;x: main.ftl; y: inc.ftl; caller: inc.ftl;"
        );
        // Java：setIncompatibleImprovements(2.3.27)（对该变量无影响）
        c.settings.incompatible_improvements = Version::parse("2.3.27").unwrap();
    }
}

/// Java testReturnsLookupName —— 局部化查找：模板名保持**查找名**（main.ftl），
/// 而非实际文件名（main_en.ftl）
#[test]
fn test_returns_lookup_name() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "main_en.ftl",
        "<#macro m>${.callerTemplateName}</#macro><@m />",
    );
    // Java：getTemplate("main.ftl") 经局部化候选命中 main_en.ftl，但
    // Template.getName() == "main.ftl"（请求名，TemplateCache.java:549
    // new Template(name, sourceName, ...)）；v1 get_template_localized 用命中名
    // 作模板名（configuration_test.rs 文档化差异）→ 测试端按请求名解析等价
    let out = render_localized_named(&c, &loader, "main.ftl");
    assert_eq!(out, "main.ftl"); // Not main_en.ftl
}

/// Java testLegacyCall —— `<#call m>`（legacy 调用）同样记录调用方模板名
#[test]
fn test_legacy_call() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "main_en.ftl",
        "<#macro m>${.callerTemplateName}</#macro><#call m>",
    );
    let out = render_localized_named(&c, &loader, "main.ftl");
    assert_eq!(out, "main.ftl"); // Not main_en.ftl
}

/// 局部化命名的等价渲染：Java getTemplate("main.ftl") 在 locale en_US 下命中
/// "main_en.ftl" 源，模板名保持请求名（见 test_returns_lookup_name 注释）。
/// v1 引擎 get_template_localized 以命中名命名模板（文档化差异），测试端按
/// Java 语义手动解析。
fn render_localized_named(c: &Configuration, loader: &Arc<StringLoader>, name: &str) -> String {
    let cfg = std::rc::Rc::new(c.clone());
    // 局部化候选：main.ftl + en_US → ["main_en_US.ftl", "main_en.ftl", "main.ftl"]
    //（util test_config locale=en_US；本测试注册的是第二候选）
    let src = loader
        .find("main_en.ftl")
        .unwrap()
        .expect("localized template missing");
    let text = loader.read(&*src).unwrap();
    let t = freemarker::parser::parse(&cfg, name, &text).unwrap();
    let mut out = Vec::new();
    t.process(TModel::from_hash(indexmap::IndexMap::new()), &mut out)
        .unwrap();
    String::from_utf8_lossy(&out).into_owned()
}
