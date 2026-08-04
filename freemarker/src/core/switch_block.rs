//! 分支块 —— 对应 Java `freemarker.core.SwitchBlock`
//! （accept：子块按源码序 fall-through；break/continue 均视为 break :108-115；
//! default 按源码位参与，legacy 怪癖：default 可不在末尾且不后落到后续 case）

use crate::core::environment::RunSignal;
use crate::core::eval;
use crate::core::exec::ExecOutcome;
use crate::core::{CaseDef, Element, Expr};
use crate::error::{Result, TemplateError};

/// `<#switch>` 块（对应 SwitchBlock.java）
pub struct SwitchBlock {
    pub expr: Expr,
    pub cases: Vec<CaseDef>,
    pub default: Option<Vec<Element>>,
    /// default 在源码序列中的位置（0 起始；Java SwitchBlock 子块按源码序）
    pub default_pos: Option<usize>,
}

impl SwitchBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(
        expr: Expr,
        cases: Vec<CaseDef>,
        default: Option<Vec<Element>>,
        default_pos: Option<usize>,
    ) -> Self {
        SwitchBlock {
            expr,
            cases,
            default,
            default_pos,
        }
    }

    /// 执行（Java accept）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_switch(
            env,
            &self.expr,
            &self.cases,
            &self.default,
            &self.default_pos,
        )
    }
}

fn exec_switch(
    env: &mut crate::core::Environment,
    expr: &crate::core::Expr,
    cases: &[crate::core::CaseDef],
    default: &Option<Vec<Element>>,
    default_pos: &Option<usize>,
) -> Result<ExecOutcome> {
    let searched = eval::eval(env, expr)?;
    let mut matched: Option<usize> = None;
    for (i, c) in cases.iter().enumerate() {
        let v = eval::eval(env, &c.value)?;
        if eval::compare_models(env, &searched, &v, eval::CmpOp::Eq)? {
            matched = Some(i);
            break;
        }
    }
    let mut r = ExecOutcome::Done;
    let mut stopped_by_flow = false;
    match matched {
        Some(start) => {
            // Java SwitchBlock.accept：子块按源码序 fall-through（default 也按源码位参与）。
            // default 之前的 case 下标直接映射源码位；default 之后的 case 源码位 +1。
            let source_start = match default_pos {
                Some(dp) if *dp <= start => start + 1,
                _ => start,
            };
            let mut src_idx = source_start;
            loop {
                // 源码序取下一个子块：default 在 default_pos 位
                let block: &[Element] = match default_pos {
                    Some(dp) if *dp == src_idx => {
                        if let Some(d) = default {
                            d
                        } else {
                            break;
                        }
                    }
                    _ => {
                        let case_idx = match default_pos {
                            Some(dp) if *dp < src_idx => src_idx - 1,
                            _ => src_idx,
                        };
                        match cases.get(case_idx) {
                            Some(c) => &c.body,
                            None => break,
                        }
                    }
                };
                match env.run(block) {
                    Ok(RunSignal::Completed) => {}
                    Ok(RunSignal::Returned(v)) => {
                        r = ExecOutcome::ReturnValue(v);
                        break;
                    }
                    // Java SwitchBlock :108-115：break/continue 均视为 break
                    Err(TemplateError::Flow(_)) => {
                        stopped_by_flow = true;
                        break;
                    }
                    Err(e) => return Err(e),
                }
                src_idx += 1;
            }
            let _ = stopped_by_flow;
        }
        None => {
            // 未匹配：仅执行 default（Java legacy 怪癖：default 不后落到后续 case）
            if let Some(d) = default {
                match env.run(d) {
                    Ok(RunSignal::Completed) => {}
                    Ok(RunSignal::Returned(v)) => r = ExecOutcome::ReturnValue(v),
                    Err(TemplateError::Flow(_)) => {}
                    Err(e) => return Err(e),
                }
            }
        }
    }
    Ok(r)
}
