//! 未定义自定义格式异常 —— 对应 Java `freemarker.core.UndefinedCustomFormatException`
//! （引用了未定义的自定义格式名；Rust 侧由 `TemplateError` 承载）
//!
//! 报错路径已在 `builtins/iso_date_format.rs`/`java_date_format.rs` 实现：
//! "No custom format was defined with name \"{name}\""

use crate::error::TemplateError;

/// Java `UndefinedCustomFormatException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
