//! JavaScript 输出格式 —— 对应 Java `freemarker.core.JavaScriptOutputFormat`
//! （escape：`\" \' \\ \n \r \t` 等转义；name = "JavaScript"；v1 不转义——P4 对齐项）

/// JavaScript 输出格式（对应 JavaScriptOutputFormat.java；OutputFormatKind::JavaScript 的承载）
#[allow(dead_code)]
pub(crate) struct JavaScriptOutputFormat;

#[allow(dead_code)]
impl JavaScriptOutputFormat {
    /// Java `getOutputFormatName()`
    pub(crate) fn name() -> &'static str {
        "JavaScript"
    }

    /// Java `escape(String)`（v1：原样返回——JS 转义属 P4）
    pub(crate) fn escape(s: &str) -> String {
        s.to_string()
    }
}
