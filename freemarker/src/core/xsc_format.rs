//! XS C 格式 —— 对应 Java `freemarker.core.XSCFormat`
//! （`CFormatKind::Xs` 变体承载；字符串不转义——假定已有 XML 自动转义）

/// Java 类锚点：`XSCFormat`（Rust 侧由 `CFormatKind::Xs` 承载）
#[allow(dead_code)]
pub(crate) struct XsCFormat;

impl XsCFormat {
    /// Java `XSCFormat.NAME`
    #[allow(dead_code)]
    pub(crate) const NAME: &'static str = "XS";
}
