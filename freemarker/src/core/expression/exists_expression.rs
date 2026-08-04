//! 存在性 —— 对应 Java `freemarker.core.ExistsExpression`
//! （`_eval` :42-50：求值成功且非 null → TRUE）

use crate::core::eval::eval_lenient;
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;

/// 存在性表达式（对应 ExistsExpression.java；解析器经 `ExprKind::Exists` 承载）
pub struct ExistsExpression {
    pub target: Expr,
}

impl ExistsExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: Expr) -> Self {
        ExistsExpression { target }
    }

    /// 求值（Java `_eval`：目标求值成功且非 null → TRUE）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        let m = eval_lenient(env, &self.target)?;
        Ok(TModel::from_boolean(!m.is_nothing()))
    }
}
