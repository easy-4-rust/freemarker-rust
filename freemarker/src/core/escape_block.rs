//! 转义块 —— 对应 Java `freemarker.core.EscapeBlock`
//! （块内插值统一应用转义；v1 运行时转义栈，Java 在解析期包装插值，行为等价）

use crate::core::environment::EscapeState;
use crate::core::exec::{outcome_from_run, ExecOutcome};
use crate::core::Element;
use crate::error::Result;
use std::rc::Rc;

/// `<#escape expr>` 块（对应 EscapeBlock.java）
pub struct EscapeBlock {
    pub expr: crate::core::Expr,
    pub body: Vec<Element>,
}

impl EscapeBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(expr: crate::core::Expr, body: Vec<Element>) -> Self {
        EscapeBlock { expr, body }
    }

    /// 执行（Java accept → pushEscape/visit/popEscape）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        // Java EscapeBlock：body 内插值统一应用转义（v1 运行时转义栈；
        // Java 在解析期包装插值，行为等价）
        let state = match &self.expr.kind {
            crate::core::ExprKind::Ident(n) if n == "html" => EscapeState::Html,
            crate::core::ExprKind::Ident(n) if n == "xml" => EscapeState::Xml,
            crate::core::ExprKind::Ident(n) if n == "xhtml" => EscapeState::Html, // v1：xhtml 按 html
            _ => EscapeState::Custom(Rc::new(self.expr.clone())),
        };
        env.push_escape(state);
        let r = env.run(&self.body);
        env.pop_escape();
        outcome_from_run(r)
    }
}
