//! 停止指令 —— 对应 Java `freemarker.core.StopInstruction`
//! （accept :43-49：抛 StopException；消息经 evalAndCoerceToPlainText）

use crate::core::exec::{eval_to_string, ExecOutcome};
use crate::core::Expr;
use crate::error::Result;

/// `<#stop>` / `<#stop "msg">` 指令（对应 StopInstruction.java）
pub struct StopInstruction {
    pub msg: Option<Expr>,
}

impl StopInstruction {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(msg: Option<Expr>) -> Self {
        StopInstruction { msg }
    }

    /// 执行（Java accept → StopException）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let message = match &self.msg {
            Some(e) => Some(eval_to_string(env, e)?),
            None => None,
        };
        Ok(ExecOutcome::Stop(message))
    }
}
