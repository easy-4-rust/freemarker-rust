//! 一元正负号 —— 对应 Java `freemarker.core.UnaryPlusMinusExpression`
//! （`_eval` :42（TYPE_MINUS → ArithmeticEngine.negate）；TYPE_PLUS 恒等返回）
//! v1 仅实现负号（ExprKind::UnaryMinus）

use crate::core::arithmetic_engine::{ArithmeticEngine, BigDecimalEngine};
use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 一元负号（对应 UnaryPlusMinusExpression.java；解析器经 `ExprKind::UnaryMinus` 承载）
pub struct UnaryPlusMinusExpression {
    pub target: Expr,
}

impl UnaryPlusMinusExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: Expr) -> Self {
        UnaryPlusMinusExpression { target }
    }

    /// 求值（Java `_eval` TYPE_MINUS 分支）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        // Java UnaryPlusMinusExpression.java:42 _eval（TYPE_MINUS → ArithmeticEngine.negate）；
        // 操作数 null → modelToNumber → NonNumericalException（消息同 InvalidReference）
        let m = eval(env, &self.target)?;
        if m.is_nothing() {
            return Err(TemplateError::invalid_reference(
                crate::core::environment::expr_desc(&self.target),
            ));
        }
        let n = m.get_number()?;
        let engine = BigDecimalEngine::default();
        Ok(TModel::from_number(engine.negate(&n)?))
    }
}
