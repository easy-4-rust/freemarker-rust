//! 中断/继续信号 —— 对应 Java `freemarker.core.BreakOrContinueException`
//! （`<#break>`/`<#continue>` 的内部控制流信号，由 `#list` 捕获；Rust 侧为
//! `TemplateError::Flow(FlowKind)`，消息仅用于循环外非法使用）

use crate::error::{FlowKind, TemplateError};

/// Java `BreakOrContinueException` 的 Rust 入口（消息见 FlowKind Display）
#[allow(dead_code)]
pub(crate) fn new(kind: FlowKind) -> TemplateError {
    TemplateError::Flow(kind)
}
