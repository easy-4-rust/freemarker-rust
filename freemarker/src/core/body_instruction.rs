//! 嵌套指令 —— 对应 Java `freemarker.core.BodyInstruction`
//! （`<#nested>` 宏体回插；Context 构造 :122-155，invokeNestedContent :606-631）

use crate::core::environment::LocalEntry;
use crate::core::eval;
use crate::core::exec::{outcome_from_run, ExecOutcome};
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use std::collections::HashMap;
use std::rc::Rc;

/// `<#nested>` 指令（对应 BodyInstruction.java）
pub struct BodyInstruction {
    pub args: Vec<Expr>,
}

impl BodyInstruction {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(args: Vec<Expr>) -> Self {
        BodyInstruction { args }
    }

    /// 执行（Java accept → invokeNestedContent）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_nested(env, &self.args)
    }
}

fn exec_nested(
    env: &mut crate::core::Environment,
    args: &[crate::core::Expr],
) -> Result<ExecOutcome> {
    let frame = env.get_current_macro_frame().ok_or_else(|| {
        TemplateError::misc("Cannot use a \"nested\" instruction outside a macro.")
    })?;
    // Java BodyInstruction.Context 构造（:122-148）：参数在宏上下文求值
    let mut arg_values = Vec::new();
    for a in args {
        arg_values.push(eval::eval(env, a)?);
    }
    let call_body = match &frame.call_body {
        Some(b) => b.clone(),
        None => return Ok(ExecOutcome::Done), // 无调用方 body → 无操作（Java childBuffer==null）
    };
    // 体参数（<@m ; a, b> 中 a/b 按位置绑定 <#nested v1 v2> 的 v1/v2；
    // Java BodyInstruction.Context :122-155）
    let mut body_vars = HashMap::new();
    for (i, bp) in frame.body_params.iter().enumerate() {
        if let Some(v) = arg_values.get(i) {
            body_vars.insert(bp.clone(), v.clone());
        }
    }
    // Java invokeNestedContent :606-631：切换到调用方上下文
    let prev_macro = env.macro_frames.pop();
    let prev_ns = std::mem::replace(&mut env.current_ns, frame.caller_ns.clone());
    let prev_local = std::mem::take(&mut env.local_stack);
    // 词法宏名随调用方上下文恢复（调用方 body 元素的帧 `in macro "m"` 定位；
    // Java getEnclosingMacro 沿父元素链）
    let prev_macro_name =
        std::mem::replace(&mut env.current_macro_name, frame.caller_macro_name.clone());
    // 词法模板名随调用方上下文恢复（嵌套内容词法上位于调用点模板 ——
    // Java getCurrentTemplate 的指令栈顶元素 template 语义；帧记录调用时的值）
    let fallback_lexical = env.lexical_template_name.clone();
    let prev_lexical = std::mem::replace(
        &mut env.lexical_template_name,
        frame
            .prev_lexical_template_name
            .clone()
            .unwrap_or(fallback_lexical),
    );
    env.local_stack = frame.caller_local_stack.clone();
    if !frame.body_params.is_empty() {
        env.push_local(LocalEntry::Body(Rc::new(
            crate::core::environment::BodyCtx { vars: body_vars },
        )));
    }
    let r = env.run(&call_body);
    // 恢复
    env.current_macro_name = prev_macro_name;
    env.local_stack = prev_local;
    env.current_ns = prev_ns;
    env.lexical_template_name = prev_lexical;
    if let Some(f) = prev_macro {
        env.macro_frames.push(f);
    }
    outcome_from_run(r)
}
