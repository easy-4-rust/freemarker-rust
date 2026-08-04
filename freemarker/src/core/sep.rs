//! 就地分隔元素 —— 对应 Java `freemarker.core.Sep`
//! （accept :35-47：当前迭代 hasNext 时渲染 body）

use crate::core::environment::RunSignal;
use crate::core::exec::ExecOutcome;
use crate::core::Element;
use crate::error::{Result, TemplateError};

/// `<#sep>` 就地元素（对应 Sep.java；当前迭代 hasNext 时渲染 body）
pub struct Sep {
    pub body: Vec<Element>,
}

impl Sep {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(body: Vec<Element>) -> Self {
        Sep { body }
    }

    /// 执行（Java accept）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_sep(env, &self.body)
    }
}

fn exec_sep(env: &mut crate::core::Environment, body: &[Element]) -> Result<ExecOutcome> {
    let lc = env
        .get_loop_context(None)
        .ok_or_else(|| TemplateError::misc("#sep without iteration in context"))?;
    if !lc.borrow().has_next {
        return Ok(ExecOutcome::Done);
    }
    match env.run(body) {
        Ok(RunSignal::Completed) => Ok(ExecOutcome::Done),
        Ok(RunSignal::Returned(v)) => Ok(ExecOutcome::ReturnValue(v)),
        Err(e) => Err(e),
    }
}
