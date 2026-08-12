//! 存在性 null 转换内建 —— 对应 Java `BuiltInsForExistenceHandling.java`
//! （blank_to_null/empty_to_null/trim_to_null；default/exists/if_exists/has_content 在 eval.rs）
//!
//! 语义要点（Java 对照，StringUtil 家族）：
//! - `?empty_to_null` → emptyToNull：空串 → null；
//! - `?blank_to_null` → blankToNull：全空白（java_trim 后为空）→ null；
//! - `?trim_to_null` → trimToNull：java_trim 后为空 → null，否则返回 trim 结果。

use crate::core::{Environment, Expr};
use crate::error::Result;
use crate::template::utility::java_trim;
use crate::template::TModel;

/// 目标求值为标量（Java BuiltInForString 语义；数字/布尔强制转换）。
/// 注意：`*_to_null` 是 Java `BuiltInsForExistenceHandling.ExistenceBuiltIn`
/// （evalMaybeNonexistentTarget）：目标缺失（仅括号/标识符表达式捕获
/// InvalidReferenceException）→ 结果为 null/缺失（`v?blank_to_null!'-'` → '-'）
fn target_string(env: &mut Environment, target: &Expr) -> Result<Option<String>> {
    let m = crate::core::eval::eval_lenient(env, target)?;
    if m.is_nothing() {
        // Java evalMaybeNonexistentTarget → null → *_to_null 返回 null
        return Ok(None);
    }
    crate::core::eval_util::coerce_to_string(env, &m).map(Some)
}

/// ?empty_to_null —— Java empty_to_nullBI
pub fn empty_to_null(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    match target_string(env, target)? {
        None => Ok(Some(TModel::nothing())),
        Some(s) => Ok(Some(if s.is_empty() {
            TModel::nothing()
        } else {
            TModel::from_scalar(s)
        })),
    }
}

/// ?blank_to_null —— Java blank_to_nullBI
pub fn blank_to_null(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    match target_string(env, target)? {
        None => Ok(Some(TModel::nothing())),
        Some(s) => Ok(Some(if java_trim(&s).is_empty() {
            TModel::nothing()
        } else {
            TModel::from_scalar(s)
        })),
    }
}

/// ?trim_to_null —— Java trim_to_nullBI
pub fn trim_to_null(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    match target_string(env, target)? {
        None => Ok(Some(TModel::nothing())),
        Some(s) => {
            let t = java_trim(&s).to_string();
            Ok(Some(if t.is_empty() {
                TModel::nothing()
            } else {
                TModel::from_scalar(t)
            }))
        }
    }
}
