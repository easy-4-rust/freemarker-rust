//! API 不支持异常 —— 对应 Java `freemarker.core.APINotSupportedTemplateException`
//! （`?api` 仅适用于适配后的 Java 对象；模板内创建的值不支持）

use crate::error::TemplateError;

/// Java `APINotSupportedTemplateException` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(description: impl Into<String>) -> TemplateError {
    TemplateError::misc(description)
}
