//! 继续指令 —— 对应 Java `freemarker.core.ContinueInstruction`
//! （accept：抛 BreakOrContinueException.CONTINUE_INSTANCE，由 `#list` 捕获）

use crate::core::exec::ExecOutcome;
use crate::error::{FlowKind, Result};

/// `<#continue>` 指令（对应 ContinueInstruction.java；无字段）
pub struct ContinueInstruction;

impl ContinueInstruction {
    /// 构造（Java 无参构造器；Rust 侧由解析器产生）
    pub fn new() -> Self {
        ContinueInstruction
    }

    /// 执行（Java accept → BreakOrContinueException.CONTINUE_INSTANCE）
    pub(crate) fn exec(&self, _env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        Ok(ExecOutcome::Flow(FlowKind::Continue))
    }
}
