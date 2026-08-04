//! 加/拼接表达式 —— 对应 Java `freemarker.core.AddConcatExpression`
//! （`_eval` :63-134；AddConcatExpression.java 462-550 的
//! ConcatenatedHash 语义见 eval_add 哈希分支）

use crate::core::arithmetic_engine::{ArithmeticEngine, BigDecimalEngine};
use crate::core::environment::model_to_string;
use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;
use indexmap::IndexMap;

/// 加/拼接表达式（对应 AddConcatExpression.java；解析器经 `ExprKind::Add` 承载）
pub struct AddConcatExpression {
    pub left: Expr,
    pub right: Expr,
}

impl AddConcatExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(left: Expr, right: Expr) -> Self {
        AddConcatExpression { left, right }
    }

    /// 求值（Java `_eval` :63-134）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_add(env, &self.left, &self.right)
    }
}

/// 加法/字符串拼接（Java AddConcatExpression.java:63-134 `_eval`）：
/// 数字+数字 → BigDecimalEngine.add；序列+序列 → 拼接序列；哈希+哈希 → 拼接哈希；
/// 其余 → 字符串拼接（数字 canonical、布尔 boolean_format、标量原样）
pub(crate) fn eval_add(env: &mut crate::core::Environment, a: &Expr, b: &Expr) -> Result<TModel> {
    let l = eval(env, a)?;
    let r = eval(env, b)?;
    if l.is_number() && r.is_number() {
        let engine = BigDecimalEngine::default();
        return Ok(TModel::from_number(
            engine.add(&l.get_number()?, &r.get_number()?)?,
        ));
    }
    if l.is_sequence() && r.is_sequence() {
        // Java ConcatenatedSequence
        let ls = l.get_sequence()?;
        let rs = r.get_sequence()?;
        let n = ls.size()? + rs.size()?;
        let mut v = Vec::with_capacity(n);
        for i in 0..ls.size()? {
            v.push(ls.get(i)?);
        }
        for i in 0..rs.size()? {
            v.push(rs.get(i)?);
        }
        return Ok(TModel::from_sequence(v));
    }
    if l.is_hash() && r.is_hash() {
        // Java ConcatenatedHashEx（AddConcatExpression.java:462-550）：
        // - get(key)：先右后左（右键覆盖左键）；
        // - keys()：先左后右，重复键保留首次出现位置（LinkedHashSet 语义）；
        // - values()：按 keys 顺序取 get 值
        let lh = l.get_hash()?;
        let rh = r.get_hash()?;
        let mut map = IndexMap::new();
        if let Some(ex) = &l.hash_ex {
            for key in ex.keys()? {
                if let Some(v) = lh.get(&key)? {
                    map.entry(key).or_insert(v);
                }
            }
        }
        if let Some(ex) = &r.hash_ex {
            for key in ex.keys()? {
                if let Some(v) = rh.get(&key)? {
                    map.entry(key).or_insert(v);
                }
            }
        }
        // 值取右侧优先（Java ConcatenatedHash.get :470-475）
        for key in map.keys().cloned().collect::<Vec<_>>() {
            if let Some(v) = rh.get(&key)? {
                map.insert(key, v);
            }
        }
        return Ok(TModel::from_hash(map));
    }
    // 字符串拼接（Java EvalUtil.coerceModelToStringOrMarkup）
    let ls = model_to_string(env, &l)?;
    let rs = model_to_string(env, &r)?;
    Ok(TModel::from_scalar(ls + &rs))
}
