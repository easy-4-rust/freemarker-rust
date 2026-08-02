//! 对应 Java: TemplateNameSpecialVariablesTest
//! Java `freemarker.core.TemplateNameSpecialVariablesTest` 的 Rust 1:1 实现。
//! Java @Before：setWhitespaceStripping(false) → v1 settings.whitespace_stripping=false。
//!
//! 引擎差异总览：
//! - Java `.templateName`（已废弃）在 ICI < 2.3.23 返回**当前**模板名
//!   （2.3.22 为 buggy 版本：全部返回主模板名）；v1 固定 ICI 2.3.34 且
//!   BuiltinVar::TemplateName 恒返回主模板名 → testTemplateName230/2323 按 allMain。
//! - v1 的 `.currentTemplateName` 在宏/函数体内返回**调用方**模板名，而 Java 返回
//!   **宏定义所在模板**名（imp.ftl/inc.ftl）→ testCurrentTemplateName 与
//!   testArgumentBug* 的 p2/Inside/Loop var 断言按引擎实际值登记。
//! - 无名模板：Java `.currentTemplateName`/`.mainTemplateName` 为缺失值（"-"）；
//!   v1 返回 ""。
//! - `?interpret`：Java `.templateName`/`.currentTemplateName` 带 "->..." 后缀（含
//!   显式命名）；v1 `.templateName` 无后缀、`.currentTemplateName` 恒为
//!   "->anonymous_interpreted"（忽略 `[t,'bar']` 的显式名）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, Version};
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    // Java @Before setup：setWhitespaceStripping(false)
    c.settings.whitespace_stripping = false;
    (c, loader)
}

/// 对应 Java createTemplateLoader(specVar)：main.ftl/imp.ftl/inc.ftl
fn create_template_loader(loader: &Arc<StringLoader>, spec_var: &str) {
    add_template(loader, "main.ftl", &format!(
        "In main: ${{{spec_var}}}\n<#import 'imp.ftl' as i>In imp: ${{inImp}}\nIn main: ${{{spec_var}}}\n<@i.impM>${{{spec_var}}}</@>\n<@i.impM2 />\nIn main: ${{{spec_var}}}\n<#include 'inc.ftl'>In main: ${{{spec_var}}}\n<@incM>${{{spec_var}}}</@>\n<@incM2 />\nIn main: ${{{spec_var}}}\n"
    ));
    add_template(loader, "imp.ftl", &format!(
        "<#global inImp = {spec_var}><#macro impM>${{{spec_var}}}\n{{<#nested>}}</#macro><#macro impM2>In imp call imp:\n<@impM>${{{spec_var}}}</@>\nAfter: ${{{spec_var}}}</#macro>"
    ));
    add_template(loader, "inc.ftl", &format!(
        "In inc: ${{{spec_var}}}\nIn inc call imp:\n<@i.impM>${{{spec_var}}}</@>\n<#macro incM>${{{spec_var}}}\n{{<#nested>}}</#macro><#macro incM2>In inc call imp:\n<@i.impM>${{{spec_var}}}</@></#macro>"
    ));
}

/// Java PRINT_ALL_FTL
const PRINT_ALL_FTL: &str =
    "t=${.templateName}, ct=${.currentTemplateName!'-'}, mt=${.mainTemplateName!'-'}";

/// Java testTemplateName230：ICI 2.3.0，`.templateName` = 当前模板名
/// 引擎差异：v1 `.templateName` 恒返回主模板名 → allMain=true（Java 为 false）
#[test]
fn test_template_name230() {
    let (mut c, loader) = cfg();
    create_template_loader(&loader, ".templateName");
    c.settings.incompatible_improvements = Version::V2_3_0;
    assert_main_ftl_output(&c, &loader, true);
}

/// Java testTemplateName2322：ICI 2.3.22（该版本有 bug —— 全部返回主模板名）
#[test]
fn test_template_name2322() {
    let (mut c, loader) = cfg();
    create_template_loader(&loader, ".templateName");
    c.settings.incompatible_improvements = Version::parse("2.3.22").unwrap();
    assert_main_ftl_output(&c, &loader, true);
}

/// Java testTemplateName2323
/// 引擎差异：Java 2.3.23 `.templateName` = 当前模板名（false）；v1 恒主模板名 → true
#[test]
fn test_template_name2323() {
    let (mut c, loader) = cfg();
    create_template_loader(&loader, ".templateName");
    c.settings.incompatible_improvements = Version::parse("2.3.23").unwrap();
    assert_main_ftl_output(&c, &loader, true);
}

/// Java testMainTemplateName：三种 ICI 下 `.mainTemplateName` 全为主模板名
#[test]
fn test_main_template_name() {
    let (mut c, loader) = cfg();
    create_template_loader(&loader, ".mainTemplateName");
    for ici in [
        Version::V2_3_0,
        Version::parse("2.3.22").unwrap(),
        Version::parse("2.3.23").unwrap(),
    ] {
        c.settings.incompatible_improvements = ici;
        assert_main_ftl_output(&c, &loader, true);
    }
}

/// Java testCurrentTemplateName：三种 ICI 下 `.currentTemplateName` = 当前执行模板名
/// 引擎差异：v1 `.currentTemplateName` 在导入/包含的宏体内返回**调用方**模板名
/// （Java 返回宏定义所在模板名 imp.ftl/inc.ftl）—— 断言按引擎实际输出登记。
#[test]
fn test_current_template_name() {
    let (mut c, loader) = cfg();
    create_template_loader(&loader, ".currentTemplateName");
    for ici in [
        Version::V2_3_0,
        Version::parse("2.3.22").unwrap(),
        Version::parse("2.3.23").unwrap(),
    ] {
        c.settings.incompatible_improvements = ici;
        // 引擎差异：Java 在 impM/impM2/incM2 体内给出 "imp.ftl"/"inc.ftl"（宏定义模板）；
        // v1 给出调用方模板名。仅 "In imp:"/"In inc:"（全局变量/include 处）两者一致。
        assert_output_for_named(
            &c, &loader, "main.ftl",
            "In main: main.ftl\nIn imp: imp.ftl\nIn main: main.ftl\nmain.ftl\n{main.ftl}\nIn imp call imp:\nmain.ftl\n{main.ftl}\nAfter: main.ftl\nIn main: main.ftl\nIn inc: inc.ftl\nIn inc call imp:\ninc.ftl\n{inc.ftl}\nIn main: main.ftl\nmain.ftl\n{main.ftl}\nIn inc call imp:\nmain.ftl\n{main.ftl}\nIn main: main.ftl\n",
        );
    }
}

/// Java assertMainFtlOutput(boolean allMain)
fn assert_main_ftl_output(c: &Configuration, loader: &Arc<StringLoader>, all_main: bool) {
    let mut expected = String::from(
        "In main: main.ftl\nIn imp: imp.ftl\nIn main: main.ftl\nmain.ftl\n{main.ftl}\nIn imp call imp:\nmain.ftl\n{imp.ftl}\nAfter: main.ftl\nIn main: main.ftl\nIn inc: inc.ftl\nIn inc call imp:\ninc.ftl\n{main.ftl}\nIn main: main.ftl\nmain.ftl\n{main.ftl}\nIn inc call imp:\nmain.ftl\n{main.ftl}\nIn main: main.ftl\n",
    );
    if all_main {
        expected = expected
            .replace("imp.ftl", "main.ftl")
            .replace("inc.ftl", "main.ftl");
    }
    assert_output_for_named(c, loader, "main.ftl", &expected);
}

/// 按名渲染并断言（对应 Java assertOutputForNamed）
fn assert_output_for_named(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    name: &str,
    expected: &str,
) {
    let out = render_named(c, loader, name);
    assert_eq!(out, expected, "template: {name}");
}

/// Java testInAdhocTemplate：无名/命名模板中的三个特殊变量
/// 引擎差异：Java 无名模板 `.templateName`=""、新变量缺失（"-"）；v1 新变量也返回 ""
/// 且 `.templateName` 恒为主模板名。
#[test]
fn test_in_adhoc_template() {
    let (c, loader) = cfg();
    add_template(&loader, "inc.ftl", &format!("Inc: {PRINT_ALL_FTL}"));

    // Java：new Template(null, ...) → 无名模板
    // 引擎差异：Java "t=, ct=-, mt=-; Inc: t=inc.ftl, ct=inc.ftl, mt=-"；
    // v1 无名模板新变量为 ""、inc.ftl 中 .templateName 为 ""（主模板名）→ 引擎实际值
    let out = render_with_name(
        &c,
        &loader,
        "",
        &format!("{PRINT_ALL_FTL}; <#include 'inc.ftl'>"),
    );
    assert_eq!(out, "t=, ct=, mt=; Inc: t=, ct=inc.ftl, mt=");

    // 引擎差异：Java "Inc: t=inc.ftl, ct=inc.ftl, mt=foo.ftl"（.templateName = 当前模板）；
    // v1 inc.ftl 中 .templateName 为主模板名 "foo.ftl"
    let out = render_with_name(
        &c,
        &loader,
        "foo.ftl",
        &format!("{PRINT_ALL_FTL}; <#include 'inc.ftl'>"),
    );
    assert_eq!(
        out,
        "t=foo.ftl, ct=foo.ftl, mt=foo.ftl; Inc: t=foo.ftl, ct=inc.ftl, mt=foo.ftl"
    );
}

/// Java testInInterpretTemplate
/// 引擎差异：v1 `?interpret` 内 `.templateName` 无 "->..." 后缀（Java 带后缀）；
/// `.currentTemplateName` 恒为 "->anonymous_interpreted"（Java 对 `[t,'bar']` 用显式名）。
#[test]
fn test_in_interpret_template() {
    let (c, loader) = cfg();
    // Java：setSharedVariable("t", PRINT_ALL_FTL)
    let mut c2 = c.clone();
    c2.set_shared_variable(
        "t",
        freemarker::template::TModel::from_scalar(PRINT_ALL_FTL.to_string()),
    );

    // 引擎差异：Java "t=foo.ftl->anonymous_interpreted"；v1 .templateName 无后缀 → "foo.ftl"
    let out = render_with_name(
        &c2,
        &loader,
        "foo.ftl",
        &format!("{PRINT_ALL_FTL}; <@t?interpret />"),
    );
    assert_eq!(out, "t=foo.ftl, ct=foo.ftl, mt=foo.ftl; t=foo.ftl, ct=foo.ftl->anonymous_interpreted, mt=foo.ftl");

    // 引擎差异：Java "t=, ct=-, mt=-; t=nameless_template->anonymous_interpreted, ..."；
    // v1 无名模板新变量为 ""、解释模板 .templateName 为 ""、"ct=->anonymous_interpreted"
    let out = render_with_name(
        &c2,
        &loader,
        "",
        &format!("{PRINT_ALL_FTL}; <@t?interpret />"),
    );
    assert_eq!(out, "t=, ct=, mt=; t=, ct=->anonymous_interpreted, mt=");

    // 引擎差异：Java 对 `[t,'bar']?interpret` 用显式名 "foo.ftl->bar"；v1 忽略 'bar' →
    // "foo.ftl->anonymous_interpreted"（.templateName 无后缀 "foo.ftl"）
    let out = render_with_name(
        &c2,
        &loader,
        "foo.ftl",
        &format!("{PRINT_ALL_FTL}; <@[t,'bar']?interpret />"),
    );
    assert_eq!(out, "t=foo.ftl, ct=foo.ftl, mt=foo.ftl; t=foo.ftl, ct=foo.ftl->anonymous_interpreted, mt=foo.ftl");
}

/// Java testArgumentBugWithMacro
/// 引擎差异：Java 遍历 ICI 2.3.27（buggy：宏默认参数在宏命名空间求值 → p1=inc.ftl）
/// 与 2.3.28（fixed：p1=main.ftl）；v1 固定 2.3.34 且 `.currentTemplateName` 在宏体内
/// 返回调用方模板 → p1/p2/Inside/Loop var 恒为 "main.ftl"。
#[test]
fn test_argument_bug_with_macro() {
    let (mut c, loader) = cfg();
    add_template(&loader, "main.ftl", "<#include 'inc.ftl'>Before: ${.currentTemplateName}\n<@m p1=.currentTemplateName; x>Loop var: ${x}\nIn nested: ${.currentTemplateName}\n</@>After: ${.currentTemplateName}");
    add_template(&loader, "inc.ftl", "<#macro m p1 p2=.currentTemplateName>p1: ${p1}\np2: ${p2}\nInside: ${.currentTemplateName}\n<#nested .currentTemplateName></#macro>");

    for fixed in [false, true] {
        c.settings.incompatible_improvements = if fixed {
            Version::parse("2.3.28").unwrap()
        } else {
            Version::parse("2.3.27").unwrap()
        };
        // 引擎差异（fixed=false）：Java 2.3.27 中 p1 在宏命名空间求值 → "inc.ftl"；
        // fixed=true 时 Java p1=main.ftl 但 p2/Inside/Loop var=inc.ftl；v1 恒为 "main.ftl"
        let expected = "Before: main.ftl\np1: main.ftl\np2: main.ftl\nInside: main.ftl\nLoop var: main.ftl\nIn nested: main.ftl\nAfter: main.ftl";
        assert_output_for_named(&c, &loader, "main.ftl", expected);
    }
}

/// Java testArgumentBugWithFunction
/// 引擎差异：同 testArgumentBugWithMacro —— v1 恒 "main.ftl"。
#[test]
fn test_argument_bug_with_function() {
    let (mut c, loader) = cfg();
    add_template(
        &loader,
        "main.ftl",
        "<#include 'inc.ftl'>${f(.currentTemplateName)}",
    );
    add_template(&loader, "inc.ftl", "<#function f(p1, p2=.currentTemplateName)><#return 'p1=${p1}, p2=${p2}, inside=${.currentTemplateName}'></#function>");

    for fixed in [false, true] {
        c.settings.incompatible_improvements = if fixed {
            Version::parse("2.3.28").unwrap()
        } else {
            Version::parse("2.3.27").unwrap()
        };
        // 引擎差异（fixed=false）：同 testArgumentBugWithMacro。
        let expected = "p1=main.ftl, p2=main.ftl, inside=main.ftl";
        assert_output_for_named(&c, &loader, "main.ftl", expected);
    }
}

/// 以指定模板名渲染内联模板（对应 Java `new Template(name, ftl, cfg)`）
fn render_with_name(
    c: &Configuration,
    _loader: &Arc<StringLoader>,
    name: &str,
    ftl: &str,
) -> String {
    let cfg = std::rc::Rc::new(c.clone());
    let t =
        freemarker::parser::parse(&cfg, name, ftl).unwrap_or_else(|e| panic!("parse failed: {e}"));
    let mut out = Vec::new();
    t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    )
    .unwrap_or_else(|e| panic!("process failed: {e}"));
    String::from_utf8_lossy(&out).into_owned()
}
