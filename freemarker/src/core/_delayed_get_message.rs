//! 惰性获取异常消息 —— 对应 Java `freemarker.core._DelayedGetMessage`
//! （_DelayedConversionToString 子类；doConversion = throwable.getMessage()；
//!  空消息 → "[No exception message]"；Rust 由 TemplateError::to_string() 覆盖）

/// Java 类锚点：`_DelayedGetMessage`（Rust 由 TemplateError::to_string() 覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedGetMessage;
