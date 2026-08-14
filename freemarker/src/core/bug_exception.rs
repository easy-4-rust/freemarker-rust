//! 内部缺陷异常 —— 对应 Java `freemarker.core.BugException`
//! （RuntimeException；标记 FreeMarker 内部逻辑错误，不应由模板作者触发）

use crate::error::TemplateError;

/// Java `BugException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: impl Into<String>) -> TemplateError {
    TemplateError::misc(format!(
        "A bug was detected in FreeMarker; please report it with stack-trace: {}",
        message.into()
    ))
}
