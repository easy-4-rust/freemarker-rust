//! 哈希字面量 —— 对应 Java `freemarker.core.HashLiteral`
//! （`_eval` :99-115；ICI < 2.3.21 的 legacy SequenceHash 分支 :116-151
//! 用的 LegacyHashLiteral 模型在 core/hash_literal.rs）

use crate::core::environment::model_to_string;
use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;
use indexmap::IndexMap;
use std::rc::Rc;

/// 哈希字面量（对应 HashLiteral.java；解析器经 `ExprKind::HashLit` 承载）
pub struct HashLiteral {
    pub pairs: Vec<(Expr, Expr)>,
}

impl HashLiteral {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(pairs: Vec<(Expr, Expr)>) -> Self {
        HashLiteral { pairs }
    }

    /// 求值（Java `_eval` :99-151）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        // Java HashLiteral → SimpleHash(LinkedHashMap)：插入序即键序
        let mut map = IndexMap::new();
        let mut raw: Vec<(String, TModel)> = Vec::new();
        for (k, v) in &self.pairs {
            // Java HashLiteral：键按 EvalUtil 强制转字符串（数字 "123"、布尔按 boolean_format）
            let km = eval(env, k)?;
            let key = model_to_string(env, &km)?;
            let value = eval(env, v)?;
            map.insert(key.clone(), value.clone());
            raw.push((key, value));
        }
        // Java HashLiteral.java:116-151：ICI ≥ 2.3.21 → LinkedHashMap（重复键覆盖）；
        // ICI < 2.3.21 → legacy 分支（SequenceHash 保留重复键——?keys/?values/
        // `#list h as k, v` 输出全部字面量对，h[key] 仍取最后值）
        if env.settings.incompatible_improvements.to_int() < 2_003_021 {
            let h = Rc::new(crate::core::hash_literal::LegacyHashLiteral::new(raw));
            return Ok(TModel {
                hash: Some(h.clone()),
                hash_ex: Some(h),
                type_name: "hash",
                kind: crate::template::ModelKind::Hash,
                ..TModel::nothing()
            });
        }
        Ok(TModel::from_hash(map))
    }
}
