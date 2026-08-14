//! 不可格式化值异常 —— 对应 Java `freemarker.core.UnformattableValueException`
//! （TemplateValueFormatException 子类；值无法格式化为字符串时抛出）

use crate::error::TemplateError;

/// Java `UnformattableValueException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
