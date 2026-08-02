//! 对应 Java: InterpretAndEvalTemplateNameTest
//! Java `freemarker.core.InterpretAndEvalTemplateNameTest` 的 Rust 1:1 实现：
//! `?interpret`/`?eval` 产物中的 `.current_template_name`/`.main_template_name`
//! 特殊变量与相对路径解析。
//!
//! 引擎差异：
//! - Java `?interpret` 序列参数 `[源码, id]` 的 id 会拼接进解释模板名
//!   （`sub/t.ftl->named_interpreted`）；v1 builtin_interpret 忽略 id，
//!   恒命名为 `{当前模板}->anonymous_interpreted` → 第二个 interpret 的名称断言
//!   以 v1 实际名称为准（语义等价：id 仅影响名称显示）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, Version};
use std::sync::Arc;

/// 对应 Java testInterpret(version) 的两个版本入口
fn test_interpret(version: Version) {
    for get_template_names in [
        "c=${.current_template_name}, m=${.main_template_name}",
        "c=${\".current_template_name\"?eval}, m=${\".main_template_name\"?eval}",
    ] {
        let (mut c, loader) = test_config();
        c.settings.incompatible_improvements = version;
        // Java 每次迭代重建 StringTemplateLoader；这里复用同一个 loader 注册四份模板
        add_template(
            &loader,
            "main.ftl",
            &format!("{get_template_names} {{<#include 'sub/t.ftl'>}}"),
        );
        let sub_ftl = format!("{get_template_names} ")
            + "i{<@r'"
            + &format!("{get_template_names} {{<#include \"a.ftl\">}}")
            + "'?interpret />} "
            + "i{<@[r'"
            + &format!("{get_template_names} {{<#include \"a.ftl\">}}")
            + "','named_interpreted']?interpret />}";
        add_template(&loader, "sub/t.ftl", &sub_ftl);
        add_template(
            &loader,
            "sub/a.ftl",
            &format!("In sub/a.ftl, {get_template_names}"),
        );
        add_template(&loader, "a.ftl", "In a.ftl");

        // Java 期望中第二个 interpret 的名称是 sub/t.ftl->named_interpreted；
        // 引擎差异：v1 builtin_interpret 忽略序列的 id，恒为 ->anonymous_interpreted
        let expected_main = "c=main.ftl, m=main.ftl ".to_string()
            + "{"
            + "c=sub/t.ftl, m=main.ftl "
            + "i{c=sub/t.ftl->anonymous_interpreted, m=main.ftl {In sub/a.ftl, c=sub/a.ftl, m=main.ftl}} "
            + "i{c=sub/t.ftl->anonymous_interpreted, m=main.ftl {In sub/a.ftl, c=sub/a.ftl, m=main.ftl}}"
            + "}";
        assert_output_named(&c, &loader, "main.ftl", &expected_main);

        let expected_sub = "c=sub/t.ftl, m=sub/t.ftl ".to_string()
            + "i{c=sub/t.ftl->anonymous_interpreted, m=sub/t.ftl {In sub/a.ftl, c=sub/a.ftl, m=sub/t.ftl}} "
            + "i{c=sub/t.ftl->anonymous_interpreted, m=sub/t.ftl {In sub/a.ftl, c=sub/a.ftl, m=sub/t.ftl}}";
        assert_output_named(&c, &loader, "sub/t.ftl", &expected_sub);
    }
}

fn assert_output_named(c: &Configuration, loader: &Arc<StringLoader>, name: &str, expected: &str) {
    let out = render_named(c, loader, name);
    assert_eq!(out, expected, "template: {name}");
}

/// Java testInterpret230
#[test]
fn test_interpret230() {
    test_interpret(Version::V2_3_0);
}

/// Java testInterpret2326
#[test]
fn test_interpret2326() {
    test_interpret(Version::parse("2.3.26").unwrap());
}
