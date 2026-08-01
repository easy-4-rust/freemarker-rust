//! 惰性条件内建 —— 对应 Java `BuiltInsWithLazyConditionals.java`（then/switch）
//!
//! 语义要点（Java 对照）：
//! - `?then(a, b)` → then_BI：目标 evalToBoolean，只求值选中的分支（惰性）；
//! - `?switch(c1, v1, c2, v2, ...[, default])` → switch_BI：目标求值一次，case 用 `==`
//!   语义比较（EvalUtil.compare），命中返回对应值（惰性）；偶数个参数且未命中 → 报错。

use crate::builtins::eval_util::{arg_count, check_arg_count};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// ?then(a, b) —— Java then_BI：目标为布尔，选择对应分支（只求值一个）
pub fn then(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("then", args, 2, 2)?;
    let m = crate::core::eval::eval(env, target)?;
    let b = m.eval_boolean()?;
    let chosen = if b { 0 } else { 1 };
    let e = &args.unwrap()[chosen];
    let v = crate::core::eval::eval(env, e)?;
    if v.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(e),
        ));
    }
    Ok(Some(v))
}

/// ?switch(c1, v1, c2, v2, ...[, default]) —— Java switch_BI：
/// 目标求值一次；case 按 == 比较；未命中且参数为偶数 → 报错
pub fn switch(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let as_ = args.ok_or_else(|| TemplateError::misc("?switch expects arguments"))?;
    if as_.len() < 2 {
        return Err(TemplateError::misc(
            "?switch must have at least 2 arguments",
        ));
    }
    let target_m = crate::core::eval::eval(env, target)?;
    if target_m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    let n = as_.len();
    let mut i = 0;
    while i + 1 < n {
        let case_m = crate::core::eval::eval(env, &as_[i])?;
        if crate::core::eval::compare_models(env, &target_m, &case_m, crate::core::eval::CmpOp::Eq)?
        {
            let v = crate::core::eval::eval(env, &as_[i + 1])?;
            if v.is_nothing() {
                return Err(TemplateError::invalid_reference(
                    crate::core::environment::expr_desc(&as_[i + 1]),
                ));
            }
            return Ok(Some(v));
        }
        i += 2;
    }
    if n % 2 == 0 {
        return Err(TemplateError::misc("The value before ?switch(case1, value1, case2, value2, ...) didn't match any of the cases, and there was no default value (the last argument without a case before it).".to_string()));
    }
    let v = crate::core::eval::eval(env, &as_[n - 1])?;
    if v.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(&as_[n - 1]),
        ));
    }
    Ok(Some(v))
}

/// 参数计数辅助（避免未使用告警）
#[allow(dead_code)]
fn _argc(args: Option<&[Expr]>) -> usize {
    arg_count(args)
}
