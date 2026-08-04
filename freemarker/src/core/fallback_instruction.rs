//! 回退指令 —— 对应 Java `freemarker.core.FallbackInstruction`
//! （v1 无 XML 节点模型，执行即报错）

use crate::core::exec::ExecOutcome;
use crate::error::{Result, TemplateError};

/// `<#fallback>` 指令（对应 FallbackInstruction.java；无字段）
pub struct FallbackInstruction;

impl FallbackInstruction {
    /// 构造（Java 无参构造器；Rust 侧由解析器产生）
    pub fn new() -> Self {
        FallbackInstruction
    }

    /// 执行（Java accept → 回退到默认节点模板；v1 不支持）
    pub(crate) fn exec(&self, _env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        Err(TemplateError::misc(
            "#fallback needs XML node support (a Java-specific feature).",
        ))
    }
}
