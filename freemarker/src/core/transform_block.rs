//! 变换块 —— 对应 Java `freemarker.core.TransformBlock`
//! （accept :64-85 → env.visitAndTransform（Environment.java:495-543）：
//! getWriter 先产出变换自身输出（`?interpret` 即 include 解释模板），
//! body 写入变换 writer（StandardCompress 等压缩/转义 body），close 时变换输出）

use crate::core::environment::{expr_desc, RunSignal};
use crate::core::eval::eval;
use crate::core::exec::ExecOutcome;
use crate::core::{Element, Expr};
use crate::error::{Result, TemplateError};
use std::collections::HashMap;

/// `<#transform expr>body</#transform>`（对应 TransformBlock.java；
/// 旧式 TemplateTransformModel 指令；`?interpret` 产物为变换模型）
pub struct TransformBlock {
    pub expr: Expr,
    pub body: Vec<Element>,
}

impl TransformBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(expr: Expr, body: Vec<Element>) -> Self {
        TransformBlock { expr, body }
    }

    /// 执行（Java accept → env.visitAndTransform）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let m = eval(env, &self.expr)?;
        if m.is_nothing() {
            return Err(TemplateError::invalid_reference(expr_desc(&self.expr)));
        }
        let Some(ttm) = env.as_transform(&m) else {
            return Err(TemplateError::type_mismatch("transform", m.type_name));
        };
        let signal = ttm.transform_with_body(env, &HashMap::new(), &self.body)?;
        match signal {
            RunSignal::Returned(v) => Ok(ExecOutcome::ReturnValue(v)),
            _ => Ok(ExecOutcome::Done),
        }
    }
}
