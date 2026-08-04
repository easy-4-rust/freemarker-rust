//! JSON 输出格式 —— 对应 Java `freemarker.core.JSONOutputFormat`
//! （escape：JSON 字符串转义；name = "JSON"；v1 不转义——P4 对齐项）

/// JSON 输出格式（对应 JSONOutputFormat.java；OutputFormatKind::Json 的承载）
#[allow(dead_code)]
pub(crate) struct JsonOutputFormat;

#[allow(dead_code)]
impl JsonOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "JSON"
    }

    /// Java `escape(String)`（v1：原样返回——JSON 转义属 P4）
    pub(crate) fn escape(s: &str) -> String {
        s.to_string()
    }
}
