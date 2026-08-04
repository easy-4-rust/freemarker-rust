//! 就地列表项 —— 对应 Java `freemarker.core.Items`
//! （accept → loopForItemsElement，Items.java:40-48 / IteratorBlock.java:230-250）

use crate::core::exec::ExecOutcome;
use crate::core::Element;
use crate::error::{Result, TemplateError};

/// `<#items as x[, y]>` 就地元素（对应 Items.java；render 时由最近的 #list
/// 迭代上下文驱动 body 逐项执行）
pub struct Items {
    pub var: String,
    pub var2: Option<String>,
    pub body: Vec<Element>,
}

impl Items {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(var: String, var2: Option<String>, body: Vec<Element>) -> Self {
        Items { var, var2, body }
    }

    /// 执行（Java accept → loopForItemsElement）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_items(env, &self.var, &self.var2, &self.body)
    }
}

pub(crate) fn exec_items(
    env: &mut crate::core::Environment,
    var: &str,
    var2: &Option<String>,
    body: &[Element],
) -> Result<ExecOutcome> {
    let lc = env
        .get_loop_context(None)
        .ok_or_else(|| TemplateError::misc("#items without iteration in context"))?;
    {
        let mut c = lc.borrow_mut();
        if c.items_entered {
            return Err(TemplateError::misc(
                "The #items directive was already entered earlier for this listing.",
            ));
        }
        c.items_entered = true;
        c.var_name = var.to_string();
        c.var2_name = var2.clone();
    }
    let r = crate::core::iterator_block::run_loop_iterations(env, &lc, body);
    {
        let mut c = lc.borrow_mut();
        c.var_name.clear();
        c.var2_name = None;
    }
    r
}
