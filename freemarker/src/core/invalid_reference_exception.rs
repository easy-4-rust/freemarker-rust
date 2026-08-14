//! 变量缺失异常 —— 对应 Java `freemarker.core.InvalidReferenceException`
//! （`The following has evaluated to null or missing:\n==> {name}` + Tip 段；
//! getInstance(blamed, env) :110-158 的 Tip 语义见 with_dot_tip）

use crate::error::{ErrorCtx, TemplateError};
use crate::span::Span;

/// Java `InvalidReferenceException.getInstance(blamed, env)` 的 Rust 入口
/// （无位置版本；blamed = 失败表达式描述）
pub(crate) fn new_instance(blamed: impl Into<String>) -> TemplateError {
    TemplateError::InvalidReference {
        name: blamed.into(),
        ctx: Box::new(ErrorCtx::default()),
    }
}

/// 带 blame 表达式位置的版本（Java 异常构造时 blamed.getStartLocation；
/// 渲染层未提供位置时以元素位置回退）
pub(crate) fn new_instance_at(blamed: impl Into<String>, span: Span) -> TemplateError {
    TemplateError::InvalidReference {
        name: blamed.into(),
        ctx: Box::new(ErrorCtx {
            span,
            ..ErrorCtx::default()
        }),
    }
}

/// 附加点链缺失 Tip（Java Dot._eval 的 `newInvalidReferenceException`：
/// "It's the step after the last dot that caused this error, not those before it."）
pub(crate) fn with_dot_tip(mut e: TemplateError) -> TemplateError {
    if let TemplateError::InvalidReference { ctx, .. } = &mut e {
        ctx.extra_tip = Some(
            "It's the step after the last dot that caused this error, not those before it."
                .to_string(),
        );
    }
    e
}

/// Java InvalidReferenceException 的提示段（InvalidReferenceException.java，
/// `Tip:` 字面，jar 实测逐字）
pub(crate) const INVALID_REFERENCE_TIP: &str = "If the failing expression is known to legally refer to something that's sometimes null or missing, either specify a default value like myOptionalVar!myDefault, or use <#if myOptionalVar??>when-present<#else>when-missing</#if>. (These only cover the last step of the expression; to cover the whole expression, use parenthesis: (myOptionalVar.foo)!myDefault, (myOptionalVar.foo)??";
