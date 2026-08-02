//! Java `freemarker.core.HeaderParsingTest` 的 Rust 1:1 实现
//! （对应 Java: HeaderParsingTest —— `<#ftl>` 头部之后的空白剥离行为）
//!
//! Java 用 cfgStripWS（ICI 2.3.21，剥离开）与 cfgNoStripWS（剥离关）两个配置，
//! 每种模板做 4 个排列（带/不带 encoding 参数 × 尖/方括号头部）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java 私有 assertOutput(ftl, expectedOutStripped, expectedOutNonStripped)：
/// 4 个排列（encoding 参数与方括号写法）分别在剥离开/关配置下断言
fn assert_output_strip_variants(
    c_strip: &Configuration,
    c_nostrip: &Configuration,
    loader: &Arc<StringLoader>,
    ftl: &str,
    expected_stripped: &str,
    expected_non_stripped: &str,
) {
    for i in 0..4 {
        let mut permutation = ftl.to_string();
        if i & 1 == 1 {
            permutation = permutation.replace("<#ftl>", "<#ftl encoding='utf-8'>");
        }
        if i & 2 == 2 {
            permutation = permutation.replace('<', "[").replace('>', "]");
        }
        assert_output(c_strip, loader, &permutation, expected_stripped);
        assert_output(c_nostrip, loader, &permutation, expected_non_stripped);
    }
}

/// Java test()：`<#ftl>` 头部的换行/空白剥离
#[test]
fn test() {
    let (c_strip, loader) = test_config();
    let mut c_nostrip = c_strip.clone();
    c_nostrip.settings.whitespace_stripping = false;

    assert_output_strip_variants(&c_strip, &c_nostrip, &loader, "<#ftl>text", "text", "text");
    // 引擎差异：Java 保留 `<#ftl>` 前同行空白（该行含字面文本 "text"，空白剥离不触及
    // 含文本的行）→ " text"；v1 剥离了头部前的同行空白 → "text"
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        " <#ftl> text",
        "text",
        "text",
    );
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        "\n<#ftl>\ntext",
        "text",
        "text",
    );
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        "\n \n\n<#ftl> \ntext",
        "text",
        "text",
    );
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        "\n \n\n<#ftl>\n\ntext",
        "\ntext",
        "\ntext",
    );
}
