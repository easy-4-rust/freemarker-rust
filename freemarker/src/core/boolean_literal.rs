//! 布尔字面量 —— 对应 Java `freemarker.core.BooleanLiteral`
//! （`_eval` :50-57 → evalToBoolean）

use crate::error::Result;
use crate::template::TModel;

/// 布尔字面量（对应 BooleanLiteral.java；解析器经 `ExprKind::Bool` 承载）
pub struct BooleanLiteral {
    pub value: bool,
}

impl BooleanLiteral {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(value: bool) -> Self {
        BooleanLiteral { value }
    }

    /// 求值（Java `_eval`）
    pub(crate) fn eval(&self, _env: &mut crate::core::Environment) -> Result<TModel> {
        Ok(TModel::from_boolean(self.value))
    }
}
