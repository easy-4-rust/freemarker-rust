//! 无效格式参数异常 —— 对应 Java `freemarker.core.InvalidFormatParametersException`
//! （格式字符串的参数部分格式错误；Rust 侧由 `TemplateError` 承载）

use crate::error::TemplateError;

/// Java `InvalidFormatParametersException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
