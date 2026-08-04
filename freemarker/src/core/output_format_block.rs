//! 输出格式块 —— 对应 Java `freemarker.core.OutputFormatBlock`
//! （块内切换 outputFormat；v1：仅影响插值自动转义）

use crate::core::exec::{eval_to_string, outcome_from_run, ExecOutcome};
use crate::core::{Element, Expr, OutputFormatKind};
use crate::error::{Result, TemplateError};

/// `<#outputformat "HTML">` 块（对应 OutputFormatBlock.java）
pub struct OutputFormatBlock {
    pub name: Expr,
    pub body: Vec<Element>,
}

impl OutputFormatBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(name: Expr, body: Vec<Element>) -> Self {
        OutputFormatBlock { name, body }
    }

    /// 执行（Java accept → 块内切换 outputFormat）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let n = eval_to_string(env, &self.name)?;
        let fmt = OutputFormatKind::parse(&n)
            .ok_or_else(|| TemplateError::misc(format!("Unknown output format: {n}")))?;
        let prev = env.settings.output_format;
        env.settings.to_mut().output_format = fmt;
        let r = env.run(&self.body);
        env.settings.to_mut().output_format = prev;
        outcome_from_run(r)
    }
}
