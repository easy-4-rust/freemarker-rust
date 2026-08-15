//! 扩展哈希类型异常 —— 对应 Java `freemarker.core.NonExtendedHashException`
//! （UnexpectedTypeException 子类；expected 描述 = "extended hash"）

use crate::error::TemplateError;

/// Java `NonExtendedHashException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("extended hash", actual)
}
