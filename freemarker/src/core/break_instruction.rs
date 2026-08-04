//! 中断指令 —— 对应 Java `freemarker.core.BreakInstruction`
//! （accept：抛 BreakOrContinueException.BREAK_INSTANCE，由 `#list` 捕获）

use crate::core::exec::ExecOutcome;
use crate::error::{FlowKind, Result};

/// `<#break>` 指令（对应 BreakInstruction.java；无字段）
pub struct BreakInstruction;

impl BreakInstruction {
    /// 构造（Java 无参构造器；Rust 侧由解析器产生）
    pub fn new() -> Self {
        BreakInstruction
    }

    /// 执行（Java accept → BreakOrContinueException.BREAK_INSTANCE）
    pub(crate) fn exec(&self, _env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        Ok(ExecOutcome::Flow(FlowKind::Break))
    }
}
