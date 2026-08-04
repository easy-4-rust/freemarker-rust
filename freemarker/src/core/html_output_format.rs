//! HTML 输出格式 —— 对应 Java `freemarker.core.HTMLOutputFormat`
//! （escape：`& < > "`（不含 `\'`，不同于 XHTMLEnc）；name = "HTML"）

use crate::template::utility::html_escape;

/// HTML 输出格式（对应 HTMLOutputFormat.java；OutputFormatKind::Html 的承载）
#[allow(dead_code)]
pub(crate) struct HtmlOutputFormat;

#[allow(dead_code)]
impl HtmlOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "HTML"
    }

    /// Java `escape(String)` → HTMLEncUtil（`& < > "` 实体集）
    pub(crate) fn escape(s: &str) -> String {
        html_escape(s)
    }
}
