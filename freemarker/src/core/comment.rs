//! 注释 —— 对应 Java `freemarker.core.Comment`
//! （accept：无输出——注释文本不渲染）

use crate::core::exec::ExecOutcome;
use crate::error::Result;

/// `<#-- comment -->` 指令（对应 Comment.java；文本仅在解析期使用）
pub struct Comment {
    /// 注释文本（Java Comment 持有；渲染期无输出）
    #[allow(dead_code)]
    pub text: String,
}

impl Comment {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(text: String) -> Self {
        Comment { text }
    }

    /// 执行（Java accept：无输出）
    pub(crate) fn exec(&self, _env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        Ok(ExecOutcome::Done)
    }
}
