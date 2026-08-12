//! RTF 输出格式 —— 对应 Java `freemarker.core.RTFOutputFormat`
//! （escape：`\\`、`{`、`}`、非 ASCII → \\uXXXX；name = "RTF"；v1 不转义——P4 对齐项）

/// RTF 输出格式（对应 RTFOutputFormat.java；OutputFormatKind::Rtf 的承载）
#[allow(dead_code)]
pub(crate) struct RtfOutputFormat;

#[allow(dead_code)]
impl RtfOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "RTF"
    }

    /// Java `escape(String)`（v1：原样返回——RTF 转义属 P4）
    pub(crate) fn escape(s: &str) -> String {
        s.to_string()
    }
}
