//! 无界范围列表异常 —— 对应 Java `freemarker.core.NonListableRightUnboundedRangeModelException`
//! （UnexpectedTypeException 子类；expected 描述 = "listable range"）

use crate::error::TemplateError;

/// Java `NonListableRightUnboundedRangeModelException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("listable range", actual)
}
