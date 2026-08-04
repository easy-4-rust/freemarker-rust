//! 全局赋值指令 —— 对应 Java `freemarker.core.GlobalAssignment`
//! （accept：表达式或块捕获 → 写入全局命名空间）

use crate::core::assignment::{exec_assign, AssignScope};
use crate::core::environment::RunSignal;
use crate::core::exec::ExecOutcome;
use crate::core::{AssignOp, Element, Expr};
use crate::error::Result;
use crate::template::TModel;

/// `<#global>` 指令（对应 GlobalAssignment.java）
pub struct GlobalAssignment {
    pub target: String,
    pub expr: Option<Expr>,
    pub body: Option<Vec<Element>>,
    pub op: AssignOp,
}

impl GlobalAssignment {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(
        target: String,
        expr: Option<Expr>,
        body: Option<Vec<Element>>,
        op: AssignOp,
    ) -> Self {
        GlobalAssignment {
            target,
            expr,
            body,
            op,
        }
    }

    /// 执行（Java accept）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        if let Some(e) = &self.expr {
            exec_assign(env, &self.target, e, &self.op, None, AssignScope::Global)
        } else if let Some(b) = &self.body {
            let captured = env.capture(|env| env.run(b))?;
            if let RunSignal::Returned(v) = captured.0 {
                return Ok(ExecOutcome::ReturnValue(v));
            }
            let value = TModel::from_scalar(captured.1);
            env.set_global_variable(&self.target, value);
            Ok(ExecOutcome::Done)
        } else {
            Ok(ExecOutcome::Done)
        }
    }
}
