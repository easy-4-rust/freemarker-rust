//! Java `freemarker.core.CombinedMarkupOutputFormatTest` 的 Rust 1:1 实现
//! （对应 Java: CombinedMarkupOutputFormatTest —— `CombinedMarkupOutputFormat`
//!   组合标记输出格式类的单测：HTML{RTF}/XML{XML} 的名称、组合转义输出、
//!   fromPlainTextByEscaping/fromMarkup、concat、getMarkupString、mimeType）。
//!
//! ENGINE_GAP: 引擎没有 `CombinedMarkupOutputFormat` 类（output_format.rs 只有
//!   8 个单格式 kind，无"组合格式"；无 TemplateCombinedMarkupOutputModel——
//!   组合格式的"外层格式转义 + 内层格式标记"的 concat/输出语义未移植）。
//!   本测试类全部 10 个方法都是对该类（及其输出模型）的直接单测，无法 1:1
//!   迁移，全部登记 NOT_APPLICABLE（Java 原文保留为注释）。
//!
//! NOT_APPLICABLE: 全部 10 个方法 —— 引擎无 CombinedMarkupOutputFormat /
//!   TemplateCombinedMarkupOutputModel（组合格式的嵌套转义语义：如 HTML{RTF}
//!   的 `fromPlainTextByEscaping("foo { bar } \\ ")` → `foo \{ bar \} \\ ` 需
//!   RTF 转义外层套 HTML 转义内层的两段式输出，v1 无此机制）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（CombinedMarkupOutputFormatTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java 测试常量（Java 原文）：
//   private static final CombinedMarkupOutputFormat HTML_RTF = new CombinedMarkupOutputFormat(
//           HTMLOutputFormat.INSTANCE, RTFOutputFormat.INSTANCE);
//   private static final CombinedMarkupOutputFormat XML_XML = new CombinedMarkupOutputFormat(
//           XMLOutputFormat.INSTANCE, XMLOutputFormat.INSTANCE);

// Java testName（Java 原文）：
//   assertEquals("HTML{RTF}", HTML_RTF.getName());
//   assertEquals("XML{XML}", XML_XML.getName());

// Java testOutputMO（Java 原文）：
//   StringWriter out = new StringWriter();
//   HTML_RTF.output(HTML_RTF.fromMarkup("<pre>\\par Test "), out);
//   HTML_RTF.output(HTML_RTF.fromPlainTextByEscaping("foo { bar } \\ "), out);
//   HTML_RTF.output(HTML_RTF.fromPlainTextByEscaping("& baaz "), out);
//   HTML_RTF.output(HTML_RTF.fromPlainTextByEscaping("\\par & qwe"), out);
//   HTML_RTF.output(HTML_RTF.fromMarkup("\\par{0} End</pre>"), out);
//   assertEquals(
//           "<pre>\\par Test "
//           + "foo \\{ bar \\} \\\\ "
//           + "&amp; baaz "
//           + "\\\\par &amp; qwe"
//           + "\\par{0} End</pre>",
//           out.toString());

// Java testOutputMO2（Java 原文）：
//   StringWriter out = new StringWriter();
//   XML_XML.output(XML_XML.fromMarkup("<pre>&lt;p&gt; Test "), out);
//   XML_XML.output(XML_XML.fromPlainTextByEscaping("a & b < c"), out);
//   XML_XML.output(XML_XML.fromMarkup(" End</pre>"), out);
//   assertEquals(
//           "<pre>&lt;p&gt; Test "
//           + "a &amp;amp; b &amp;lt; c"
//           + " End</pre>",
//           out.toString());

// Java testOutputMO3（Java 原文）：
//   MarkupOutputFormat outputFormat = new CombinedMarkupOutputFormat(
//           RTFOutputFormat.INSTANCE,
//           new CombinedMarkupOutputFormat(RTFOutputFormat.INSTANCE, RTFOutputFormat.INSTANCE));
//   StringWriter out = new StringWriter();
//   outputFormat.output(outputFormat.fromPlainTextByEscaping("b{}"), out);
//   outputFormat.output(outputFormat.fromMarkup("a{}"), out);
//   assertEquals(
//           "b\\\\\\\\\\\\\\{\\\\\\\\\\\\\\}"
//           + "a{}",
//           out.toString());

// Java testOutputString（Java 原文）：
//   StringWriter out = new StringWriter();
//   HTML_RTF.output("a", out);
//   HTML_RTF.output("{", out);
//   HTML_RTF.output("<b>}c", out);
//   assertEquals("a\\{&lt;b&gt;\\}c", out.toString());

// Java testOutputString2（Java 原文）：
//   StringWriter out = new StringWriter();
//   XML_XML.output("a", out);
//   XML_XML.output("&", out);
//   XML_XML.output("<b>", out);
//   assertEquals("a&amp;amp;&amp;lt;b&amp;gt;", out.toString());

// Java testFromPlainTextByEscaping（Java 原文）：
//   String plainText = "a\\b&c";
//   TemplateCombinedMarkupOutputModel mo = HTML_RTF.fromPlainTextByEscaping(plainText);
//   assertSame(plainText, mo.getPlainTextContent());
//   assertNull(mo.getMarkupContent()); // Not the MO's duty to calculate it!

// Java testFromMarkup（Java 原文）：
//   String markup = "a \\par <b>";
//   TemplateCombinedMarkupOutputModel mo = HTML_RTF.fromMarkup(markup);
//   assertSame(markup, mo.getMarkupContent());
//   assertNull(mo.getPlainTextContent()); // Not the MO's duty to calculate it!

// Java testGetMarkup（Java 原文）：
//   {
//       String markup = "a \\par <b>";
//       TemplateCombinedMarkupOutputModel mo = HTML_RTF.fromMarkup(markup);
//       assertSame(markup, HTML_RTF.getMarkupString(mo));
//   }
//   {
//       String safe = "abc";
//       TemplateCombinedMarkupOutputModel mo = HTML_RTF.fromPlainTextByEscaping(safe);
//       assertSame(safe, HTML_RTF.getMarkupString(mo));
//   }

// Java testConcat（Java 原文）：
//   assertMO(
//           "ab", null,
//           HTML_RTF.concat(
//                   new TemplateCombinedMarkupOutputModel("a", null, HTML_RTF),
//                   new TemplateCombinedMarkupOutputModel("b", null, HTML_RTF)));
//   assertMO(
//           null, "ab",
//           HTML_RTF.concat(
//                   new TemplateCombinedMarkupOutputModel(null, "a", HTML_RTF),
//                   new TemplateCombinedMarkupOutputModel(null, "b", HTML_RTF)));
//   assertMO(
//           null, "{<a>}\\{&lt;b&gt;\\}",
//           HTML_RTF.concat(
//                   new TemplateCombinedMarkupOutputModel(null, "{<a>}", HTML_RTF),
//                   new TemplateCombinedMarkupOutputModel("{<b>}", null, HTML_RTF)));
//   assertMO(
//           null, "\\{&lt;a&gt;\\}{<b>}",
//           HTML_RTF.concat(
//                   new TemplateCombinedMarkupOutputModel("{<a>}", null, HTML_RTF),
//                   new TemplateCombinedMarkupOutputModel(null, "{<b>}", HTML_RTF)));
//   其中 assertMO(pc, mc, mo)：
//       assertEquals(pc, mo.getPlainTextContent());
//       assertEquals(mc, mo.getMarkupContent());

// Java testEscaplePlainText（Java 原文）：
//   assertEquals("", HTML_RTF.escapePlainText(""));
//   assertEquals("a", HTML_RTF.escapePlainText("a"));
//   assertEquals("\\{a\\\\b&amp;\\}", HTML_RTF.escapePlainText("{a\\b&}"));
//   assertEquals("a\\\\b&amp;", HTML_RTF.escapePlainText("a\\b&"));
//   assertEquals("\\{\\}&amp;", HTML_RTF.escapePlainText("{}&"));
//   assertEquals("a", XML_XML.escapePlainText("a"));
//   assertEquals("a&amp;apos;b", XML_XML.escapePlainText("a'b"));

// Java testGetMimeType（Java 原文）：
//   assertEquals("text/html", HTML_RTF.getMimeType());
//   assertEquals("application/xml", XML_XML.getMimeType());
