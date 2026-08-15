//! 核心 API 工具 —— 对应 Java `freemarker.core._CoreAPI`
//! （ERROR_MESSAGE_HR 常量、所有 Setting 名集合、API 名辅助方法；
//!  Rust 无 Setting 名反射机制 → 锚点）

/// Java 类锚点：`_CoreAPI` 的 Rust 语义分散在 Settings 与各模块常量中
#[allow(dead_code)]
pub(crate) struct _CoreAPI;

impl _CoreAPI {
    /// 错误消息分隔线（Java ERROR_MESSAGE_HR = "----"）
    #[allow(dead_code)]
    pub(crate) const ERROR_MESSAGE_HR: &'static str = "----";
}
