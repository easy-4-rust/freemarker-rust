//! 块赋值指令 —— 对应 Java `freemarker.core.BlockAssignment`
//! （块输出捕获为字符串后赋值）

use crate::core::assignment::{exec_assign, AssignScope};
use crate::core::environment::RunSignal;
use crate::core::exec::ExecOutcome;
use crate::core::{AssignOp, Element, Expr};
use crate::error::Result;
use crate::span::Span;

/// `<#assign name>body</#assign>` 块捕获（对应 BlockAssignment.java）
pub struct BlockAssignment {
    pub target: String,
    pub body: Vec<Element>,
    pub op: AssignOp,
    pub namespace: Option<Expr>,
    /// 元素源码位置（Java 以 BlockAssignment 元素位置 blame）
    pub span: Span,
}

impl BlockAssignment {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(
        target: String,
        body: Vec<Element>,
        op: AssignOp,
        namespace: Option<Expr>,
        span: Span,
    ) -> Self {
        BlockAssignment {
            target,
            body,
            op,
            namespace,
            span,
        }
    }

    /// 执行（Java accept：块输出捕获为字符串后赋值）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let (sig, text) = env.capture(|env| env.run(&self.body))?;
        match sig {
            RunSignal::Returned(v) => Ok(ExecOutcome::ReturnValue(v)),
            RunSignal::Completed => {
                let placeholder =
                    crate::core::Expr::new(crate::core::ExprKind::Str(text), self.span);
                exec_assign(
                    env,
                    &self.target,
                    &placeholder,
                    &self.op,
                    self.namespace.as_ref(),
                    AssignScope::Namespace,
                )
            }
        }
    }
}
