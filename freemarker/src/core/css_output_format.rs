//! CSS 输出格式 —— 对应 Java `freemarker.core.CSSOutputFormat`
//! （escape：`\\` 与注释标记转义；name = "CSS"；v1 不转义——P4 对齐项）

/// CSS 输出格式（对应 CSSOutputFormat.java；OutputFormatKind::Css 的承载）
#[allow(dead_code)]
pub(crate) struct CssOutputFormat;

#[allow(dead_code)]
impl CssOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "CSS"
    }

    /// Java `escape(String)`（v1：原样返回——CSS 转义属 P4）
    pub(crate) fn escape(s: &str) -> String {
        s.to_string()
    }
}
