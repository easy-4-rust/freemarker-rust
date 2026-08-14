//! 标识符 —— 对应 Java `freemarker.core.Identifier`
//! （`_eval` :48-50：env.getVariable(name)）

use crate::error::Result;
use crate::template::TModel;

/// 标识符（变量引用；对应 Identifier.java；解析器经 `ExprKind::Ident` 承载）
pub struct Identifier {
    pub name: String,
}

impl Identifier {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(name: String) -> Self {
        Identifier { name }
    }

    /// 求值（Java `_eval` → Environment.getVariable）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        env.get_variable(&self.name)
    }
}
