//! 模板异常 —— 对应 Java `freemarker.template.TemplateException`
//! （Java :669 行：模板执行/解析错误基类——环境、栈跟踪、cause 链；
//! Rust 引擎内部使用 `TemplateError`（error/template_error.rs 合并全部异常
//! 层级，消息逐字对齐）——本类型为 Java API 对应物）

use std::fmt;

/// 模板异常（对应 TemplateException.java；Rust 引擎内部等价物 = TemplateError）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateException(pub String);

impl fmt::Display for TemplateException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TemplateException {}

impl From<crate::error::TemplateError> for TemplateException {
    fn from(e: crate::error::TemplateError) -> Self {
        TemplateException(e.to_string())
    }
}
