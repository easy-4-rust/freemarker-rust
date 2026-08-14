//! 逻辑与表达式 —— 对应 Java `freemarker.core.AndExpression`
//! （`_eval`：短路——lho.evalToBoolean && rho.evalToBoolean）

use crate::core::eval::eval;
use crate::core::eval::model_to_boolean;
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;

/// 逻辑与表达式（对应 AndExpression.java；解析器经 `ExprKind::And` 承载）
pub struct AndExpression {
    pub left: Expr,
    pub right: Expr,
}

impl AndExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(left: Expr, right: Expr) -> Self {
        AndExpression { left, right }
    }

    /// 求值（Java `_eval`：短路）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_and(env, &self.left, &self.right)
    }
}

/// 短路与（Java AndExpression：lho.evalToBoolean && rho.evalToBoolean）
pub(crate) fn eval_and(env: &mut crate::core::Environment, a: &Expr, b: &Expr) -> Result<TModel> {
    let lm = eval(env, a)?;
    let l = model_to_boolean(env, &lm)?;
    if !l {
        return Ok(TModel::from_boolean(false));
    }
    let rm = eval(env, b)?;
    let r = model_to_boolean(env, &rm)?;
    Ok(TModel::from_boolean(l && r))
}
