//! 数值字面量 —— 对应 Java `freemarker.core.NumberLiteral`
//! （`_eval` :86-98 → evalToNumber）

use crate::error::Result;
use crate::template::TModel;
use crate::value::TNumber;

/// 数值字面量（对应 NumberLiteral.java；解析器经 `ExprKind::Num` 承载）
pub struct NumberLiteral {
    pub value: TNumber,
}

impl NumberLiteral {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(value: TNumber) -> Self {
        NumberLiteral { value }
    }

    /// 求值（Java `_eval`）
    pub(crate) fn eval(&self, _env: &mut crate::core::Environment) -> Result<TModel> {
        Ok(TModel::from_number(self.value.clone()))
    }
}
