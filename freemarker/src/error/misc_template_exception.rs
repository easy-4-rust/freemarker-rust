//! 通用运行时异常 —— 对应 Java `freemarker.core.MiscTemplateException`
//! （_MiscTemplateException 的公开包装；Rust 侧与内部版共用 `TemplateError::Misc`，
//! 构造见 _misc_template_exception.rs）

use crate::error::TemplateError;

/// Java `MiscTemplateException(Environment, String)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    crate::error::_misc_template_exception::new(message)
}
