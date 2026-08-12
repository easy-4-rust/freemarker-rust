//! 模板异常根 —— 对应 Java `freemarker.template.TemplateException`
//! （错误层级根类；Rust 侧为 `TemplateError` 枚举，消息渲染见
//! template_error.rs::to_user_message；附加指令栈见 with_stack）

use crate::error::error_ctx::render_ftl_stack_section;
use crate::error::{ErrorCtx, TemplateError};

/// 附加指令栈（`----\nFTL stack trace ...` 段；渲染层 attach 时调用，
/// 只附加一次——消息已含 "FTL stack trace" 则跳过）
pub(crate) fn with_stack(
    mut e: TemplateError,
    stack: Vec<crate::error::StackFrame>,
) -> TemplateError {
    let section = render_ftl_stack_section(&stack);
    let Some(section) = section else {
        return e;
    };
    match &mut e {
        TemplateError::InvalidReference { ctx, .. } | TemplateError::TypeMismatch { ctx, .. } => {
            if ctx.instruction_stack.is_empty() {
                ctx.instruction_stack = stack;
            }
        }
        TemplateError::Misc { message }
        | TemplateError::Parse { message, .. }
        | TemplateError::Model { message } => {
            if !message.contains("FTL stack trace") {
                message.push_str(&section);
            }
        }
        TemplateError::Stop { message } => {
            let msg = message.get_or_insert_with(String::new);
            if !msg.contains("FTL stack trace") {
                // Java StopException 无消息 → 消息体 "[No error description was
                // available.]"（含栈时同样前置该文本，jar 实测 stop_plain）
                if msg.is_empty() {
                    msg.push_str("[No error description was available.]");
                }
                msg.push_str(&section);
            }
        }
        TemplateError::NotFound { .. } => {}
        TemplateError::Flow(_) | TemplateError::Io(_) => {}
    }
    e
}

// 供 error_ctx 使用的 ErrorCtx 类型锚（Java TemplateException 的
// _MessageUtil/ErrorDescriptionBuilder 对应物）
#[allow(unused_imports)]
use ErrorCtx as _ErrorDescriptionBuilderAnchor;
