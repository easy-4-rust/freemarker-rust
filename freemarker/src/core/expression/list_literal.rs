//! 列表字面量 —— 对应 Java `freemarker.core.ListLiteral`
//! （`_eval` :59-69：逐元素求值 → SimpleSequence）

use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;

/// 列表字面量（对应 ListLiteral.java；解析器经 `ExprKind::ListLit` 承载）
pub struct ListLiteral {
    pub items: Vec<Expr>,
}

impl ListLiteral {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(items: Vec<Expr>) -> Self {
        ListLiteral { items }
    }

    /// 求值（Java `_eval`：逐元素求值 → SimpleSequence）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        let mut v = Vec::with_capacity(self.items.len());
        for i in &self.items {
            v.push(eval(env, i)?);
        }
        Ok(TModel::from_sequence(v))
    }
}
