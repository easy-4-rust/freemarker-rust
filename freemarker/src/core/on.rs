//! 节点处理器注册指令 —— 对应 Java `freemarker.core.On`
//! （2.3.28+：visit 块内的节点类型处理器；`<#on name>body</#on>` 与
//! `<#macro name>body</#macro>` 等价——On.java 内部注册为命名宏）

use crate::core::exec::{eval_to_string, ExecOutcome};
use crate::core::{Element, Expr};
use crate::error::Result;
use std::rc::Rc;

/// `<#on name>body</#on>`（对应 On.java）
pub struct On {
    pub expr: Expr,
    pub body: Vec<Element>,
}

impl On {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(expr: Expr, body: Vec<Element>) -> Self {
        On { expr, body }
    }

    /// 执行（Java accept：注册为命名宏）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let name = eval_to_string(env, &self.expr)?;
        let mv = macro_from_body(name.clone(), self.body.clone());
        env.get_current_namespace().put_macro(name, mv);
        Ok(ExecOutcome::Done)
    }
}

fn macro_from_body(name: String, body: Vec<Element>) -> Rc<crate::core::environment::MacroValue> {
    let def = Rc::new(crate::core::MacroDef {
        name,
        is_function: false,
        params: Vec::new(),
        body,
        namespace: None,
        template_name: String::new(),
        span: crate::span::Span::new(0, 0),
    });
    Rc::new(crate::core::environment::MacroValue {
        def,
        ns: std::rc::Weak::new(),
        with_args: None,
    })
}
