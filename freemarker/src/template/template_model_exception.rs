//! 模板模型异常 —— 对应 Java `freemarker.template.TemplateModelException`
//! （Java :140 行：模板模型操作错误；TemplateException 子类）

use crate::template::template_exception::TemplateException;
use std::fmt;

/// 模板模型异常（对应 TemplateModelException.java；TemplateException 子类）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateModelException(pub String);

impl fmt::Display for TemplateModelException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TemplateModelException {}

impl From<TemplateModelException> for TemplateException {
    fn from(e: TemplateModelException) -> Self {
        TemplateException(e.0)
    }
}
