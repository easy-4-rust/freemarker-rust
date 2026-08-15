//! 别名目标值格式异常 —— 对应 Java `freemarker.core.AliasTargetTemplateValueFormatException`
//! （别名格式引用的目标格式创建失败时抛出；Rust 侧由 `TemplateError` 承载）

use crate::error::TemplateError;

/// Java `AliasTargetTemplateValueFormatException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(message)
}
