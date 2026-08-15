//! 模板值格式异常 —— 对应 Java `freemarker.core.TemplateValueFormatException`
//! （所有格式异常的公共基类；Rust 侧由 `TemplateError` 承载）

use crate::error::TemplateError;

/// Java `TemplateValueFormatException` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
