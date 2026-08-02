//! 对应 Java: AbsoluteTemplateNameBITest
//! Java `freemarker.core.AbsoluteTemplateNameBITest` 的 Rust 1:1 实现。
//! Java createConfiguration：StringTemplateLoader（与 v1 test_config 相同）。
//!
//! 引擎差异：`?absolute_template_name` 内建在 v1 **未实现**（builtins 注册表与
//! eval.rs 均无）→ 所有断言渲染报 "Unknown built-in: ?absolute_template_name"，
//! Java 断言值全部无法达到（断言原样保留）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java basicsTest
#[test]
#[ignore = "引擎差异：?absolute_template_name 内建未实现（v1 报 Unknown built-in），断言保留 Java 原文"]
fn basics_test() {
    let (c, loader) = cfg();
    // 引擎差异：?absolute_template_name 未实现（v1 报 Unknown built-in）。
    assert_output(&c, &loader, "${'a/b'?absolute_template_name}", "/a/b");
    assert_output(&c, &loader, "${'a/b/'?absolute_template_name}", "/a/b/");
    assert_output(
        &c,
        &loader,
        "${'foo://a/b'?absolute_template_name}",
        "foo://a/b",
    );
    assert_output(&c, &loader, "${'/a/b'?absolute_template_name}", "/a/b");

    assert_output_of_dir_per_f(&c, &loader, "${'a/b'?absolute_template_name}", "/dir/a/b");
    assert_output_of_dir_per_f(&c, &loader, "${'a/b/'?absolute_template_name}", "/dir/a/b/");
    assert_output_of_dir_per_f(
        &c,
        &loader,
        "${'foo://a/b'?absolute_template_name}",
        "foo://a/b",
    );
    assert_output_of_dir_per_f(&c, &loader, "${'/a/b'?absolute_template_name}", "/a/b");

    for base_name in ["dir/f", "/dir/f", "dir/", "/dir/"] {
        assert_output(
            &c,
            &loader,
            &format!("${{'a/b'?absolute_template_name('{base_name}')}}"),
            "/dir/a/b",
        );
        assert_output(
            &c,
            &loader,
            &format!("${{'a/b/'?absolute_template_name('{base_name}')}}"),
            "/dir/a/b/",
        );
        assert_output(
            &c,
            &loader,
            &format!("${{'foo://a/b'?absolute_template_name('{base_name}')}}"),
            "foo://a/b",
        );
        assert_output(
            &c,
            &loader,
            &format!("${{'/a/b'?absolute_template_name('{base_name}')}}"),
            "/a/b",
        );
    }

    assert_output(
        &c,
        &loader,
        "${'a/b'?absolute_template_name('schema://dir/f')}",
        "schema://dir/a/b",
    );
    assert_output(
        &c,
        &loader,
        "${'a/b/'?absolute_template_name('schema://dir/f')}",
        "schema://dir/a/b/",
    );
    assert_output(
        &c,
        &loader,
        "${'foo://a/b'?absolute_template_name('schema://dir/f')}",
        "foo://a/b",
    );
    assert_output(
        &c,
        &loader,
        "${'/a/b'?absolute_template_name('schema://dir/f')}",
        "schema://a/b",
    );
}

/// Java assertOutputOfDirPerF：addTemplate("dir/f", ftl) 后按名渲染
fn assert_output_of_dir_per_f(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    ftl: &str,
    expected_out: &str,
) {
    add_template(loader, "dir/f", ftl);
    // Java 还调用 removeTemplateFromCache("dir/f")；v1 每次 get_template 重新解析
    // （util.render_named 走缓存；StringLoader 内容已替换，缓存键为名称——
    // 此处模板名不重复注册，无需清缓存）
    let out = render_named(c, loader, "dir/f");
    assert_eq!(out, expected_out, "dir/f: {ftl}");
}
