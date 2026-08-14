//! 无效格式字符串异常 —— 对应 Java `freemarker.core.InvalidFormatStringException`
//! （格式字符串格式错误；Rust 侧由 `TemplateError` 承载）

use crate::error::TemplateError;

/// Java `InvalidFormatStringException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
