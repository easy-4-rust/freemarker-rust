//! 解析不支持异常 —— 对应 Java `freemarker.core.ParsingNotSupportedException`
//! （TemplateValueFormatException 子类；值格式不支持解析时抛出）

use crate::error::TemplateError;

/// Java `ParsingNotSupportedException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
