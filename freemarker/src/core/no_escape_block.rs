//! 不转义块 —— 对应 Java `freemarker.core.NoEscapeBlock`
//! （关闭外层 escape 与自动转义）

use crate::core::environment::EscapeState;
use crate::core::exec::{outcome_from_run, ExecOutcome};
use crate::core::Element;
use crate::error::Result;

/// `<#noescape>` 块（对应 NoEscapeBlock.java）
pub struct NoEscapeBlock {
    pub body: Vec<Element>,
}

impl NoEscapeBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(body: Vec<Element>) -> Self {
        NoEscapeBlock { body }
    }

    /// 执行（Java accept → pushEscape(PLAIN)/visit/popEscape）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        env.push_escape(EscapeState::Plain);
        let r = env.run(&self.body);
        env.pop_escape();
        outcome_from_run(r)
    }
}
