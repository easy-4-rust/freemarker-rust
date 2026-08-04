//! 畸形模板名异常 —— 对应 Java `freemarker.template.MalformedTemplateNameException`
//! （Java :56 行：模板名不符合 TemplateNameFormat）

use crate::template::template_exception::TemplateException;
use std::fmt;

/// 畸形模板名异常（对应 MalformedTemplateNameException.java）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalformedTemplateNameException {
    pub name: String,
    pub reason: String,
}

impl fmt::Display for MalformedTemplateNameException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Malformed template name {}: {}", self.name, self.reason)
    }
}

impl std::error::Error for MalformedTemplateNameException {}

impl From<MalformedTemplateNameException> for TemplateException {
    fn from(e: MalformedTemplateNameException) -> Self {
        TemplateException(e.to_string())
    }
}
