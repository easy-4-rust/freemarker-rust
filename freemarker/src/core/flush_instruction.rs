//! 冲刷指令 —— 对应 Java `freemarker.core.FlushInstruction`

use crate::core::exec::ExecOutcome;
use crate::error::{Result, TemplateError};

/// `<#flush>` 指令（对应 FlushInstruction.java；无字段）
pub struct FlushInstruction;

impl FlushInstruction {
    /// 构造（Java 无参构造器；Rust 侧由解析器产生）
    pub fn new() -> Self {
        FlushInstruction
    }

    /// 执行（Java accept：冲刷输出缓冲）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        // Java FlushInstruction：冲刷输出缓冲
        if env.redirect.is_some() {
            return Ok(ExecOutcome::Done);
        }
        env.out.flush().map_err(TemplateError::Io)?;
        Ok(ExecOutcome::Done)
    }
}
