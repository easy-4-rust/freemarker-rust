//! XHTML 输出格式 —— 对应 Java `freemarker.core.XHTMLOutputFormat`
//! （escape：`& < > " \'`；name = "XHTML"）

use crate::core::html_output_format::HtmlOutputFormat;

/// XHTML 输出格式（对应 XHTMLOutputFormat.java；OutputFormatKind::XHtml 的承载）
#[allow(dead_code)]
pub(crate) struct XHtmlOutputFormat;

#[allow(dead_code)]
impl XHtmlOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "XHTML"
    }

    /// Java `escape(String)` → XHTMLEncUtil（`& < > " \'` 实体集；
    /// v1 委托 html_escape——`\'` 实体差异属 P4 对齐项）
    pub(crate) fn escape(s: &str) -> String {
        HtmlOutputFormat::escape(s)
    }
}
