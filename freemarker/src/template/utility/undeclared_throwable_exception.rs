//! 未声明异常包装 —— 对应 Java
//! `freemarker.template.utility.UndeclaredThrowableException`
//! （内部异常包装；Java 运行时工具——v1 用 TemplateError 承载）

use crate::template::template_exception::TemplateException;
use std::fmt;

/// 未声明异常包装（对应 UndeclaredThrowableException.java）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndeclaredThrowableException(pub String);

impl fmt::Display for UndeclaredThrowableException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UndeclaredThrowableException {}

impl From<UndeclaredThrowableException> for TemplateException {
    fn from(e: UndeclaredThrowableException) -> Self {
        TemplateException(e.0)
    }
}
