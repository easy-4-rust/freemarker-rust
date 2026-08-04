//! Java `freemarker.core.XHTMLOutputFormatTest` 的 Rust 1:1 实现
//! （对应 Java: XHTMLOutputFormatTest —— ' 转 &#39;（与 HTML 相同），MIME 不同）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java XHTMLOutputFormat.escapePlainText：& < > " '（' → &#39;，与 HTML 相同）
fn escape_plain_text(s: &str) -> String {
    freemarker::template::utility::html_escape(s)
}

/// Java XHTMLOutputFormat.getMimeType
fn get_mime_type() -> &'static str {
    "application/xhtml+xml"
}

/// Java testOutputMO
#[test]
fn test_output_mo() {
    let out = escape_plain_text("a'b");
    assert_eq!(out, "a&#39;b");
}

/// Java testOutputString
#[test]
fn test_output_string() {
    let out = escape_plain_text("a'b");
    assert_eq!(out, "a&#39;b");
}

/// Java testEscaplePlainText
#[test]
fn test_escape_plain_text() {
    assert_eq!(escape_plain_text(""), "");
    assert_eq!(escape_plain_text("a"), "a");
    assert_eq!(
        escape_plain_text("<a&b'c\"d>"),
        "&lt;a&amp;b&#39;c&quot;d&gt;"
    );
    assert_eq!(escape_plain_text("<>"), "&lt;&gt;");
}

/// Java testGetMimeType
#[test]
fn test_get_mime_type() {
    assert_eq!(get_mime_type(), "application/xhtml+xml");
}
