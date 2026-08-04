//! 宏定义指令 —— 对应 Java `freemarker.core.Macro`
//! （accept :154-156 → Environment.visitMacroDef :1164-1167：解析期已提取到
//! Template.macros；执行期将定义加入当前命名空间）

use crate::core::exec::ExecOutcome;
use crate::core::MacroDef;
use crate::error::Result;

/// `<#macro>` / `<#function>` 指令（对应 Macro.java）
pub struct Macro {
    pub def: MacroDef,
}

impl Macro {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(def: MacroDef) -> Self {
        Macro { def }
    }

    /// 执行（Java accept → visitMacroDef）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        env.register_macro_def(&self.def);
        Ok(ExecOutcome::Done)
    }
}
