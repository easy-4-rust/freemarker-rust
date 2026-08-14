//! JavaScript 或 JSON C 格式 —— 对应 Java `freemarker.core.JavaScriptOrJSONCFormat`
//! （`CFormatKind::JavaScriptOrJson` 变体承载；ICI >= 2.3.32 默认格式）

/// Java 类锚点：`JavaScriptOrJSONCFormat`（Rust 侧由 `CFormatKind::JavaScriptOrJson` 承载）
#[allow(dead_code)]
pub(crate) struct JavaScriptOrJsonCFormat;

impl JavaScriptOrJsonCFormat {
    /// Java `JavaScriptOrJSONCFormat.NAME`
    #[allow(dead_code)]
    pub(crate) const NAME: &'static str = "JavaScript or JSON";
}
