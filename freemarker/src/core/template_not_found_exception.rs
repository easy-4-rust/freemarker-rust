//! 模板未找到异常 —— 对应 Java `freemarker.template.TemplateNotFoundException`
//! （`Template not found for name "{name}".`；template-loader 解析失败路径）

use crate::error::TemplateError;

/// Java `TemplateNotFoundException(String name)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(name: impl Into<String>) -> TemplateError {
    TemplateError::NotFound { name: name.into() }
}
