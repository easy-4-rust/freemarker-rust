//! 括号表达式 —— 对应 Java `freemarker.core.ParentheticalExpression`
//! （`_eval` :49-51：直接求值内层表达式）

use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;

/// 括号表达式（对应 ParentheticalExpression.java；解析器经 `ExprKind::Paren` 承载）
pub struct ParentheticalExpression {
    pub inner: Expr,
}

impl ParentheticalExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(inner: Expr) -> Self {
        ParentheticalExpression { inner }
    }

    /// 求值（Java `_eval`：直接求值内层表达式）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval(env, &self.inner)
    }
}
