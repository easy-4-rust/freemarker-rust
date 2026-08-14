//! JSON C 格式 —— 对应 Java `freemarker.core.JSONCFormat`
//! （`CFormatKind::Json` 变体承载；字符串用 jsStringEnc(JS_OR_JSON, QUOTATION_MARK)）

/// Java 类锚点：`JSONCFormat`（Rust 侧由 `CFormatKind::Json` 承载）
#[allow(dead_code)]
pub(crate) struct JsonCFormat;

impl JsonCFormat {
    /// Java `JSONCFormat.NAME`
    #[allow(dead_code)]
    pub(crate) const NAME: &'static str = "JSON";
}
