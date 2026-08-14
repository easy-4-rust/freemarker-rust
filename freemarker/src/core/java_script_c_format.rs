//! JavaScript C 格式 —— 对应 Java `freemarker.core.JavaScriptCFormat`
//! （`CFormatKind::JavaScript` 变体承载；字符串用 jsStringEnc(JS, QUOTATION_MARK)）

/// Java 类锚点：`JavaScriptCFormat`（Rust 侧由 `CFormatKind::JavaScript` 承载）
#[allow(dead_code)]
pub(crate) struct JavaScriptCFormat;

impl JavaScriptCFormat {
    /// Java `JavaScriptCFormat.NAME`
    #[allow(dead_code)]
    pub(crate) const NAME: &'static str = "JavaScript";
}
