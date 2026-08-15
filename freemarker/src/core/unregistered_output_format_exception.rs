//! 未注册输出格式异常 —— 对应 Java `freemarker.core.UnregisteredOutputFormatException`
//! （引用了未注册的输出格式名；Rust 侧由 `TemplateError` 承载）

use crate::error::TemplateError;

/// Java `UnregisteredOutputFormatException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
