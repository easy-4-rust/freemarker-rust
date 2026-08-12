//! 字符串/模板输出类型异常 —— 对应 Java `freemarker.core.NonStringOrTemplateOutputException`
//! （UnexpectedTypeException 子类；expected 描述 = "string or template output"）

use crate::error::TemplateError;

/// Java `NonStringOrTemplateOutputException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("string or template output", actual)
}
