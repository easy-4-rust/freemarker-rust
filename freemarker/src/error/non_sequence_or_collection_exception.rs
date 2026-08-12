//! 序列/集合类型异常 —— 对应 Java `freemarker.core.NonSequenceOrCollectionException`
//! （UnexpectedTypeException 子类；expected 描述 = "sequence or collection"）

use crate::error::TemplateError;

/// Java `NonSequenceOrCollectionException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("sequence or collection", actual)
}
