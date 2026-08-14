//! 流程控制异常 —— 对应 Java `freemarker.core.FlowControlException`
//! （RuntimeException 子类；<#stop>/<#return>/<#break>/<#continue> 的内部信号，
//!  不应暴露给模板作者；ThreadInterruptionSupport 也使用子类）

use crate::error::TemplateError;

/// Java `FlowControlException` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new(message: Option<String>) -> TemplateError {
    match message {
        Some(m) => TemplateError::Stop { message: Some(m) },
        None => TemplateError::Stop { message: None },
    }
}
