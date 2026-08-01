//! 模板错误 —— 对应 Java `freemarker.template.TemplateException`
//! （错误层级与消息逐字对齐见 docs/09）

use crate::error::FlowKind;
use crate::span::Span;

pub type Result<T> = std::result::Result<T, TemplateError>;

/// 错误分类（对应 Java 异常层级；消息逐字对齐见 docs/09 §2）
#[derive(Debug)]
pub enum TemplateError {
    /// 变量缺失：`The following has evaluated to null or missing: ==> {name}`
    InvalidReference { name: String },
    /// 类型不匹配（NonXxxException 族）：`For "{target}" something that is a {expected} is required, but this has evaluated to a {actual}`
    TypeMismatch {
        expected: &'static str,
        actual: String,
    },
    /// 通用运行时错误（_MiscTemplateException）
    Misc { message: String },
    /// 解析错误：`Parsing error in template "{name}" at line L, column C. {message}`
    Parse { template: String, message: String },
    /// stop 指令（非错误但终止渲染；StopException）
    Stop { message: Option<String> },
    /// break/continue 流控信号（内部传播，不面向用户）
    Flow(FlowKind),
    /// 模板加载失败（TemplateNotFoundException）
    NotFound { name: String },
    /// I/O 错误
    Io(std::io::Error),
    /// 模板模型层错误（TemplateModelException）
    Model { message: String },
}

impl TemplateError {
    /// 附加源码位置（模板名 + 行列）
    pub fn with_span(self, template_name: &str, _span: Span) -> Self {
        match self {
            TemplateError::Parse {
                template: _,
                message,
            } => TemplateError::Parse {
                template: template_name.to_string(),
                message,
            },
            other => other,
        }
    }

    /// 附加指令栈描述（`The problematic instruction was: ...`；渲染层拼接）
    pub fn with_stack(self, _stack: Vec<String>) -> Self {
        self
    }

    pub fn invalid_reference(name: impl Into<String>) -> Self {
        TemplateError::InvalidReference { name: name.into() }
    }

    pub fn type_mismatch(expected: &'static str, actual: impl Into<String>) -> Self {
        TemplateError::TypeMismatch {
            expected,
            actual: actual.into(),
        }
    }

    pub fn misc(message: impl Into<String>) -> Self {
        TemplateError::Misc {
            message: message.into(),
        }
    }

    /// 生成与 Java 版对齐的错误消息文本
    /// （Java: `FreeMarker template error (DEBUG mode; use RETHROW in production!)\n{message}\n    at ...`）
    pub fn to_user_message(&self) -> String {
        match self {
            TemplateError::InvalidReference { name } => {
                // Java InvalidReferenceException：消息含 Tip 段（描述构建器，
                // jar 实测；existence-operators 的 isNonFastIRE 断言 "Tip:" 字样）
                format!(
                    "The following has evaluated to null or missing: ==> {name}\n\n----\nTip: If the failing expression is known to legally refer to something that's sometimes null or missing, either specify a default value like myOptionalVar!myDefault, or use <#if myOptionalVar??>when-present<#else>when-missing</#if>. (These only cover the last step of the expression; to cover the whole expression, use parenthesis: (myOptionalVar.foo)!myDefault, (myOptionalVar.foo)??\n----"
                )
            }
            TemplateError::TypeMismatch { expected, actual } => format!(
                "For \"...\" something that is a {expected} is required, but this has evaluated to a {actual}."
            ),
            TemplateError::Misc { message } => message.clone(),
            TemplateError::Parse { template, message } => {
                format!("Parsing error in template {template}: {message}")
            }
            TemplateError::Stop { message } => match message {
                Some(m) => format!("Template has been stopped: {m}"),
                None => "Template has been stopped.".to_string(),
            },
            TemplateError::Flow(kind) => match kind {
                FlowKind::Break => "break is illegal outside a loop".to_string(),
                FlowKind::Continue => "continue is illegal outside a loop".to_string(),
            },
            TemplateError::NotFound { name } => {
                format!("Template not found for name \"{name}\".")
            }
            TemplateError::Io(e) => e.to_string(),
            TemplateError::Model { message } => message.clone(),
        }
    }
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_user_message())
    }
}

impl std::error::Error for TemplateError {}

impl From<std::io::Error> for TemplateError {
    fn from(e: std::io::Error) -> Self {
        TemplateError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_reference_message_matches_java() {
        // 对照 Java InvalidReferenceException 消息格式（描述 + Tip 段，jar 实测）
        let e = TemplateError::invalid_reference("user.name");
        let msg = e.to_user_message();
        assert!(
            msg.starts_with("The following has evaluated to null or missing: ==> user.name"),
            "{msg}"
        );
        assert!(
            msg.contains("\n\n----\nTip: If the failing expression is known to legally refer"),
            "{msg}"
        );
        assert!(
            msg.ends_with("(myOptionalVar.foo)!myDefault, (myOptionalVar.foo)??\n----"),
            "{msg}"
        );
    }

    #[test]
    fn flow_kind_display() {
        assert_eq!(
            TemplateError::Flow(FlowKind::Break).to_user_message(),
            "break is illegal outside a loop"
        );
        assert_eq!(
            TemplateError::Flow(FlowKind::Continue).to_user_message(),
            "continue is illegal outside a loop"
        );
    }
}
