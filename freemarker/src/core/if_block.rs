//! 条件块 —— 对应 Java `freemarker.core.IfBlock`
//! （accept :43-61：条件求布尔（modelToBoolean——classic 兼容模式下缺失/空值 →
//! false）；then/else 子块；elseif 链扁平化下钻热路径）

use crate::core::eval;
use crate::core::exec::ExecOutcome;
use crate::core::Element;
use crate::core::ElementKind;
use crate::error::{Result, TemplateError};
use crate::span::Span;

/// `<#if>` 块（对应 IfBlock.java；elseif 已扁平化为嵌套 If 的 else 分支）
pub struct IfBlock {
    pub cond: crate::core::Expr,
    pub then: Vec<Element>,
    pub else_: Option<Vec<Element>>,
    /// 元素源码位置（Java IfBlock 元素位置 blame）
    pub span: Span,
}

impl IfBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(
        cond: crate::core::Expr,
        then: Vec<Element>,
        else_: Option<Vec<Element>>,
        span: Span,
    ) -> Self {
        IfBlock {
            cond,
            then,
            else_,
            span,
        }
    }

    /// 执行（Java accept）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        // Java IfBlock.accept :43-61：条件求布尔（modelToBoolean——classic 兼容
        // 模式下缺失/空值 → false）；then/else 子块。
        // elseif 链扁平化下钻：else 分支为单个 If 元素时（`<#elseif>`/`<#else>`
        // 内嵌 `<#if>` 结构同形）沿链求值，不克隆未命中分支——热路径
        // （长 elseif 链）避免每次条件失败都深克隆剩余整条链。
        exec_if(env, self.span, &self.cond, &self.then, &self.else_)
    }
}

fn blame_if_condition(
    e: TemplateError,
    env: &mut crate::core::Environment,
    cond: &crate::core::Expr,
) -> TemplateError {
    if let TemplateError::TypeMismatch { ctx, .. } = &e {
        if ctx.blamer.is_none() {
            return e.with_blame_at(
                "#if",
                "condition",
                &crate::core::environment::expr_desc(cond),
                &env.current_template_name,
                cond.span,
            );
        }
    }
    crate::core::environment::attach_location(e, &env.current_template_name, cond.span)
}

fn exec_if(
    env: &mut crate::core::Environment,
    span: Span,
    cond: &crate::core::Expr,
    then: &[Element],
    else_: &Option<Vec<Element>>,
) -> Result<ExecOutcome> {
    let mut cur_span = span;
    let mut cond = cond;
    let mut then = then;
    let mut else_ = else_;
    loop {
        let cm = eval::eval(env, cond).map_err(|e| {
            crate::core::environment::attach_location(e, &env.current_template_name, cur_span)
        })?;
        // Java IfBlock.accept → condition.evalToBoolean：条件类型错误 blame 条件表达式
        // —— `For "#if" condition: Expected a boolean, but this has evaluated to a
        // {type}: ==> {cond}`（位置 = 条件表达式起始）
        let b = eval::model_to_boolean(env, &cm).map_err(|e| blame_if_condition(e, env, cond))?;
        if b {
            return Ok(ExecOutcome::Next(then.to_vec()));
        }
        match else_ {
            Some(v) if v.len() == 1 => {
                if let ElementKind::If {
                    cond: c2,
                    then: t2,
                    else_: e2,
                } = &v[0].kind
                {
                    cur_span = v[0].span;
                    cond = c2;
                    then = t2;
                    else_ = e2;
                    continue;
                }
                return Ok(ExecOutcome::Next(v.clone()));
            }
            Some(v) => return Ok(ExecOutcome::Next(v.clone())),
            None => return Ok(ExecOutcome::Done),
        }
    }
}
