//! 自动转义块 —— 对应 Java `freemarker.core.AutoEscBlock`
//! （块内开启自动转义）

use crate::core::exec::{outcome_from_run, ExecOutcome};
use crate::core::Element;
use crate::error::Result;

/// `<#autoesc>` 块（对应 AutoEscBlock.java）
pub struct AutoEscBlock {
    pub body: Vec<Element>,
}

impl AutoEscBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(body: Vec<Element>) -> Self {
        AutoEscBlock { body }
    }

    /// 执行（Java accept → 块内开启自动转义）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let prev = env.is_auto_escape();
        env.set_auto_escape(true);
        let r = env.run(&self.body);
        env.set_auto_escape(prev);
        outcome_from_run(r)
    }
}
