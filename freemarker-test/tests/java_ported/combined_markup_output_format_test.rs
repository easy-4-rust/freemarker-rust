//! Java `freemarker.core.CombinedMarkupOutputFormatTest` 的 Rust 1:1 实现
//! （对应 Java: CombinedMarkupOutputFormatTest —— `CombinedMarkupOutputFormat`
//!   组合标记输出格式类的单测：HTML{RTF}/XML{XML} 的名称、组合转义输出、
//!   fromPlainTextByEscaping/fromMarkup、concat、getMarkupString、mimeType）。
//!
//! 引擎实现：core/combined_markup_output_format.rs（components: Vec<OutputFormatKind>，
//! components[0] = 最外层；escape 最内层先转义逐层向外——CombinedMarkupOutputFormat.java
//! :78-80；name = "A{B}" 递归——:51-60；mimeType = 外层——:63-65）。
//! 转义规则与 freemarker-2.3.34.jar 实测逐字对齐（RTFEnc 仅转义 `\` `{` `}`，
//! StringUtil.java:276-314；HTMLEnc `& < > "`、XMLEnc `& < > " '`）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::core::{CombinedMarkupOutputFormat, CombinedMarkupOutputModel, OutputFormatKind};

/// Java 测试常量：`new CombinedMarkupOutputFormat(HTMLOutputFormat.INSTANCE,
/// RTFOutputFormat.INSTANCE)`（CombinedMarkupOutputFormatTest.java:32-35）
fn html_rtf() -> CombinedMarkupOutputFormat {
    CombinedMarkupOutputFormat::new(vec![OutputFormatKind::Html, OutputFormatKind::Rtf])
}

fn xml_xml() -> CombinedMarkupOutputFormat {
    CombinedMarkupOutputFormat::new(vec![OutputFormatKind::Xml, OutputFormatKind::Xml])
}

/// Java assertMO(pc, mc, mo)：getPlainTextContent/getMarkupContent 逐项断言
fn assert_mo(pc: Option<&str>, mc: Option<&str>, mo: &CombinedMarkupOutputModel) {
    assert_eq!(mo.plain_text.as_deref(), pc);
    assert_eq!(mo.markup.as_deref(), mc);
}

/// Java testName（Java 原文）：
///   assertEquals("HTML{RTF}", HTML_RTF.getName());
///   assertEquals("XML{XML}", XML_XML.getName());
#[test]
fn test_name() {
    assert_eq!(html_rtf().name(), "HTML{RTF}");
    assert_eq!(xml_xml().name(), "XML{XML}");
}

/// Java testOutputMO（Java 原文）：HTML_RTF 组合输出——fromMarkup 原样、
/// fromPlainTextByEscaping 逐层转义（RTF 先 `\`/`{`/`}`、HTML 再 `& < > "`）
#[test]
fn test_output_mo() {
    let f = html_rtf();
    let mut out = String::new();
    out.push_str(&f.output_model(&f.from_markup("<pre>\\par Test ".to_string())));
    out.push_str(&f.output_model(&f.from_plain_text_by_escaping("foo { bar } \\ ".to_string())));
    out.push_str(&f.output_model(&f.from_plain_text_by_escaping("& baaz ".to_string())));
    out.push_str(&f.output_model(&f.from_plain_text_by_escaping("\\par & qwe".to_string())));
    out.push_str(&f.output_model(&f.from_markup("\\par{0} End</pre>".to_string())));
    assert_eq!(
        out,
        concat!(
            "<pre>\\par Test ",
            "foo \\{ bar \\} \\\\ ",
            "&amp; baaz ",
            "\\\\par &amp; qwe",
            "\\par{0} End</pre>"
        )
    );
}

/// Java testOutputMO2（Java 原文）：XML{XML}——内层 XML 转义结果再经外层
/// XML 转义（`&` → `&amp;` 二次，故 "a & b < c" → "a &amp;amp; b &amp;lt; c"）
#[test]
fn test_output_mo2() {
    let f = xml_xml();
    let mut out = String::new();
    out.push_str(&f.output_model(&f.from_markup("<pre>&lt;p&gt; Test ".to_string())));
    out.push_str(&f.output_model(&f.from_plain_text_by_escaping("a & b < c".to_string())));
    out.push_str(&f.output_model(&f.from_markup(" End</pre>".to_string())));
    assert_eq!(
        out,
        "<pre>&lt;p&gt; Test a &amp;amp; b &amp;lt; c End</pre>"
    );
}

/// Java testOutputMO3（Java 原文）：
///   MarkupOutputFormat outputFormat = new CombinedMarkupOutputFormat(
///           RTFOutputFormat.INSTANCE,
///           new CombinedMarkupOutputFormat(RTFOutputFormat.INSTANCE, RTFOutputFormat.INSTANCE));
/// RTF 三层组合：escapePlainText("b{}") 实测（jar 2.3.34）=
/// `b` + 7×`\` + `{` + 7×`\` + `}`（每层把 `\` `{` `}` 各前置 `\`）
#[test]
fn test_output_mo3() {
    let f = CombinedMarkupOutputFormat::new(vec![
        OutputFormatKind::Rtf,
        OutputFormatKind::Rtf,
        OutputFormatKind::Rtf,
    ]);
    let mut out = String::new();
    out.push_str(&f.output_model(&f.from_plain_text_by_escaping("b{}".to_string())));
    out.push_str(&f.output_model(&f.from_markup("a{}".to_string())));
    assert_eq!(out, "b\\\\\\\\\\\\\\{\\\\\\\\\\\\\\}a{}");
}

/// Java testOutputString（Java 原文）：HTML{RTF}.output(String)——
/// `outer.output(inner.escapePlainText(textToEsc), out)`（CombinedMarkupOutputFormat.java:68-70）
#[test]
fn test_output_string() {
    let f = html_rtf();
    // Java 三段输出拼接：output("a") + output("{") + output("<b>}c")
    assert_eq!(f.output("a"), "a");
    assert_eq!(f.output("{"), "\\{");
    assert_eq!(f.output("<b>}c"), "&lt;b&gt;\\}c");
    let mut out = String::new();
    out.push_str(&f.output("a"));
    out.push_str(&f.output("{"));
    out.push_str(&f.output("<b>}c"));
    assert_eq!(out, "a\\{&lt;b&gt;\\}c");
}

/// Java testOutputString2（Java 原文）：XML{XML}.output(String) 双重 XML 转义
#[test]
fn test_output_string2() {
    let f = xml_xml();
    assert_eq!(f.output("a"), "a");
    assert_eq!(f.output("&"), "&amp;amp;");
    assert_eq!(f.output("<b>"), "&amp;lt;b&amp;gt;");
}

/// Java testFromPlainTextByEscaping（Java 原文）：模型只存 plainTextContent，
/// markupContent 为 null（"Not the MO's duty to calculate it!"——CommonMarkupOutputFormat :34-37）
#[test]
fn test_from_plain_text_by_escaping() {
    let plain_text = "a\\b&c";
    let mo = html_rtf().from_plain_text_by_escaping(plain_text.to_string());
    assert_eq!(mo.plain_text.as_deref(), Some(plain_text));
    assert_eq!(mo.markup, None);
}

/// Java testFromMarkup（Java 原文）：模型只存 markupContent，plainTextContent 为 null
#[test]
fn test_from_markup() {
    let markup = "a \\par <b>";
    let mo = html_rtf().from_markup(markup.to_string());
    assert_eq!(mo.markup.as_deref(), Some(markup));
    assert_eq!(mo.plain_text, None);
}

/// Java testGetMarkup（Java 原文）：getMarkupString——markup 原样；纯文本经
/// escapePlainText（"abc" 无可转义字符 → 原样）
#[test]
fn test_get_markup() {
    let f = html_rtf();
    {
        let markup = "a \\par <b>";
        let mo = f.from_markup(markup.to_string());
        assert_eq!(f.get_markup_string(&mo), markup);
    }
    {
        let safe = "abc";
        let mo = f.from_plain_text_by_escaping(safe.to_string());
        assert_eq!(f.get_markup_string(&mo), safe);
    }
}

/// Java testConcat（Java 原文）：CommonMarkupOutputFormat.concat :60-75——
/// 双 plain 拼接 / 双 markup 拼接 / 混合时纯文本侧经 getMarkupString 转义后拼接
#[test]
fn test_concat() {
    let f = html_rtf();
    let mo = |pc: Option<&str>, mc: Option<&str>| CombinedMarkupOutputModel {
        plain_text: pc.map(String::from),
        markup: mc.map(String::from),
    };
    assert_mo(
        Some("ab"),
        None,
        &f.concat(&mo(Some("a"), None), &mo(Some("b"), None)),
    );
    assert_mo(
        None,
        Some("ab"),
        &f.concat(&mo(None, Some("a")), &mo(None, Some("b"))),
    );
    assert_mo(
        None,
        Some("{<a>}\\{&lt;b&gt;\\}"),
        &f.concat(&mo(None, Some("{<a>}")), &mo(Some("{<b>}"), None)),
    );
    assert_mo(
        None,
        Some("\\{&lt;a&gt;\\}{<b>}"),
        &f.concat(&mo(Some("{<a>}"), None), &mo(None, Some("{<b>}"))),
    );
}

/// Java testEscaplePlainText（Java 原文）：escapePlainText = 外层 escape(内层
/// escape(...))；HTML{RTF} 与 XML{XML} 的逐层转义（含 `'` → `&apos;` 的二次转义）
#[test]
fn test_escape_plain_text() {
    let hr = html_rtf();
    assert_eq!(hr.escape_plain_text(""), "");
    assert_eq!(hr.escape_plain_text("a"), "a");
    assert_eq!(hr.escape_plain_text("{a\\b&}"), "\\{a\\\\b&amp;\\}");
    assert_eq!(hr.escape_plain_text("a\\b&"), "a\\\\b&amp;");
    assert_eq!(hr.escape_plain_text("{}&"), "\\{\\}&amp;");

    let xx = xml_xml();
    assert_eq!(xx.escape_plain_text("a"), "a");
    // 双层 XML：' 先 → &apos;，& 再 → &amp;
    assert_eq!(xx.escape_plain_text("a'b"), "a&amp;apos;b");
}

/// Java testGetMimeType（Java 原文）：组合格式取外层 MIME
#[test]
fn test_get_mime_type() {
    assert_eq!(html_rtf().mime_type(), "text/html");
    assert_eq!(xml_xml().mime_type(), "application/xml");
}
