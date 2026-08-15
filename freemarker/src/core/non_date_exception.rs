//! 非日期类型异常 —— 对应 Java `freemarker.core.NonDateException`
//! （UnexpectedTypeException 子类；expected 描述 = "date/time"）

use crate::error::TemplateError;

/// Java `NonDateException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("date/time", actual)
}
