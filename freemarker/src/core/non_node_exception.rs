//! 非节点类型异常 —— 对应 Java `freemarker.core.NonNodeException`
//! （UnexpectedTypeException 子类；expected 描述 = "node"）

use crate::error::TemplateError;

/// Java `NonNodeException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(actual: impl Into<String>) -> TemplateError {
    TemplateError::type_mismatch("node", actual)
}
