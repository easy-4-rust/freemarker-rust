//! 非序列类型异常 —— 对应 Java `freemarker.core.NonSequenceException`
//! （UnexpectedTypeException 子类；expected 描述 = "sequence"）

use crate::error::TemplateError;

/// Java `NonSequenceException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("sequence", actual)
}
