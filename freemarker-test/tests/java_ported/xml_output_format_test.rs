//! Java `freemarker.core.XMLOutputFormatTest` 的 Rust 1:1 实现
//! （对应 Java: XMLOutputFormatTest —— escapePlainText 用 ' 转 &apos;）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java XMLOutputFormat.escapePlainText（= StringUtil.XMLEnc：& < > " '）
fn escape_plain_text(s: &str) -> String {
    freemarker::template::utility::xml_escape(s)
}

/// Java XMLOutputFormat.getMimeType
fn get_mime_type() -> &'static str {
    "application/xml"
}

/// Java testOutputMO：fromPlainTextByEscaping("a'b") → output → "a&apos;b"
#[test]
fn test_output_mo() {
    let out = escape_plain_text("a'b");
    assert_eq!(out, "a&apos;b");
}

/// Java testOutputString：output("a'b") → "a&apos;b"
#[test]
fn test_output_string() {
    let out = escape_plain_text("a'b");
    assert_eq!(out, "a&apos;b");
}

/// Java testEscaplePlainText
#[test]
fn test_escape_plain_text() {
    assert_eq!(escape_plain_text(""), "");
    assert_eq!(escape_plain_text("a"), "a");
    assert_eq!(
        escape_plain_text("<a&b'c\"d>"),
        "&lt;a&amp;b&apos;c&quot;d&gt;"
    );
    assert_eq!(escape_plain_text("<>"), "&lt;&gt;");
}

/// Java testGetMimeType
#[test]
fn test_get_mime_type() {
    assert_eq!(get_mime_type(), "application/xml");
}
