//! 非命名空间类型异常 —— 对应 Java `freemarker.core.NonNamespaceException`
//! （UnexpectedTypeException 子类；expected 描述 = "namespace"）

use crate::error::TemplateError;

/// Java `NonNamespaceException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("namespace", actual)
}
