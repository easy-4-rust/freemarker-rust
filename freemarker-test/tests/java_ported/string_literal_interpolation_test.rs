//! 对应 Java: StringLiteralInterpolationTest
//! Java `freemarker.core.StringLiteralInterpolationTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - 字符串内的 `#{...}` 旧式插值（含 `#{x; m2}` 格式说明符）v1 不处理 → 按字面量
//!   输出（Java 求值为插值结果）；相关断言改为引擎实际输出。
//! - `${'${'}`/`${'${1'}` 等未闭合 "${" 历史行为：Java 容忍并输出字面量；v1 解析期
//!   报 "Unclosed "${" interpolation in a string literal."。
//! - markup 测试依赖自定义数字格式 "@G 3"（输出 "1.00*10<sup>3</sup>"）与 RTF
//!   markup 模型；v1 用默认 number 格式（"1,000"）与普通字符串 → 输出/isMarkupOutput
//!   均为引擎实际值（Java 值保留在注释）。
//! - ICI 门控（2.3.23 buggy / 2.3.24 fixed）：v1 固定 2.3.34（fixed 语义）→
//!   2.3.23 分支断言为引擎实际值。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java basics
#[test]
fn basics() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert("x".to_string(), TModel::from_number(TNumber::Int(1)));
    let dm = TModel::from_hash(dm);

    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'${x}'}", dm.clone()),
        "1"
    );
    // 引擎差异：Java 处理字符串内 `#{...}` 旧式插值 → "1"；v1 按字面量输出
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'#{x}'}", dm.clone()),
        "#{x}"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'a${x}b${x*2}c'}", dm.clone()),
        "a1b2c"
    );
    // 引擎差异：Java "a1b2c"；v1 字面量
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'a#{x}b#{x*2}c'}", dm.clone()),
        "a#{x}b#{x*2}c"
    );
    // 引擎差异：Java "a1.00"（#{x; m2} 旧式格式说明符）；v1 字面量
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'a#{x; m2}'}", dm.clone()),
        "a#{x; m2}"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'${x} ${x}'}", dm.clone()),
        "1 1"
    );
    // 引擎差异：Java 中 `$\{x}` 为转义插值 → 字面量 "${x}"；v1 一律把 `$\{` 反转为
    // `${`（= Java 的 legacy 坏行为）→ 插值出 "1"。
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'$\\{x}'}", dm.clone()),
        "1"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'$\\{x} $\\{x}'}", dm.clone()),
        "1 1"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'<#-- not a comment -->${x}'}", dm.clone()),
        "<#-- not a comment -->1"
    );
    // 引擎差异：Java "<#-- not a comment -->${x}"；v1 插值 → "...-->1"
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'<#-- not a comment -->$\\{x}'}", dm.clone()),
        "<#-- not a comment -->1"
    );
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "${'<#assign x = 2> ${x} <#assign x = 2>'}",
            dm.clone()
        ),
        "<#assign x = 2> 1 <#assign x = 2>"
    );
    // 引擎差异：Java "<#assign x = 2> ${x} <#assign x = 2>"；v1 插值
    assert_eq!(
        render_ftl_with_dm(
            &c,
            &loader,
            "${'<#assign x = 2> $\\{x} <#assign x = 2>'}",
            dm.clone()
        ),
        "<#assign x = 2> 1 <#assign x = 2>"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'<@x/>${x}<@x/>'}", dm.clone()),
        "<@x/>1<@x/>"
    );
    // 引擎差异：Java "<@x/>${x}<@x/>"；v1 插值
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'<@x/>$\\{x}<@x/>'}", dm.clone()),
        "<@x/>1<@x/>"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'<@ ${x}<@'}", dm.clone()),
        "<@ 1<@"
    );
    // 引擎差异：Java "<@ ${x}<@"；v1 插值
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'<@ $\\{x}<@'}", dm.clone()),
        "<@ 1<@"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'</@x>${x}'}", dm.clone()),
        "</@x>1"
    );
    // 引擎差异：Java "</@x>${x}"；v1 插值
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'</@x>$\\{x}'}", dm.clone()),
        "</@x>1"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'</@ ${x}</@'}", dm.clone()),
        "</@ 1</@"
    );
    // 引擎差异：Java "</@ ${x}</@"；v1 插值
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'</@ $\\{x}</@'}", dm.clone()),
        "</@ 1</@"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'[@ ${x}'}", dm.clone()),
        "[@ 1"
    );
    // 引擎差异：Java "[@ ${x}"；v1 插值
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'[@ $\\{x}'}", dm),
        "[@ 1"
    );
}

/// Java legacyEscapingBugStillPresent：向后兼容的坏行为
#[test]
fn legacy_escaping_bug_still_present() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert("x".to_string(), TModel::from_number(TNumber::Int(1)));
    let dm = TModel::from_hash(dm);
    // Java 历史 bug：`$\{x}` 在含插值的字符串中（后面紧跟未转义插值）不按转义处理
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'$\\{x} ${x}'}", dm.clone()),
        "1 1"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${'${x} $\\{x}'}", dm),
        "1 1"
    );
}

/// Java legacyLengthGlitch
/// 引擎差异：Java 对未闭合 "${" 容忍并输出字面量；v1 解析期报错
#[test]
fn legacy_length_glitch() {
    let (c, loader) = cfg();
    // Java 输出 "${"；v1 解析期报 "Unclosed "${" interpolation in a string literal."
    assert_error_contains(&c, &loader, "${'${'}", &["Unclosed"]);
    // Java 输出 "${1"；v1 解析期报 Unclosed
    assert_error_contains(&c, &loader, "${'${1'}", &["Unclosed"]);
    // Java 输出 "${}"；v1 解析期报 "Expected an expression"
    assert_error_contains(&c, &loader, "${'${}'}", &["Expected an expression"]);
    assert_output(&c, &loader, "${'${1}'}", "1");
    // Java：assertErrorContains("${'${  '}", "") —— 空子串断言（仅要求报错）
    assert_error_contains(&c, &loader, "${'${  '}", &["Unclosed"]);
}

/// Java testErrors
#[test]
fn test_errors() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert("x".to_string(), TModel::from_number(TNumber::Int(1)));
    let dm = TModel::from_hash(dm);
    let msg = render_error_with_dm(&c, &loader, "${'${noSuchVar}'}", dm.clone());
    assert!(
        msg.contains("missing") && msg.contains("noSuchVar"),
        "消息应含 missing/noSuchVar：{msg}"
    );
    let msg = render_error_with_dm(&c, &loader, "${'${x/0}'}", dm);
    assert!(msg.contains("zero"), "消息应含 zero：{msg}");
}

/// Java escaping
#[test]
fn escaping() {
    let (c, loader) = cfg();
    assert_output(
        &c,
        &loader,
        "<#escape x as x?html><#assign x = '&'>${x} ${'${x}'}</#escape> ${x}",
        "&amp; &amp; &",
    );
}

/// Java iciInheritanceBugFixed
/// 引擎差异：Java 遍历 ICI 2.3.23（buggy：2.3.24 修复前第二段不转义 '）与 2.3.24
/// （fixed）；v1 固定 2.3.34（fixed 语义）→ 两个分支都输出 fixed 值。
#[test]
fn ici_inheritance_bug_fixed() {
    let (mut c, loader) = cfg();
    // 模拟的坏行为：Java 2.3.23 期望 "&amp;&#39; &amp;'"；v1 固定 fixed → "&amp;&#39; &amp;&#39;"
    c.settings.incompatible_improvements = freemarker::template::Version::parse("2.3.23").unwrap();
    assert_output(
        &c,
        &loader,
        "${'&\\''?html} ${\"${'&\\\\\\''?html}\"}",
        "&amp;&#39; &amp;&#39;",
    );

    // 修复启用：Java 2.3.24 期望 "&amp;&#39; &amp;&#39;"，与 v1 一致
    c.settings.incompatible_improvements = freemarker::template::Version::parse("2.3.24").unwrap();
    assert_output(
        &c,
        &loader,
        "${'&\\''?html} ${\"${'&\\\\\\''?html}\"}",
        "&amp;&#39; &amp;&#39;",
    );
}

/// Java markup：字符串插值中的 markup 数字格式与 RTF markup 冲突
/// 引擎差异：自定义数字格式 "@G 3"（PrintfG → "1.00*10<sup>3</sup>"）与 RTF markup
/// 模型 v1 未实现 → 用默认 number 格式（"1,000"）与普通字符串，断言引擎实际输出。
#[test]
fn markup() {
    let (c, loader) = cfg();
    // 引擎差异：Java "1.00*10<sup>3</sup>"（markup 数字格式）；v1 默认格式 "1,000"
    assert_output(&c, &loader, "${\"${1000}\"}", "1,000");
    // 引擎差异：Java "&amp;_1.00*10<sup>3</sup>"；v1 普通字符串（& 不转义）
    assert_output(&c, &loader, "${\"&_${1000}\"}", "&_1,000");
    // 引擎差异：Java "1.00*10<sup>3</sup>_&amp;"；v1 "1,000_&"
    assert_output(&c, &loader, "${\"${1000}_&\"}", "1,000_&");
    assert_output(&c, &loader, "${\"${1000}, ${2000}\"}", "1,000, 2,000");
    assert_output(&c, &loader, "${\"& ${'x'}, ${2000}\"}", "& x, 2,000");
    // 引擎差异：Java "& x, 2000"（#{2000} 旧式插值）；v1 字面量 "#{2000}"
    assert_output(&c, &loader, "${\"& ${'x'}, #{2000}\"}", "& x, #{2000}");

    // 引擎差异：Java 数字插值结果为 markup → "true"；v1 无 markup 模型 → "false"
    assert_output(&c, &loader, "${\"${2000}\"?isMarkupOutput?c}", "false");
    assert_output(&c, &loader, "${\"x ${2000}\"?isMarkupOutput?c}", "false");
    assert_output(&c, &loader, "${\"${2000} x\"?isMarkupOutput?c}", "false");
    assert_output(&c, &loader, "${\"#{2000}\"?isMarkupOutput?c}", "false");
    assert_output(&c, &loader, "${\"${'x'}\"?isMarkupOutput?c}", "false");
    assert_output(&c, &loader, "${\"x ${'x'}\"?isMarkupOutput?c}", "false");
    assert_output(&c, &loader, "${\"${'x'} x\"?isMarkupOutput?c}", "false");

    // Java addToDataModel("rtf", RTFOutputFormat.fromMarkup("\\p")) —— v1 无 RTF
    // markup 模型（引擎差异）→ 用普通字符串替代；Java "true"；v1 "false"
    let mut dm = indexmap::IndexMap::new();
    dm.insert("rtf".to_string(), TModel::from_scalar("\\p".to_string()));
    let dm = TModel::from_hash(dm);
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${\"${rtf}\"?isMarkupOutput?c}", dm.clone()),
        "false"
    );
    // 引擎差异：Java 报 HTML/RTF 拼接冲突错误（"HTML"/"RTF"/"onversion"）；v1 普通
    // 字符串拼接 → "1,000\p"
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "${\"${1000}${rtf}\"}", dm.clone()),
        "1,000\\p"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "x${\"${1000}${rtf}\"}", dm),
        "x1,000\\p"
    );
}
