//! legacy C 格式 —— 对应 Java `freemarker.core.LegacyCFormat`
//! （`CFormatKind::Legacy` 变体承载；与默认 JavaScriptOrJSON 共享字符串转义，
//!  数字符号不同：Infinity/NaN 使用 Java 风格）

/// Java 类锚点：`LegacyCFormat`（Rust 侧由 `CFormatKind::Legacy` 承载）
#[allow(dead_code)]
pub(crate) struct LegacyCFormat;

impl LegacyCFormat {
    /// Java `LegacyCFormat.NAME`
    #[allow(dead_code)]
    pub(crate) const NAME: &'static str = "legacy";
}
