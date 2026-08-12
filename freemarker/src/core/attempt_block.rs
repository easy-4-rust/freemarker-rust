//! 尝试块 —— 对应 Java `freemarker.core.AttemptBlock`
//! （Java :3557-3567 visitAttemptRecover：错误 → recover；`.error` 读取
//! recoveredErrorStack——recover 渲染结束弹出，Environment.java:575-578）

use crate::core::environment::RunSignal;
use crate::core::exec::ExecOutcome;
use crate::core::Element;
use crate::error::{Result, TemplateError};

/// `<#attempt>` / `<#recover>` 块（对应 AttemptBlock.java）
pub struct AttemptBlock {
    pub try_: Vec<Element>,
    pub recover: Vec<Element>,
}

impl AttemptBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(try_: Vec<Element>, recover: Vec<Element>) -> Self {
        AttemptBlock { try_, recover }
    }

    /// 执行（Java visitAttemptRecover :3557-3567）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let captured = env.capture(|env| {
            env.attempt_depth += 1;
            let r = env.run(&self.try_);
            env.attempt_depth -= 1;
            r
        });
        match captured {
            Ok((RunSignal::Completed, text)) => {
                env.emit(&text)?;
                Ok(ExecOutcome::Done)
            }
            // Java：Return/Flow 是 RuntimeException，attempt 不捕获（visitAttemptRecover 只捕 TemplateException）
            Ok((RunSignal::Returned(v), _)) => Ok(ExecOutcome::ReturnValue(v)),
            Err(TemplateError::Flow(k)) => Err(TemplateError::Flow(k)),
            Err(e) => {
                // 错误 → recover（Java :3557-3567）；错误消息压入 recoveredErrorStack 供
                // `.error` 读取，recover 渲染结束弹出（Environment.java:575-578 的 finally
                // ——嵌套 attempt 的内层 recover 结束后 `.error` 恢复为外层错误）
                env.recovered_errors.push(e.to_string());
                let r = match env.run(&self.recover) {
                    Ok(RunSignal::Completed) => Ok(ExecOutcome::Done),
                    Ok(RunSignal::Returned(v)) => Ok(ExecOutcome::ReturnValue(v)),
                    Err(e2) => Err(e2),
                };
                env.recovered_errors.pop();
                r
            }
        }
    }
}
