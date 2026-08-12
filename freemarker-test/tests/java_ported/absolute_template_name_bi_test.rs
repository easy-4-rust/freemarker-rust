//! 对应 Java: AbsoluteTemplateNameBITest
//! Java `freemarker.core.AbsoluteTemplateNameBITest` 的 Rust 1:1 实现。
//! Java createConfiguration：StringTemplateLoader（与 v1 test_config 相同）。

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
fn basics_test() {
    let (c, loader) = cfg();
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
    // Java 调用 removeTemplateFromCache("dir/f")——v1 用 clear_template_cache
    // 等价（模板缓存 1 秒 delay 内同名复用，须清缓存才能取到新内容）
    c.clear_template_cache();
    let out = render_named(c, loader, "dir/f");
    assert_eq!(out, expected_out, "dir/f: {ftl}");
}
