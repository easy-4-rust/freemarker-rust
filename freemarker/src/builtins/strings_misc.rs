//! 字符串杂项内建 —— 对应 Java `freemarker.core.builtins.BuiltInsForStringsMisc.java`
//! （eval_json：JSON 字符串解析为模型；eval/boolean 等其余杂项内建 v1 仍在
//! built_in.rs——本文件按 Java 类承载 eval_json 全量实现）

use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// ?eval_json —— Java evalJsonBI（BuiltInsForStringsMisc.java:116-131）：JSON 字符串
/// 解析为模型；失败消息 = "Failed to \"?eval_json\" string with this error:"
/// + EMBEDDED_MESSAGE 段 + "The failing expression:"（源码拼接，jar 实测格式）。
/// 内嵌消息用 serde_json 原文（Java JSONParser 逐字消息无 golden/parity 场景覆盖——
/// 文档化偏差）
pub fn eval_json(
    env: &mut crate::core::Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = eval(env, target)?;
    let s = m.get_scalar()?;
    match serde_json::from_str::<serde_json::Value>(&s) {
        Ok(v) => Ok(Some(json_value_to_model(&v))),
        Err(e) => Err(TemplateError::misc(format!(
            "Failed to \"?eval_json\" string with this error:\n\n---begin-message---\n{e}\n---end-message---\n\nThe failing expression:"
        ))),
    }
}

/// JSON 值 → 模板模型（Java JSONParser 的 value 构造；数字整值 → Int/Long，
/// 非整值 → Double）
fn json_value_to_model(v: &serde_json::Value) -> TModel {
    match v {
        serde_json::Value::Null => TModel::nothing(),
        serde_json::Value::Bool(b) => TModel::from_boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                TModel::from_number(crate::value::TNumber::from_i64(i))
            } else if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 && f.is_finite() {
                    TModel::from_number(crate::value::TNumber::from_i64(f as i64))
                } else {
                    TModel::from_number(crate::value::TNumber::Double(f))
                }
            } else {
                TModel::nothing()
            }
        }
        serde_json::Value::String(s) => TModel::from_scalar(s.clone()),
        serde_json::Value::Array(arr) => {
            TModel::from_sequence(arr.iter().map(json_value_to_model).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = indexmap::IndexMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_value_to_model(v));
            }
            TModel::from_hash(map)
        }
    }
}
