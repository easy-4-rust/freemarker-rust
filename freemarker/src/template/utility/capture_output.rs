//! 输出捕获变换 —— 对应 Java `freemarker.template.utility.CaptureOutput`
//! （<#capture_output> 共享变换：捕获嵌套输出；Java 依赖 writer 管道）
//! v1 占位：输出捕获由 env.capture 承载（块捕获指令），本变换为透传
//! （文档化偏差，见 docs/10）

use crate::core::environment::RunSignal;
use crate::core::{Element, Environment};
use crate::error::Result;
use crate::template::{TModel, TemplateTransformModel};
use std::collections::HashMap;

/// 输出捕获变换（对应 CaptureOutput.java；v1 透传）
pub struct CaptureOutputTransform;

impl TemplateTransformModel for CaptureOutputTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        _params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        // v1：env.capture 等价 Java 的 writer 管道捕获
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&captured)?;
        Ok(signal)
    }
}
