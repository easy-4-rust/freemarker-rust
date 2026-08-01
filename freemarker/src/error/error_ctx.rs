//! 错误上下文 —— 对应 Java `freemarker.core._ErrorDescriptionBuilder`
//! （模板名 + 行列 + 指令栈；消息结构见 docs/09 §2）

use crate::span::Span;

/// 错误上下文（对应 `_ErrorDescriptionBuilder` 输出中的位置与指令栈）
#[derive(Debug, Clone, Default)]
pub struct ErrorCtx {
    pub template_name: Option<String>,
    pub span: Span,
    pub instruction_stack: Vec<String>,
}
