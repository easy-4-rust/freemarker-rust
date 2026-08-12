//! 纯文本输出格式 —— 对应 Java `freemarker.core.PlainTextOutputFormat`
//! （无转义；name = "plainText"）

/// 纯文本输出格式（对应 PlainTextOutputFormat.java；OutputFormatKind::PlainText 的承载）
#[allow(dead_code)]
pub(crate) struct PlainTextOutputFormat;

#[allow(dead_code)]
impl PlainTextOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "plainText"
    }

    /// Java `escape(String)`：恒等
    pub(crate) fn escape(s: &str) -> String {
        s.to_string()
    }
}
