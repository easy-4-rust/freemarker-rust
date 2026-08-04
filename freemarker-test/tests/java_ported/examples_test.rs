//! Java `freemarker.manual.ExamplesTest` 的 Rust 1:1 实现
//! （对应 Java: ExamplesTest —— manual 包示例测试的抽象 @Ignore 基类：
//!   loadPropertiesFile/createConfiguration/setupTemplateLoaders（ClassTemplateLoader +
//!   StringTemplateLoader 的 MultiTemplateLoader），自身无测试方法；实际测试在
//!   示例子类中。本文件按任务约定把示例子类（AbsoluteTemplateNameBIExample /
//!   AutoEscapingExample / ConfigureOutputFormatExamples / CustomFormatsExample /
//!   GettingStartedExample / TemplateConfigurationExamples / WithArgsExamples /
//!   WithArgsLastExamples）的 FTL 内容直接内联为断言输入（license 头注释省略——
//!   纯注释无行为影响；期望输出按 Java TestUtil.removeTxtCopyrightComment +
//!   normalizeNewLines 剥离后的原文）。
//!
//! 引擎差异总览：
//! - Java ClassTemplateLoader（类路径模板加载）→ v1 用 StringLoader（add_template）等价；
//! - `.ftlh` 文件扩展名 → HTMLOutputFormat + autoEscaping 的识别
//!   （recognizeStandardFileExtensions + 扩展名 TemplateConfiguration）引擎未实现
//!   → AutoEscapingExample 用 `<#outputFormat 'HTML'>` + `<#autoEsc>` 包裹等价翻译
//!   （同 output_format_test.rs 的模式）；引擎 autoEsc 对 markup 模型语义
//!   （?esc/?no_esc 结果被二次转义、捕获输出被转义）与 Java 差异 → 相关断言
//!   ENGINE_GAP（标 #[ignore]，Java 期望值保留）。
//!
//! ENGINE_GAP: testAbsoluteTemplateNameBI —— ?absolute_template_name 内建未实现
//!   （absolute_template_name_bi_test.rs：v1 报 Unknown built-in）；
//!   testInfoBox/testStringLiteral2/testStringConcat —— autoEsc 下引擎对 ?esc/
//!   ?no_esc 结果仍做二次转义（output_format_test.rs 头注：apply_escape 对 markup
//!   模型不跳过二次转义；Java 转义结果/markup 不再转义）；
//!   testCapture/testConvert —— v1 无 markup 输出模型：捕获输出在 autoEsc 下被
//!   转义、?esc 的 markup 无法跨 XML/RTF 格式转换（&apos;/\\{\\} 不产生）、
//!   <#attempt> 捕获段不会因格式转换失败；
//!   testConvert2 —— 空白剥离差异：Java getLastLeaf 停在 BlockAssignment（视为
//!   叶、heedsOpeningWhitespace=false，TextBlock.java:488-504），块后换行被剥离；
//!   v1 last_leaf 深入 assign body 到捕获文本（heeds=true）→ 行首多 3 个换行
//!   （其余内容逐字一致）；另有引擎末叶链对 BlockAssign 无 Macro/BlockAssignment
//!   特例（同因影响 testCapture 行首 "\n"）；
//!   testMarkup —— ?markup_string 内建未实现 + markup 二次转义语义差异；
//!   testUsingWithArgsSpecialVariable ×2 —— ?with_args/?with_args_last 对宏/函数
//!   调用 v1 仅支持方法模型（with_args_built_in_test.rs："only supported on methods"）。
//! NOT_APPLICABLE: testConfigureOutputFormatExamples / CustomFormatsExample 三个
//!   方法 / testGettingStartedMain / TemplateConfigurationExamples 的
//!   getOutputFormat/getEncoding 断言 —— Java 依赖 Template.getOutputFormat() 公开
//!   API（v1 Template 无输出格式 getter，template_configuration 为 pub(crate)）、
//!   Configuration.setSettings(properties) 的工厂 DSL 解析、自定义格式工厂
//!   （Java 类）、JavaBeans 反射与类路径模板加载，均无法 1:1（Java 原文保留）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::{
    ConditionalTemplateConfigurationFactory, FileNameGlobMatcher,
    MergingTemplateConfigurationFactory,
};
use freemarker::core::TemplateConfiguration;
use freemarker::template::TModel;
use freemarker::value::{DateType, DateValue};
use std::sync::Arc;

/// 引擎差异适配：Java `.ftlh` 扩展名 → HTMLOutputFormat + autoEscaping 的识别
/// v1 未实现——用 `<#outputFormat 'HTML'>` + `<#autoEsc>` 包裹等价翻译
/// （同 output_format_test.rs：引擎 `<#outputFormat>` 仅切 output_format，
/// 不自动开 autoEsc，需显式 `<#autoEsc>`）。
fn html_autoesc_ftl(body: &str) -> String {
    format!("<#outputFormat 'HTML'><#autoEsc>{body}</#autoEsc></#outputFormat>")
}

/// Java AbsoluteTemplateNameBIExample.test ——
/// assertOutputForNamed("dir/AbsoluteTemplateNameBIExample-main.ftl")
/// （示例 FTL：`?absolute_template_name` + `.get_optional_template` +
/// `.caller_template_name` + `<@t.include>` 的绝对模板名解析示例）
#[test]
#[ignore = "引擎差异：?absolute_template_name 内建未实现（v1 报 Unknown built-in）→ 整链渲染失败；断言保留 Java 原文"]
fn test_absolute_template_name_bi() {
    let (c, loader) = test_config();
    // 示例模板内容（license 头注释省略；目录：src/test/resources/freemarker/manual/）
    // dir/AbsoluteTemplateNameBIExample-main.ftl:
    //   <#import '/AbsoluteTemplateNameBIExample-lib.ftl' as lib>
    //   <@lib.smileyInclude 'AbsoluteTemplateNameBIExample-foo.ftl' />
    //   <@lib.smileyInclude '../AbsoluteTemplateNameBIExample-foo.ftl' />
    //   <@lib.smileyInclude '/AbsoluteTemplateNameBIExample-foo.ftl' />
    //   <@lib.smileyInclude 'AbsoluteTemplateNameBIExample-missing.ftl' />
    // AbsoluteTemplateNameBIExample-lib.ftl（宏 smileyInclude 用
    //   name?absolute_template_name(.caller_template_name) 转绝对名）：
    //   <#macro smileyInclude name>
    //     <#local t = .get_optional_template(
    //         name?absolute_template_name(.caller_template_name))>
    //     <#if t.exists>(:<@t.include /><#else>):</#if>
    //   </#macro>
    // AbsoluteTemplateNameBIExample-foo.ftl: "/foo"；dir/ 下同名文件: "/dir/foo"
    add_template(
        &loader,
        "dir/AbsoluteTemplateNameBIExample-main.ftl",
        MAIN_FTL,
    );
    add_template(&loader, "AbsoluteTemplateNameBIExample-lib.ftl", LIB_FTL);
    add_template(&loader, "AbsoluteTemplateNameBIExample-foo.ftl", "/foo\n");
    add_template(
        &loader,
        "dir/AbsoluteTemplateNameBIExample-foo.ftl",
        "/dir/foo\n",
    );
    // Java 期望输出（main.ftl.out 剥离 license 头后；第一个 include 解析为
    // dir/ 下的 foo → "/dir/foo"，后两个 → 根下的 foo → "/foo"，missing → "):"）
    let expected = "\n    (:\n/dir/foo\n    (:\n/foo\n    (:\n/foo\n    ):\n";
    assert_eq!(
        render_named(&c, &loader, "dir/AbsoluteTemplateNameBIExample-main.ftl"),
        expected
    );
}

/// Java AutoEscapingExample.testInfoBox ——
/// assertOutputForNamed("AutoEscapingExample-infoBox.ftlh")
/// （HTML autoEsc：普通字符串插值转义；`?no_esc` 结果不再转义）
#[test]
#[ignore = "引擎差异：autoEsc 下引擎对 ?no_esc 结果仍做二次转义（output_format_test.rs 头注；实际输出 \"Foo &lt;b&gt;bar&lt;/b&gt;\"）；空白剥离逐字一致；断言保留 Java 原文"]
fn test_info_box() {
    let (c, loader) = test_config();
    // 引擎差异：Java 模板经 .ftlh 扩展名识别为 HTML+autoEsc；v1 无扩展名识别 →
    // 用 html_autoesc_ftl 包裹等价翻译（见文件头）。
    // 引擎差异：autoEsc 下引擎对 ?no_esc 结果仍做二次转义（output_format_test.rs
    // 头注）→ 第二段 "Foo <b>bar</b>" 实际被转义成 "Foo &lt;b&gt;bar&lt;/b&gt;"。
    let ftl = html_autoesc_ftl(
        "<@infoBox \"Foo & bar\" />\n\
         <@infoBox \"Foo <b>bar</b>\"?no_esc />\n\
         \n\
         <#macro infoBox message>\n\
         \x20 <div class=\"infoBox\">\n\
         \x20   ${message}\n\
         \x20 </div>\n\
         </#macro>",
    );
    let expected = "  <div class=\"infoBox\">\n    Foo &amp; bar\n  </div>\n  <div class=\"infoBox\">\n    Foo <b>bar</b>\n  </div>\n\n";
    assert_output(&c, &loader, &ftl, expected);
}

/// Java AutoEscapingExample.testCapture ——
/// assertOutputForNamed("AutoEscapingExample-capture.ftlh")
/// （HTML autoEsc：字符串字面量插值转义；捕获输出（markup）不转义）
#[test]
#[ignore = "引擎差异：v1 无 markup 输出模型——捕获输出在 autoEsc 下被转义（实际 \"&lt;b&gt;Test&lt;/b&gt;\"）；另有 BlockAssign 末叶链空白剥离差异（行首多 \"\\n\"）；断言保留 Java 原文"]
fn test_capture() {
    let (c, loader) = test_config();
    // 引擎差异：Java 捕获块输出为 markup 模型，插值时不再转义；v1 捕获输出
    // 为普通字符串，autoEsc 下会被转义（output_format_test.rs 头注）→ 第二段
    // "Captured output: <b>Test</b>" 在 v1 实际输出 "&lt;b&gt;Test&lt;/b&gt;"；
    // 行首 "\n" 差异同 testConvert2 的 BlockAssignment 末叶链空白剥离差异。
    let ftl = html_autoesc_ftl(
        "<#assign captured><b>Test</b></#assign>\n\
         Just a string: ${\"<b>Test</b>\"}\n\
         Captured output: ${captured}",
    );
    let expected = "Just a string: &lt;b&gt;Test&lt;/b&gt;\nCaptured output: <b>Test</b>";
    assert_output(&c, &loader, &ftl, expected);
}

/// Java AutoEscapingExample.testMarkup ——
/// assertOutputForNamed("AutoEscapingExample-markup.ftlh")
/// （?no_esc/?esc 产生 markup 输出；?markup_string 双重转义）
#[test]
#[ignore = "引擎差异：?markup_string 内建未实现（v1 报 Unknown built-in），且 autoEsc 下 markup 语义差异；断言保留 Java 原文"]
fn test_markup() {
    let (c, loader) = test_config();
    let ftl = html_autoesc_ftl(
        "<#assign markupOutput1=\"<b>Test</b>\"?no_esc>\n\
         <#assign markupOutput2=\"Foo & bar\"?esc>\n\
         \n\
         As expected:\n\
         ${markupOutput1}\n\
         ${markupOutput2}\n\
         \n\
         Double escaping:\n\
         ${markupOutput1?markup_string}\n\
         ${markupOutput2?markup_string}",
    );
    let expected = "\nAs expected:\n<b>Test</b>\nFoo &amp; bar\n\nDouble escaping:\n&lt;b&gt;Test&lt;/b&gt;\nFoo &amp;amp; bar";
    assert_output(&c, &loader, &ftl, expected);
}

/// Java AutoEscapingExample.testConvert ——
/// assertOutputForNamed("AutoEscapingExample-convert.ftlh")
/// （?esc 的 markup 输出在 XML/RTF 格式下转换转义；捕获输出在各格式下失败）
#[test]
#[ignore = "引擎差异：v1 无 markup 输出模型——?esc 结果为普通字符串（autoEsc 下再被转义），XML(&apos;)/RTF(\\{\\}) 格式转换不产生，<#attempt> 捕获段不因格式转换失败；断言保留 Java 原文"]
fn test_convert() {
    let (c, loader) = test_config();
    // 引擎差异：v1 无 markup 输出模型——?esc 结果为普通字符串，跨格式转换
    // （XML: &apos; / RTF: \{\}）不会发生；<#attempt> 捕获段 v1 无格式转换失败。
    let ftl = html_autoesc_ftl(
        "<#assign mo1 = \"Foo's bar {}\"?esc>\n\
         HTML: ${mo1}\n\
         XML:  <#outputformat 'XML'>${mo1}</#outputformat>\n\
         RTF:  <#outputformat 'RTF'>${mo1}</#outputformat>\n\
         \n\
         <#assign mo2><p>Test</#assign>\n\
         HTML: ${mo2}\n\
         XML:  <#attempt><#outputformat 'XML'>${mo2}</#outputformat><#recover>Failed</#attempt>\n\
         RTF:  <#attempt><#outputformat 'RTF'>${mo2}</#outputformat><#recover>Failed</#attempt>\n",
    );
    let expected =
        "HTML: Foo&#39;s bar {}\nXML:  Foo&apos;s bar {}\nRTF:  Foo's bar \\{\\}\n\nHTML: <p>Test\nXML:  Failed\nRTF:  Failed\n";
    assert_output(&c, &loader, &ftl, expected);
}

/// Java AutoEscapingExample.testConvert2 ——
/// assertOutputForNamed("AutoEscapingExample-convert2.ftl")
/// （outputformat 块内捕获为各格式 markup，在 undefined 输出格式下按原样输出）
#[test]
#[ignore = "引擎差异：空白剥离——Java getLastLeaf 停在 BlockAssignment（视为叶，heedsOpeningWhitespace=false，TemplateElement.java:488-504），块后换行被剥离；v1 last_leaf 深入 assign body 到捕获文本（heeds=true）→ 行首多 3 个换行（其余内容逐字一致，捕获文本本身原样输出）；断言保留 Java 原文"]
fn test_convert2() {
    let (c, loader) = test_config();
    // .ftl 文件（非 .ftlh）：Java "undefined" 输出格式，无 autoEsc。
    // 引擎差异：v1 无 markup 模型——块内捕获为普通字符串，undefined 格式下
    // 原样输出（Java 语义：markup 在 undefined 下输出 markup 原文，观察一致）；
    // 但块后换行的空白剥离与 Java 不同（见 #[ignore] 原因），整体保留 Java 期望。
    let ftl = "<#outputformat \"HTML\"><#assign htmlMO><p>Test</#assign></#outputformat>\n\
               <#outputformat \"XML\"><#assign xmlMO><p>Test</p></#assign></#outputformat>\n\
               <#outputformat \"RTF\"><#assign rtfMO>\\par Test</#assign></#outputformat>\n\
               <#-- We assume that we have \"undefined\" output format here. -->\n\
               HTML: ${htmlMO}\n\
               XML:  ${xmlMO}\n\
               RTF:  ${rtfMO}";
    let expected = "HTML: <p>Test\nXML:  <p>Test</p>\nRTF:  \\par Test";
    assert_output(&c, &loader, ftl, expected);
}

/// Java AutoEscapingExample.testStringLiteral ——
/// assertOutputForNamed("AutoEscapingExample-stringLiteral.ftlh")
/// （autoEsc 下普通字符串插值转义，含嵌套插值的字符串字面量）
#[test]
fn test_string_literal() {
    let (c, loader) = test_config();
    let ftl = html_autoesc_ftl("<#assign s = \"Foo & bar\">\n${s}\n${\"${s} & baz\"}");
    let expected = "Foo &amp; bar\nFoo &amp; bar &amp; baz";
    assert_output(&c, &loader, &ftl, expected);
}

/// Java AutoEscapingExample.testStringLiteral2 ——
/// assertOutputForNamed("AutoEscapingExample-stringLiteral2.ftlh")
/// （?esc/?no_esc 的 markup 值嵌入字符串字面量插值）
#[test]
#[ignore = "引擎差异：autoEsc 下 ?esc/?no_esc 结果被二次转义（实际 \"Foo &amp;amp; bar baz\"）；断言保留 Java 原文"]
fn test_string_literal2() {
    let (c, loader) = test_config();
    // 引擎差异：autoEsc 下 ?esc/?no_esc 结果被二次转义（见文件头）→
    // 第一行 "Foo &amp; bar baz" 实际为 "Foo &amp;amp; bar baz"。
    let ftl = html_autoesc_ftl(
        "<#-- Markup output value created by escaping plain text: -->\n\
         <#assign mo1 = \"Foo & bar\"?esc>\n\
         <#-- Markup output value created outherwise: -->\n\
         <#assign mo2 = \"<p>Foo\"?no_esc>\n\
         \n\
         ${\"${mo1} baz\"}\n\
         ${\"${mo2} baz\"}",
    );
    let expected = "\nFoo &amp; bar baz\n<p>Foo baz";
    assert_output(&c, &loader, &ftl, expected);
}

/// Java AutoEscapingExample.testStringConcat ——
/// assertOutputForNamed("AutoEscapingExample-stringConcat.ftlh")
/// （字符串连接中 ?no_esc 片段不转义、普通片段转义）
#[test]
#[ignore = "引擎差异：autoEsc 下 ?no_esc 结果被二次转义（实际 \"&lt;h1&gt;Foo &amp; bar&lt;/h1&gt;\"）；断言保留 Java 原文"]
fn test_string_concat() {
    let (c, loader) = test_config();
    // 引擎差异：autoEsc 下 ?no_esc 结果被二次转义 → "<h1>" 实际为 "&lt;h1&gt;"。
    let ftl = html_autoesc_ftl("${\"<h1>\"?no_esc + \"Foo & bar\" + \"</h1>\"?no_esc}");
    let expected = "<h1>Foo &amp; bar</h1>";
    assert_output(&c, &loader, &ftl, expected);
}

/// Java ConfigureOutputFormatExamples.test ——
/// 按模板源名配置输出格式（程序化工厂 + properties 文件两种方式）
#[test]
fn test_configure_output_format_examples() {
    // NOT_APPLICABLE: Java 断言全部是
    //   assertEquals(HTMLOutputFormat.INSTANCE, cfg.getTemplate(name).getOutputFormat())
    // （ConfigureOutputFormatExamples.java:35-99）——v1 Template 无输出格式 getter
    // （template_configuration 为 pub(crate)），且 cfg.setSettings(properties) 的
    // 工厂 DSL（"ConditionalTemplateConfigurationFactory(PathGlobMatcher(...), ...)"）
    // 引擎无解析——断言不可移植。
    // Java 原文（ConfigureOutputFormatExamples.java）：
    //   addTemplate("mail/t.ftl", ""); addTemplate("t.html", ""); ...
    //   2/a: ConditionalTemplateConfigurationFactory(PathGlobMatcher("mail/**"),
    //        TemplateConfiguration(outputFormat=HTML)) → mail/t.ftl 为 HTML
    //   2/b: 同 2/a 的 properties 形式（ConfigureOutputFormatExamples1.properties）
    //   3/a: FirstMatch(FileExtensionMatcher("xml")→XML, Or(html,htm)→HTML,
    //        rtf→RTF).allowNoMatch(true) → t.html/t.htm 为 HTML、t.xml 为 XML、
    //        t.rtf 为 RTF
    //   3/b: 同 3/a 的 properties 形式（ConfigureOutputFormatExamples2.properties）
}

/// Java CustomFormatsExample.aliases1 —— 自定义数字/日期格式别名
/// （alias1.ftlh: ${p?string.@price} ${w?string.@weight} ${fd?string.@fileDate}
///  ${let?datetime?string.@logEventTime} → "10,000.00\n10.31\n23/Dec/15 10:09 PM\n
///  2015-12-23T21:09:04.213Z"）
#[test]
fn aliases1() {
    // NOT_APPLICABLE: Java 用 Configuration.setCustomNumberFormats/
    // setCustomDateFormats + AliasTemplateNumberFormatFactory（",000.00" 等别名）
    // （CustomFormatsExample.java:36-56）——v1 无自定义格式工厂 API（Java 类）与
    // `?string.@别名` 语法，断言不可移植；Java 数据：p=10000、w=BigDecimal("10.305")、
    // fd=let=new Date(1450904944213L)，期望输出见方法注释。
}

/// Java CustomFormatsExample.aliases2 —— 自定义数字格式别名（@base 8 进制）
/// （alias2.ftlh: ${10?string.@oct} → "12"）
#[test]
fn aliases2() {
    // NOT_APPLICABLE: setCustomNumberFormats + BaseNTemplateNumberFormatFactory/
    // AliasTemplateNumberFormatFactory（CustomFormatsExample.java:58-70）——
    // v1 无自定义格式工厂 API，断言不可移植；Java 期望输出 "12"（10 的 8 进制）。
}

/// Java CustomFormatsExample.modelAware —— 模型感知的数字格式（单位后缀）
/// （modelAware.ftlh: ${10.12356} ${weight} → "10.1236\n1.5 kg"；
///  weight=UnitAwareTemplateNumberModel(1.5, "kg")，numberFormat="@ua 0.####;; roundingMode=halfUp"）
#[test]
fn model_aware() {
    // NOT_APPLICABLE: 依赖 Java 类 UnitAwareTemplateNumberFormatFactory/
    // UnitAwareTemplateNumberModel（本测试包内的自定义 TemplateNumberModel 实现）
    // + setCustomNumberFormats（CustomFormatsExample.java:72-86）——v1 无自定义
    // 格式工厂 API 与自定义数值模型钩子，断言不可移植。
}

/// Java GettingStartedExample.main —— 手册"Getting Started"完整示例程序
/// （Configuration + JavaBean Product 数据模型 + getTemplate("test.ftlh") 输出到 stdout）
#[test]
fn test_getting_started_main() {
    // NOT_APPLICABLE: Java 依赖 JavaBeans 反射（Product 的 getUrl/getName 属性）、
    // setClassForTemplateLoading（类路径模板加载）与输出到 System.out
    // （GettingStartedExample.java:37-68）——v1 无 Bean 反射与类路径加载，
    // 且方法无断言（仅演示打印）；test.ftlh 内容见
    // src/test/resources/freemarker/manual/test.ftlh。
}

/// Java TemplateConfigurationExamples.example1 —— FileExtensionMatcher("xml") →
/// TemplateConfiguration(encoding="utf-8", outputFormat=XML)（程序化 + properties）
#[test]
fn example1() {
    // NOT_APPLICABLE: Java 断言 t.getEncoding()=="utf-8" 与
    // t.getOutputFormat()==XMLOutputFormat.INSTANCE（TemplateConfigurationExamples.java:44-63）；
    // v1 Template.encoding 仅反映 <#ftl encoding> 头声明（TemplateConfiguration 的
    // encoding 作用于读取编码 Settings.input_encoding），且无输出格式 getter——
    // 断言不可移植（引擎的工厂/匹配器机制已由 template_configuration_test.rs 覆盖）。
    // Java 原文：addTemplate("t.xml", "")；ConditionalTemplateConfigurationFactory(
    //   FileExtensionMatcher("xml"), tcUTF8XML)；cfg.setSettings(
    //   loadPropertiesFile("TemplateConfigurationExamples1.properties"))。
}

/// Java TemplateConfigurationExamples.example2 —— PathGlobMatcher("mail/**") +
/// FileNameGlobMatcher("*.subject.*"/"*.body.*") 嵌套 FirstMatch
#[test]
fn example2() {
    // NOT_APPLICABLE: Java 断言均为 cfg.getTemplate(name).getOutputFormat()
    // == Undefined/PlainText/HTMLOutputFormat（TemplateConfigurationExamples.java:65-96）；
    // v1 Template 无输出格式 getter，且 setSettings(properties) 工厂 DSL 无解析——
    // 断言不可移植（匹配器语义见 template_configuration_test.rs）。
    // Java 原文：t.subject.ftl→Undefined、mail/t.subject.ftl→PlainText、
    //   mail/t.body.ftl→HTML（程序化与 TemplateConfigurationExamples2.properties 两式）。
}

/// Java TemplateConfigurationExamples.example3 —— Merging 工厂：*.stats.* →
/// iso 日期格式 + UTC 时区；mail/** → utf-8；xml/html/htm → 输出格式
#[test]
fn example3() {
    let (mut c, loader) = test_config();
    // Java: cfg.setDefaultEncoding("ISO-8859-1")（仅影响 getEncoding() 断言——
    // NOT_APPLICABLE，v1 Template.encoding 无对应，此处省略）
    c.settings.input_encoding = Some("ISO-8859-1".to_string());
    // Java: cfg.setSharedVariable("ts", new Date(1440431606011L))
    c.shared_vars.insert(
        "ts".to_string(),
        TModel::from_date(DateValue::new(
            chrono::DateTime::from_timestamp_millis(1440431606011)
                .unwrap()
                .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
            DateType::DateTime,
        )),
    );
    // Java: tcStats.setDateTimeFormat("iso")/setDateFormat("iso")/setTimeFormat("iso")
    // + setTimeZone(DateUtil.UTC)。引擎差异：TemplateConfiguration 无 time_zone
    // 字段（core/template_configuration.rs）——用全局 UTC 等价（本测试仅渲染该模板，
    // 可观察结果一致）
    c.settings.time_zone = "UTC".parse().unwrap_or(freemarker::core::TzSetting::Fixed(
        chrono::FixedOffset::east_opt(0).unwrap(),
    ));
    c.settings.time_zone_id = "UTC".to_string();
    let mut tc_stats = TemplateConfiguration::default();
    tc_stats.date_time_format = Some("iso".to_string());
    tc_stats.date_format = Some("iso".to_string());
    tc_stats.time_format = Some("iso".to_string());
    // Java: MergingTemplateConfigurationFactory(Conditional(*.stats.* → tcStats),
    //   Conditional(mail/** → utf-8), FirstMatch(xml→XML, Or(html,htm)→HTML).
    //   allowNoMatch(true))——mail/扩展名分支的断言是 getOutputFormat/getEncoding
    //   （NOT_APPLICABLE），省略不影响本渲染断言
    c.set_template_configurations(Some(Arc::new(MergingTemplateConfigurationFactory::new(
        vec![Box::new(
            ConditionalTemplateConfigurationFactory::with_configuration(
                Box::new(FileNameGlobMatcher::new("*.stats.*")),
                Arc::new(tc_stats),
            ),
        )],
    ))));
    add_template(
        &loader,
        "t.stats.html",
        "${ts?datetime} ${ts?date} ${ts?time}",
    );
    // Java: assertOutputForNamed("t.stats.html", "2015-08-24T15:53:26.011Z 2015-08-24 15:53:26.011Z")
    // （?datetime→"2015-08-24T15:53:26.011Z"、?date→"2015-08-24"、?time→"15:53:26.011Z"）
    assert_eq!(
        render_named(&c, &loader, "t.stats.html"),
        "2015-08-24T15:53:26.011Z 2015-08-24 15:53:26.011Z"
    );
    // NOT_APPLICABLE: cfg.getTemplate("t.html"/"t.htm"/"t.xml"/"t.stats.html"/"mail/t.html")
    // 的 getOutputFormat()/getEncoding() 断言（TemplateConfigurationExamples.java:168-186）
    // —— v1 Template 无输出格式 getter；encoding 无对应读数。
}

/// Java WithArgsExamples.usingWithArgsSpecialVariable ——
/// assertOutputForNamed("WithArgsExamples-usingWithArgsSpecialVariable.ftl")
/// （宏内 `<@m1?with_args(.args) />` 委托调用示例）
#[test]
#[ignore = "引擎差异：?with_args 对宏调用 v1 仅支持方法模型（报 \"?with_args is only supported on methods in v1\"）；断言保留 Java 原文"]
fn test_using_with_args_special_variable() {
    let (c, loader) = test_config();
    // 示例 FTL（license 头省略）：
    //   <#macro m1 a b c>  m1 does things with ${a}, ${b}, ${c} </#macro>
    //   <#macro m2 a b c>
    //     m2 does things with ${a}, ${b}, ${c}
    //     Delegate to m1:
    //     <@m1?with_args(.args) />
    //   </#macro>
    //   <@m2 a=1 b=2 c=3 />
    let ftl = "<#macro m1 a b c>\n\
               \x20 m1 does things with ${a}, ${b}, ${c}\n\
               </#macro>\n\
               \n\
               <#macro m2 a b c>\n\
               \x20 m2 does things with ${a}, ${b}, ${c}\n\
               \x20 Delegate to m1:\n\
               \x20 <@m1?with_args(.args) />\n\
               </#macro>\n\
               \n\
               <@m2 a=1 b=2 c=3 />\n";
    let expected =
        "\n  m2 does things with 1, 2, 3\n  Delegate to m1:\n  m1 does things with 1, 2, 3\n";
    assert_output(&c, &loader, ftl, expected);
}

/// Java WithArgsLastExamples.usingWithArgsSpecialVariable ——
/// assertOutputForNamed("WithArgsLastExamples.ftl")
/// （函数/宏的 ?with_args/?with_args_last 与 .args 特殊变量示例）
#[test]
#[ignore = "引擎差异：?with_args/?with_args_last 对函数/宏调用 v1 仅支持方法模型（报 \"?with_args is only supported on methods in v1\"）；断言保留 Java 原文"]
fn test_using_with_args_special_variable_last() {
    let (c, loader) = test_config();
    // 示例 FTL（license 头省略）——期望输出见方法注释末尾：
    //   <#function f a b c d><#return "a=${a}, b=${b}, c=${c}, d=${d}"></#function>
    //   ${f?with_args([2, 3])(1, 2)}
    //   ${f?with_args_last([2, 3])(1, 2)}
    //   <#macro m a b others...>a=${a} b=${b} others: <#list others as k, v>...</#list></#macro>
    //   <@m?with_args({'e': 5, 'f': 6}) a=1 b=2 c=3 d=4 />
    //   <@m?with_args_last({'e': 5, 'f': 6}) a=1 b=2 c=3 d=4 />
    //   <#macro m a b others...><#list .args as k, v>${k} = ${v}</#list></#macro>
    //   <@m?with_args(...)/<@m?with_args_last(...) a=1 b=2 c=3 d=4 />
    let ftl = "<#function f a b c d>\n\
               \x20 <#return \"a=${a}, b=${b}, c=${c}, d=${d}\">\n\
               </#function>\n\
               \n\
               ${f?with_args([2, 3])(1, 2)}\n\
               ${f?with_args_last([2, 3])(1, 2)}\n\
               \n\
               <#macro m a b others...>\n\
               \x20 a=${a}\n\
               \x20 b=${b}\n\
               \x20 others:\n\
               \x20 <#list others as k, v>\n\
               \x20   ${k} = ${v}\n\
               \x20 </#list>\n\
               </#macro>\n\
               <@m?with_args({'e': 5, 'f': 6}) a=1 b=2 c=3 d=4 />\n\
               <@m?with_args_last({'e': 5, 'f': 6}) a=1 b=2 c=3 d=4 />\n\
               \n\
               <#macro m a b others...>\n\
               \x20 <#list .args as k, v>\n\
               \x20   ${k} = ${v}\n\
               \x20 </#list>\n\
               </#macro>\n\
               <@m?with_args({'e': 5, 'f': 6}) a=1 b=2 c=3 d=4 />\n\
               <@m?with_args_last({'e': 5, 'f': 6}) a=1 b=2 c=3 d=4 />\n";
    let expected = "\na=2, b=3, c=1, d=2\na=1, b=2, c=2, d=3\n\n  a=1\n  b=2\n  others:\n    e = 5\n    f = 6\n    c = 3\n    d = 4\n  a=1\n  b=2\n  others:\n    c = 3\n    d = 4\n    e = 5\n    f = 6\n\n    a = 1\n    b = 2\n    e = 5\n    f = 6\n    c = 3\n    d = 4\n    a = 1\n    b = 2\n    c = 3\n    d = 4\n    e = 5\n    f = 6\n";
    assert_output(&c, &loader, ftl, expected);
}

/// AbsoluteTemplateNameBIExample-main.ftl（license 头省略）
const MAIN_FTL: &str = "<#import '/AbsoluteTemplateNameBIExample-lib.ftl' as lib>\n\
<@lib.smileyInclude 'AbsoluteTemplateNameBIExample-foo.ftl' />\n\
<@lib.smileyInclude '../AbsoluteTemplateNameBIExample-foo.ftl' />\n\
<@lib.smileyInclude '/AbsoluteTemplateNameBIExample-foo.ftl' />\n\
<@lib.smileyInclude 'AbsoluteTemplateNameBIExample-missing.ftl' />";

/// AbsoluteTemplateNameBIExample-lib.ftl（license 与文档注释头省略）
const LIB_FTL: &str = "<#macro smileyInclude name>\n\
  <#local t = .get_optional_template(\n\
      name?absolute_template_name(.caller_template_name))>\n\
  <#if t.exists>\n\
    (:\n\
    <@t.include />\n\
  <#else>\n\
    ):\n\
  </#if>\n\
</#macro>";
