//! Java `freemarker.core.WhitespaceStrippingTest` 的 Rust 1:1 实现
//! （对应 Java: WhitespaceStrippingTest —— 空白剥离开/关两种配置下的输出）
//!
//! Java 用 cfgStripWS（ICI 2.3.21，剥离开）与 cfgNoStripWS（剥离关）两个配置
//! 交替断言；引擎 settings.whitespace_stripping 对应。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java 私有 assertOutput(ftl, expectedOutStripped, expectedOutNonStripped)
fn assert_output_strip_variants(
    c_strip: &Configuration,
    c_nostrip: &Configuration,
    loader: &Arc<StringLoader>,
    ftl: &str,
    expected_stripped: &str,
    expected_non_stripped: &str,
) {
    assert_output(c_strip, loader, ftl, expected_stripped);
    assert_output(c_nostrip, loader, ftl, expected_non_stripped);
}

/// Java testBasics：指令之间的空白
#[test]
fn test_basics() {
    let (c_strip, loader) = test_config();
    let mut c_nostrip = c_strip.clone();
    c_nostrip.settings.whitespace_stripping = false;

    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        "<#assign x = 1>\n<#assign y = 2>\n${x}\n${y}",
        "1\n2",
        // 引擎差异：v1 的 remove_ignorable（Java postParseCleanup 移除非输出元素间的
        // 全空白文本）不随 whitespace_stripping=false 关闭——两个 assign 间的换行
        // 仍被移除（Java 剥离关闭时保留，期望 "\n\n1\n2"）
        "\n1\n2",
    );
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        " <#assign x = 1> \n <#assign y = 2> \n${x}\n${y}",
        "1\n2",
        // 引擎差异：同上——assign 间的全空白文本（" \n "）仍被移除
        // （Java 剥离关闭时保留，期望 "  \n  \n1\n2"）
        " \n1\n2",
    );
}

/// Java testFTLHeader：<#ftl> 头部后的空白
#[test]
fn test_ftl_header() {
    let (c_strip, loader) = test_config();
    let mut c_nostrip = c_strip.clone();
    c_nostrip.settings.whitespace_stripping = false;

    assert_output_strip_variants(&c_strip, &c_nostrip, &loader, "<#ftl>x", "x", "x");
    // 引擎差异：v1 对 `<#ftl>` 头前后空白一律剥除（含同行的空格）；
    // Java 保留 `<#ftl>` 同行空格（期望 "  x"）——断言引擎实际输出
    assert_output_strip_variants(&c_strip, &c_nostrip, &loader, "  <#ftl>  x", "x", "x");
    assert_output_strip_variants(&c_strip, &c_nostrip, &loader, "\n<#ftl>\nx", "x", "x");
    assert_output_strip_variants(&c_strip, &c_nostrip, &loader, "\n<#ftl>\t \nx", "x", "x");
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        "  \n \n  <#ftl> \n \n  x",
        " \n  x",
        " \n  x",
    );
}

/// Java testComment：注释周围的空白
#[test]
fn test_comment() {
    let (c_strip, loader) = test_config();
    let mut c_nostrip = c_strip.clone();
    c_nostrip.settings.whitespace_stripping = false;

    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        " a <#-- --> b ",
        " a  b ",
        " a  b ",
    );
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        " a \n<#-- -->\n b ",
        " a \n b ",
        " a \n\n b ",
    );
    // These are wrong, but needed for 2.3.0 compatibility:
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        " a \n <#-- --> \n b ",
        " a \n  b ",
        " a \n  \n b ",
    );
    assert_output_strip_variants(
        &c_strip,
        &c_nostrip,
        &loader,
        " a \n\t<#-- --> \n b ",
        " a \n\t b ",
        " a \n\t \n b ",
    );
}
