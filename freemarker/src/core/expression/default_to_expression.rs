//! 缺失默认 —— 对应 Java `freemarker.core.DefaultToExpression`
//! （`_eval` :84-90：目标 null/缺失 → 默认值；无默认值 → 空串模型）

use crate::core::eval::{eval, eval_lenient};
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;

/// 缺失默认表达式（对应 DefaultToExpression.java；解析器经
/// `ExprKind::Default` 承载，default=None 为 `expr!`）
pub struct DefaultToExpression {
    pub target: Expr,
    pub default: Option<Expr>,
}

impl DefaultToExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: Expr, default: Option<Expr>) -> Self {
        DefaultToExpression { target, default }
    }

    /// 求值（Java `_eval`）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        // Java DefaultToExpression._eval：目标 null/缺失 → 默认值；无默认值 → 空串模型
        let m = eval_lenient(env, &self.target)?;
        if !m.is_nothing() {
            return Ok(m);
        }
        match &self.default {
            Some(d) => eval(env, d),
            None => Ok(TModel::from_scalar(String::new())),
        }
    }
}
