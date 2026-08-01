//! 内建函数共享工具 —— 对应 Java `freemarker.core.EvalUtil`（子集）
//! （coerceModelToStringOrMarkup / modelsEqual 等；文档见 docs/04 §5）

use crate::core::environment::model_to_string;
use crate::core::Environment;
use crate::error::Result;
use crate::template::TModel;
use crate::value::TNumber;
use std::cmp::Ordering;

/// 模型 → 字符串（Java `EvalUtil.coerceModelToStringOrMarkup`）：
/// 字符串内建（?replace/?split/?upper_case 等）对非标量目标按此规则强制转换——
/// 数字按 number_format、布尔按 boolean_format、日期按 date_format（v1 RFC3339）。
pub fn coerce_to_string(env: &mut Environment, m: &TModel) -> Result<String> {
    if m.is_nothing() {
        return Err(crate::error::TemplateError::invalid_reference(
            "The value is null or missing",
        ));
    }
    model_to_string(env, m)
}

/// 宽松模型相等（Java `SequenceBuiltins.modelsEqual`：数字按值、字符串按内容、布尔相同，
/// 其余类型不等——不报错）
pub fn models_equal(a: &TModel, b: &TModel) -> Result<bool> {
    if a.is_number() && b.is_number() {
        return Ok(a
            .get_number()?
            .as_big_decimal()
            .cmp(&b.get_number()?.as_big_decimal())
            == Ordering::Equal);
    }
    if a.is_scalar() && b.is_scalar() {
        return Ok(a.get_scalar()? == b.get_scalar()?);
    }
    if a.is_boolean() && b.is_boolean() {
        return Ok(a.get_boolean()? == b.get_boolean()?);
    }
    Ok(false)
}

/// 参数个数检查（Java `BuiltIn.checkMethodArgCount` → `_MessageUtil.newArgCntError`：
/// "?{name}(...) expects 1 or 2 arguments but has received none./{n}."——min==max 时
/// "expects {n} argument(s)"；max-min==1 时 "expects {min} or {max}"；其余 "to"）
pub fn check_arg_count(
    name: &str,
    args: Option<&[crate::core::Expr]>,
    min: usize,
    max: usize,
) -> Result<()> {
    let n = args.map_or(0, |a| a.len());
    if n < min || n > max {
        let cnt_desc = if min == max {
            if max == 0 {
                "no".to_string()
            } else {
                max.to_string()
            }
        } else if max - min == 1 {
            format!("{min} or {max}")
        } else {
            format!("{min} to {max}")
        };
        let args_word = if max > 1 { "s" } else { "" };
        return Err(crate::error::TemplateError::misc(format!(
            "?{name}(...) expects {cnt_desc} argument{args_word} but has received {}.",
            if n == 0 {
                "none".to_string()
            } else {
                n.to_string()
            }
        )));
    }
    Ok(())
}

/// 求值第 idx 个参数为标量字符串（Java getStringMethodArg）
pub fn arg_string(
    env: &mut Environment,
    args: Option<&[crate::core::Expr]>,
    idx: usize,
) -> Result<String> {
    let e = args
        .and_then(|a| a.get(idx))
        .ok_or_else(|| crate::error::TemplateError::misc("Missing argument"))?;
    let m = crate::core::eval::eval(env, e)?;
    coerce_to_string(env, &m)
}

/// 求值第 idx 个参数为数字（Java getNumberMethodArg）
pub fn arg_number(
    env: &mut Environment,
    args: Option<&[crate::core::Expr]>,
    idx: usize,
) -> Result<TNumber> {
    let e = args
        .and_then(|a| a.get(idx))
        .ok_or_else(|| crate::error::TemplateError::misc("Missing argument"))?;
    crate::core::eval::eval(env, e)?.get_number()
}

/// 目标求值并强制为标量（字符串内建统一入口；Java BuiltInForString.calculateResult）
pub fn target_string(env: &mut Environment, target: &crate::core::Expr) -> Result<String> {
    let m = crate::core::eval::eval(env, target)?;
    coerce_to_string(env, &m)
}

/// 表达式参数个数（`?xxx` 无参形式）
pub fn arg_count(args: Option<&[crate::core::Expr]>) -> usize {
    args.map_or(0, |a| a.len())
}
