//! 设置指令 —— 对应 Java `freemarker.core.PropertySetting`
//! （accept :136：设置 Configuration/Environment 属性）

use crate::core::exec::{exec_setting, ExecOutcome};
use crate::core::Expr;
use crate::error::Result;

/// `<#setting key=value>` 指令（对应 PropertySetting.java）
pub struct Setting {
    pub key: String,
    pub value: Expr,
}

impl Setting {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(key: String, value: Expr) -> Self {
        Setting { key, value }
    }

    /// 执行（Java accept → exec_setting 共享实现）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_setting(env, &self.key, &self.value)
    }
}
