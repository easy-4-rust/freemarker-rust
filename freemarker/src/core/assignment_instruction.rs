//! 多赋值指令 —— 对应 Java `freemarker.core.AssignmentInstruction`
//! （FTL.jj 3371-3378：`<#assign a=1, b=2>` 逐个执行）

use crate::core::exec::ExecOutcome;
use crate::core::Element;
use crate::error::Result;

/// `<#assign a=1, b=2>` 多赋值（对应 AssignmentInstruction.java）
pub struct AssignmentInstruction {
    pub els: Vec<Element>,
}

impl AssignmentInstruction {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(els: Vec<Element>) -> Self {
        AssignmentInstruction { els }
    }

    /// 执行（Java accept：逐个执行子赋值）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        for e in &self.els {
            let outcome = crate::core::exec::exec(env, e)?;
            if !matches!(outcome, ExecOutcome::Done) {
                return Ok(outcome);
            }
        }
        Ok(ExecOutcome::Done)
    }
}
