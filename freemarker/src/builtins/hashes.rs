//! 哈希内建 —— 对应 Java `freemarker.core.builtins.BuiltInsForHashes.java`
//! （keys/values：TemplateHashModelEx 的键/值列表，LinkedHashMap 插入序）

use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// ?keys —— Java keysBI（BuiltInsForHashes.java）：扩展哈希键列表，插入序
pub fn keys(
    env: &mut crate::core::Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = eval(env, target)?;
    let h = m.hash_ex.clone().ok_or_else(|| {
        TemplateError::misc(format!(
            "?keys is not applicable to a {} value",
            m.type_name
        ))
    })?;
    // Java BuiltInsForHashes：SimpleHash(LinkedHashMap) 插入序
    Ok(Some(TModel::from_sequence(
        h.keys()?.into_iter().map(TModel::from_scalar).collect(),
    )))
}

/// ?values —— Java valuesBI（BuiltInsForHashes.java）：按插入序取值
/// （entries() 承载重复键模型的原始键值对——legacy HashLiteral 的 valueList）
pub fn values(
    env: &mut crate::core::Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = eval(env, target)?;
    let h = m.hash_ex.clone().ok_or_else(|| {
        TemplateError::misc(format!(
            "?values is not applicable to a {} value",
            m.type_name
        ))
    })?;
    // Java BuiltInsForHashes：按插入序取值（entries() 承载重复键模型的
    // 原始键值对——legacy HashLiteral 的 valueList，HashLiteral.java:150）
    let mut v = Vec::new();
    for (_, value) in h.entries()? {
        v.push(value);
    }
    Ok(Some(TModel::from_sequence(v)))
}
