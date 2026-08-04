//! 模板模型异常 —— 对应 Java `freemarker.template.TemplateModelException`
//! （模板模型层的运行时错误；Rust 侧为 `TemplateError::Model`）

use crate::error::TemplateError;

/// Java `TemplateModelException(String description)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::Model {
        message: message.into(),
    }
}
