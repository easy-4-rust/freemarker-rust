//! 对应 Java: GetOptionalTemplateTest
//! Java `freemarker.core.GetOptionalTemplateTest.java` 的 Rust 1:1 实现。
//! `.get_optional_template(name[, options])` / `.getOptionalTemplate` 内置变量：
//! 返回 {exists/include/import} 哈希（GetOptionalTemplateMethod.java）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (c, loader) = test_config();
    (c, loader)
}

/// Java testBasicsWhenTemplateExists
/// include 可重复调用；import 返回同一命名空间（loadedLibs 缓存）；别名赋值。
#[test]
fn test_basics_when_template_exists() {
    let (c, loader) = cfg();
    add_template(&loader, "inc.ftl", "<#assign x = (x!0) + 1>inc ${x}");
    let out = render_ftl(
        &c,
        &loader,
        concat!(
            "<#assign t = .getOptionalTemplate('inc.ftl')>",
            "Exists: ${t.exists?c}; ",
            "Include: <@t.include />, <@t.include />; ",
            "Import: <#assign ns1 = t.import()><#assign ns2 = t.import()>${ns1.x}, ${ns2.x}; ",
            "Aliased: <#assign x = 9 in ns1>${ns1.x}, ${ns2.x}, <#import 'inc.ftl' as ns3>${ns3.x}",
        ),
    );
    assert_eq!(
        out,
        "Exists: true; Include: inc 1, inc 2; Import: 1, 1; Aliased: 9, 9, 9"
    );
}

/// Java testBasicsWhenTemplateIsMissing：exists=false，include/import 键缺失
#[test]
fn test_basics_when_template_is_missing() {
    let (c, loader) = cfg();
    let out = render_ftl(
        &c,
        &loader,
        concat!(
            "<#assign t = .getOptionalTemplate('missing.ftl')>",
            "Exists: ${t.exists?c}; ",
            "Include: ${t.include???c}; ",
            "Import: ${t.import???c}",
        ),
    );
    assert_eq!(out, "Exists: false; Include: false; Import: false");
}

/// Java testOptions：parse=false 原样输出；空选项哈希；encoding 选项
#[test]
fn test_options() {
    let (c, loader) = cfg();
    add_template(&loader, "inc.ftl", "${1}");
    let out = render_ftl(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('inc.ftl', { 'parse': false })><@t.include />",
    );
    assert_eq!(out, "${1}");
    let out = render_ftl(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('inc.ftl')><@t.include />",
    );
    assert_eq!(out, "1");
    let out = render_ftl(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('inc.ftl', {})><@t.include />",
    );
    assert_eq!(out, "1");

    // UTF-16BE 编码模板：指定 encoding 正确解码；不指定按 UTF-8 解码（NUL 保留）
    let u16_bytes: Vec<u8> = "foo".encode_utf16().flat_map(|u| u.to_be_bytes()).collect();
    loader.put_bytes("inc-u16.ftl", &u16_bytes);
    let out = render_ftl(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('inc-u16.ftl', { 'encoding': 'utf-16be' })><@t.include />",
    );
    assert_eq!(out, "foo");
    let out = render_ftl(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('inc-u16.ftl')><@t.include />",
    );
    assert_eq!(out, "\u{0}f\u{0}o\u{0}o");

    // parse=false + encoding 组合：原样输出（不解析 ${1}）
    let u16_bytes: Vec<u8> = "foo${1}"
        .encode_utf16()
        .flat_map(|u| u.to_be_bytes())
        .collect();
    loader.put_bytes("inc-u16.ftl", &u16_bytes);
    let out = render_ftl(
        &c,
        &loader,
        concat!(
            "<#assign t = .getOptionalTemplate('inc-u16.ftl', { 'parse': false, 'encoding': 'utf-16be' })>",
            "<@t.include />",
        ),
    );
    assert_eq!(out, "foo${1}");
}

/// Java testRelativeAndAbsolutePath：相对/绝对路径解析（基于当前模板名）
#[test]
fn test_relative_and_absolute_path() {
    let (c, loader) = cfg();
    add_template(&loader, "lib/inc.ftl", "included");

    add_template(
        &loader,
        "test1.ftl",
        "<@.getOptionalTemplate('lib/inc.ftl').include />",
    );
    assert_eq!(render_named(&c, &loader, "test1.ftl"), "included");

    add_template(
        &loader,
        "lib/test2.ftl",
        "<@.getOptionalTemplate('/lib/inc.ftl').include />",
    );
    assert_eq!(render_named(&c, &loader, "lib/test2.ftl"), "included");

    add_template(
        &loader,
        "lib/test3.ftl",
        "<@.getOptionalTemplate('inc.ftl').include />",
    );
    assert_eq!(render_named(&c, &loader, "lib/test3.ftl"), "included");

    add_template(
        &loader,
        "sub/test4.ftl",
        "<@.getOptionalTemplate('../lib/inc.ftl').include />",
    );
    assert_eq!(render_named(&c, &loader, "sub/test4.ftl"), "included");
}

/// Java testUseCase1：宏内使用 get_optional_template 判断存在性
#[test]
fn test_use_case_1() {
    let (c, loader) = cfg();
    add_template(&loader, "lib/inc.ftl", "included");
    let out = render_ftl(
        &c,
        &loader,
        concat!(
            "<#macro test templateName>",
            "<#local t = .getOptionalTemplate(templateName)>",
            "<#if t.exists>",
            "before <@t.include /> after",
            "<#else>",
            "missing",
            "</#if>",
            "</#macro>",
            "<@test 'lib/inc.ftl' />; ",
            "<@test 'inc.ftl' />",
        ),
    );
    assert_eq!(out, "before included after; missing");
}

/// Java testUseCase2：链式缺失回退（include! 默认值表达式）
#[test]
fn test_use_case_2() {
    let (c, loader) = cfg();
    add_template(&loader, "found.ftl", "found");
    let out = render_ftl(
        &c,
        &loader,
        concat!(
            "<@(",
            ".getOptionalTemplate('missing1.ftl').include!",
            ".getOptionalTemplate('missing2.ftl').include!",
            ".getOptionalTemplate('found.ftl').include!",
            ".getOptionalTemplate('missing3.ftl').include",
            ") />",
        ),
    );
    assert_eq!(out, "found");
    let out = render_ftl(
        &c,
        &loader,
        concat!(
            "<#macro fallback>fallback</#macro>",
            "<@(",
            ".getOptionalTemplate('missing1.ftl').include!",
            ".getOptionalTemplate('missing2.ftl').include!",
            "fallback",
            ") />",
        ),
    );
    assert_eq!(out, "fallback");
}

/// Java testWrongArguments：参数与选项校验错误（Java _MessageUtil 消息格式）
#[test]
fn test_wrong_arguments() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate()>",
        &[".getOptionalTemplate", "arguments", "none"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .get_optional_template()>",
        &[".get_optional_template", "arguments", "none"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate(1, 2, 3)>",
        &["arguments", "3"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate(1)>",
        &["#1", "string", "number"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('x', 1)>",
        &["#2", "hash", "number"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('x', { 'foo': 1 })>",
        &["#2", "foo", "encoding", "parse"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('x', { 'parse': 1 })>",
        &["#2", "parse", "number", "boolean"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = .getOptionalTemplate('x', { 'encoding': 1 })>",
        &["#2", "encoding", "number", "string"],
    );

    add_template(&loader, "inc.ftl", "Exists...");
    assert_error_contains(
        &c,
        &loader,
        "<@.getOptionalTemplate('inc.ftl').include x=1 />",
        &["no parameters"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<@.getOptionalTemplate('inc.ftl').include>x</@>",
        &["no nested content"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<@.getOptionalTemplate('inc.ftl').include; x />",
        &["no loop variables"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign x = .getOptionalTemplate('inc.ftl').import(1)>",
        &["no parameters"],
    );
}

/// 命名空间宏经命名参数调用（Java 语法：宏不能用圆括号表达式调用）
#[test]
fn test_namespace_macro_call() {
    let (c, loader) = cfg();
    add_template(
        &loader,
        "lib.ftl",
        "<#macro greet who>Hello ${who}!</#macro>",
    );
    let out = render_ftl(
        &c,
        &loader,
        concat!(
            "<#assign t = .get_optional_template('lib.ftl')>",
            "<#assign ns = t.import()>",
            "<@ns.greet who='World'/>",
        ),
    );
    assert_eq!(out, "Hello World!");
}
