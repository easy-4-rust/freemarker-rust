//! 节点访问指令 —— 对应 Java `freemarker.core.VisitNode`
//! （accept → Environment.visit（:2885-2940）：求值节点（无参 = 当前访问节点），
//! 压入访问栈，按节点名分派宏）

use crate::core::eval;
use crate::core::exec::ExecOutcome;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// `<#visit expr>`（对应 VisitNode.java；using 目标 XML 场景才有意义，v1 仅解析保留）
pub struct VisitNode {
    pub expr: Option<Expr>,
    /// `<#visit node using target>` 的 using 目标（Java VisitNode 的
    /// recurseTarget 参数；XML 场景才有意义，v1 仅解析保留）
    #[allow(dead_code)]
    pub using: Option<Expr>,
}

impl VisitNode {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(expr: Option<Expr>, using: Option<Expr>) -> Self {
        VisitNode { expr, using }
    }

    /// 执行（Java accept → env.visit）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let node = match &self.expr {
            Some(e) => eval::eval(env, e)?,
            None => env.get_current_visitor_node().ok_or_else(|| {
                TemplateError::misc(
                    "#visit must be given a node, or be called while visiting a node",
                )
            })?,
        };
        env.push_visitor_node(node.clone());
        let r = visit_node(env, &node);
        env.pop_visitor_node();
        r
    }
}

pub(crate) fn visit_node(env: &mut crate::core::Environment, node: &TModel) -> Result<ExecOutcome> {
    // 节点名（Java getNodeName：元素 = 标签名；text = "@text" 等，与 ?node_name 一致）
    let node_name = match &node.node {
        Some(n) => n.name()?.unwrap_or_default(),
        None => String::new(),
    };
    let ns = env.get_current_namespace();
    // 1. `@<node_name>` 宏（Java getNodeProcessor，Environment.java :2943-3000）：
    //    带命名空间节点 → 宏名 = 前缀:本地名（NsPrefixes.get_prefix_for_namespace
    //    反查宏所在模板的 ns_prefixes；default ns → 无前缀本地名；未注册前缀 →
    //    该模板不处理，跳过宏查找直接 @default）
    let macro_name = if !node_name.is_empty() {
        match &node.node {
            Some(n) => match n.namespace()? {
                Some(uri) if !uri.is_empty() => {
                    match env.current_ns_prefixes().get_prefix_for_namespace(&uri) {
                        Some(p) if !p.is_empty() => format!("{p}:{node_name}"),
                        // default ns（空前缀）→ 本地名；未注册 → 不查宏
                        Some(_) => node_name.clone(),
                        None => String::new(),
                    }
                }
                _ => node_name.clone(),
            },
            None => String::new(),
        }
    } else {
        String::new()
    };
    if !macro_name.is_empty() {
        if let Some(m) = ns.get_member(&macro_name) {
            if let Some(mv) = env.as_macro(&m) {
                return crate::core::unified_call::call_macro(env, &mv, &[], None, Vec::new());
            }
        }
    }
    // 2. `@default` 宏（Java :2903-2906）
    if let Some(m) = ns.get_member("@default") {
        if let Some(mv) = env.as_macro(&m) {
            return crate::core::unified_call::call_macro(env, &mv, &[], None, Vec::new());
        }
    }
    // 3. 默认行为（Java :2907-2924）
    let node_type = match &node.node {
        Some(n) => n.node_type()?,
        None => String::new(),
    };
    match node_type.as_str() {
        // 文本类节点：标量值写出（Java visitText 语义：text/comment/PI/attr）
        "text" | "comment" | "pi" | "attribute" => {
            if let Some(s) = &node.scalar {
                let text = s.as_string()?;
                env.emit(&text)?;
            }
            Ok(ExecOutcome::Done)
        }
        // element/document：递归 visit 子节点（Java visitNode 默认）
        _ => {
            let children = match &node.node {
                Some(n) => n.children()?,
                None => Vec::new(),
            };
            for c in children {
                env.push_visitor_node(c.clone());
                let r = visit_node(env, &c);
                env.pop_visitor_node();
                r?;
            }
            Ok(ExecOutcome::Done)
        }
    }
}
