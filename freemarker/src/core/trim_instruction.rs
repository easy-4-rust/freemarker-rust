//! 裁剪指令 —— 对应 Java `freemarker.core.TrimInstruction`
//! （Java 构造器四个布尔参数 (leftTrim, rightTrim) 决定行为：
//! `<#trim>` = (true, true) 块指令；`<#t>`/`<#lt>`/`<#rt>`/`<#nt>` =
//! 解析期标记（渲染期由文本剥离实现，exec 无操作）；v1 枚举变体承载）

use crate::core::environment::RunSignal;
use crate::core::exec::ExecOutcome;
use crate::core::Element;
use crate::error::Result;
use crate::template::utility::java_trim;

/// `<#trim>` 块指令（对应 TrimInstruction.java：(leftTrim, rightTrim) = (true, true)）
pub struct TrimInstruction {
    pub body: Vec<Element>,
}

impl TrimInstruction {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(body: Vec<Element>) -> Self {
        TrimInstruction { body }
    }

    /// 执行（Java accept：块输出捕获 → String.trim 语义 → 写出）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let captured = env.capture(|env| env.run(&self.body))?;
        match captured.0 {
            RunSignal::Returned(v) => Ok(ExecOutcome::ReturnValue(v)),
            RunSignal::Completed => {
                env.emit(java_trim(&captured.1))?;
                Ok(ExecOutcome::Done)
            }
        }
    }
}

/// `<#t>`/`<#nt>`/`<#rt>`/`<#lt>` 解析期标记（Java TrimInstruction 各参数组合；
/// 渲染期由文本剥离实现——exec 无操作）
pub struct TrimMark;

impl TrimMark {
    /// 构造（Rust 侧由解析器产生）
    pub fn new() -> Self {
        TrimMark
    }

    /// 执行（无操作——解析期标记）
    pub(crate) fn exec(&self, _env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        Ok(ExecOutcome::Done)
    }
}
