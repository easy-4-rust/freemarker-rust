//! 不可解析值异常 —— 对应 Java `freemarker.core.UnparsableValueException`
//! （TemplateValueFormatException 子类；值解析失败时抛出）

use crate::error::TemplateError;

/// Java `UnparsableValueException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
