//! 递归访问指令 —— 对应 Java `freemarker.core.RecurseNode`
//! （accept：对节点的子节点逐个 visit；无参 = 当前访问节点的子节点）

use crate::core::eval;
use crate::core::exec::ExecOutcome;
use crate::core::Expr;
use crate::error::{Result, TemplateError};

/// `<#recurse expr>`（对应 RecurseNode.java；using 目标 XML 场景才有意义，v1 仅解析保留）
pub struct RecurseNode {
    pub expr: Option<Expr>,
    /// `<#recurse node using target>`（Java RecurseNode 的 recurseTarget；
    /// XML 场景才有意义，v1 仅解析保留）
    #[allow(dead_code)]
    pub using: Option<Expr>,
}

impl RecurseNode {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(expr: Option<Expr>, using: Option<Expr>) -> Self {
        RecurseNode { expr, using }
    }

    /// 执行（Java accept：对节点的子节点逐个 visit）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let node = match &self.expr {
            Some(e) => eval::eval(env, e)?,
            None => env.get_current_visitor_node().ok_or_else(|| {
                TemplateError::misc(
                    "#recurse must be given a node, or be called while visiting a node",
                )
            })?,
        };
        let children = match &node.node {
            Some(n) => n.children()?,
            None => Vec::new(),
        };
        for c in children {
            env.push_visitor_node(c.clone());
            let r = crate::core::visit_node::visit_node(env, &c);
            env.pop_visitor_node();
            r?;
        }
        Ok(ExecOutcome::Done)
    }
}
