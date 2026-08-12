//! 数值类型异常 —— 对应 Java `freemarker.core.NonNumericalException`
//! （UnexpectedTypeException 子类；expected 描述 = "number"）

use crate::error::TemplateError;

/// Java `NonNumericalException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("number", actual)
}
