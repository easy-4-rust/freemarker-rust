//! 循环变量轮换内建 —— 对应 Java `BuiltInsForLoopVariables.java`（item_cycle/item_parity/
//! item_parity_cap；index/counter/has_next/is_* 在 eval.rs loop_state_builtin）
//!
//! 语义要点（Java 对照）：
//! - `?item_cycle(a, b, ...)` → item_cycleBI：返回方法模型（exec(args) = args[index % len]）；
//! - `?item_parity` → 每项轮换 "odd"/"even"；`?item_parity_cap` → "Odd"/"Even"；
//! - 目标须为循环变量（Java BuiltInForLoopVariable 的迭代上下文定位）。

use crate::core::eval_util::check_arg_count;
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateMethodModelEx};
use std::rc::Rc;

/// 定位循环上下文（Java BuiltInForLoopVariable：最近/同名循环层）
fn loop_index(env: &Environment, target: &Expr) -> Result<usize> {
    let target_var = match &target.kind {
        ExprKind::Ident(n) => Some(n.as_str()),
        _ => None,
    };
    let lc = env.get_loop_context(target_var).ok_or_else(|| {
        TemplateError::misc(
            "The target of the built-in is not a loop variable (no enclosing loop in scope)",
        )
    })?;
    let index = lc.borrow().index;
    Ok(index)
}

use crate::core::ExprKind;

/// ?item_parity —— Java item_parityBI：index 偶数 → "odd"，奇数 → "even"
pub fn item_parity(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("item_parity", args, 0, 0)?;
    let idx = loop_index(env, target)?;
    Ok(Some(TModel::from_scalar(
        if idx % 2 == 0 { "odd" } else { "even" }.to_string(),
    )))
}

/// ?item_parity_cap —— Java item_parity_capBI：index 偶数 → "Odd"，奇数 → "Even"
pub fn item_parity_cap(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("item_parity_cap", args, 0, 0)?;
    let idx = loop_index(env, target)?;
    Ok(Some(TModel::from_scalar(
        if idx % 2 == 0 { "Odd" } else { "Even" }.to_string(),
    )))
}

/// ?item_cycle(a, b, ...) —— Java item_cycleBI：返回方法模型（每次 exec 轮换取值）
pub fn item_cycle(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    if arg_count(args) < 1 {
        return Err(TemplateError::misc(
            "?item_cycle expects at least one argument",
        ));
    }
    let idx = loop_index(env, target)?;
    Ok(Some(TModel::from_method(ItemCycleMethod { index: idx })))
}

use crate::core::eval_util::arg_count;

/// 轮换方法（Java item_cycleBI.BIMethod：args[index % args.size()]）
struct ItemCycleMethod {
    index: usize,
}

impl TemplateMethodModelEx for ItemCycleMethod {
    fn exec(&self, _env: &mut Environment, args: Vec<TModel>) -> Result<TModel> {
        if args.is_empty() {
            return Err(TemplateError::misc(
                "?item_cycle expects at least one argument",
            ));
        }
        Ok(args[self.index % args.len()].clone())
    }
}

// 兼容引用（Rc 用于方法模型持有）
#[allow(dead_code)]
fn _rc_ref() -> Rc<()> {
    Rc::new(())
}
