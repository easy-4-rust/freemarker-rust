//! 返回信号 —— 对应 Java `freemarker.core.ReturnException`
//! （`<#return>` 的内部控制流信号，宏/函数帧捕获；Rust 侧为
//! `ExecOutcome::ReturnValue`，不构成用户可见错误——本文件为 Java 类对应锚点）

use crate::error::TemplateError;

/// Java ReturnException 不是错误：Rust 侧承载为 ExecOutcome::ReturnValue。
/// 此构造器仅在需要按错误路径传播时使用（v1 无调用点）。
#[allow(dead_code)]
pub(crate) fn new_signal() -> TemplateError {
    TemplateError::Misc {
        message: "<#return> used outside of a macro or function".to_string(),
    }
}
