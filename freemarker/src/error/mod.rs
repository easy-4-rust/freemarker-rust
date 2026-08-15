//! 错误体系（对应 Java `freemarker.template.TemplateException` 层级）

pub(crate) mod error_ctx;
mod flow_kind;
mod template_error;

pub use error_ctx::{ErrorCtx, StackFrame};
pub use flow_kind::FlowKind;
pub use template_error::{Result, TemplateError};
