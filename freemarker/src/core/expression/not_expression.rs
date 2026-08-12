//! 逻辑非 —— 对应 Java `freemarker.core.NotExpression`
//! （`evalToBoolean` → modelToBoolean 取反）

use crate::core::eval::{eval, model_to_boolean};
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;

/// 逻辑非（对应 NotExpression.java；解析器经 `ExprKind::Not` 承载）
pub struct NotExpression {
    pub target: Expr,
}

impl NotExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: Expr) -> Self {
        NotExpression { target }
    }

    /// 求值（Java `evalToBoolean` → modelToBoolean（classic 兼容见 eval.rs））
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        let m = eval(env, &self.target)?;
        let b = model_to_boolean(env, &m)?;
        Ok(TModel::from_boolean(!b))
    }
}
