//! 停止异常 —— 对应 Java `freemarker.core.StopException`
//! （`<#stop "msg">` 的 msg 原样为消息；无消息 → "[No error description was
//! available.]"，jar 实测 stop_plain）

use crate::error::TemplateError;

/// Java `StopException(String message)` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: Option<String>) -> TemplateError {
    TemplateError::Stop { message }
}
