//! 模板未找到异常 —— 对应 Java `freemarker.template.TemplateNotFoundException`
//! （Java :61 行：模板加载失败；Rust 引擎内部用
//! `TemplateError::NotFound` 变体）

use crate::template::template_exception::TemplateException;
use std::fmt;

/// 模板未找到异常（对应 TemplateNotFoundException.java）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateNotFoundException {
    pub name: String,
}

impl fmt::Display for TemplateNotFoundException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Template not found for name {}", self.name)
    }
}

impl std::error::Error for TemplateNotFoundException {}

impl From<TemplateNotFoundException> for TemplateException {
    fn from(e: TemplateNotFoundException) -> Self {
        TemplateException(e.to_string())
    }
}
