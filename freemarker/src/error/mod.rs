//! 错误体系（对应 Java `freemarker.template.TemplateException` 层级）

mod _misc_template_exception;
mod break_or_continue_exception;
mod error_ctx;
mod flow_kind;
mod invalid_reference_exception;
mod misc_template_exception;
mod non_boolean_exception;
mod non_extended_hash_exception;
mod non_hash_exception;
mod non_listable_right_unbounded_range_model_exception;
mod non_numerical_exception;
mod non_sequence_or_collection_exception;
mod non_string_exception;
mod non_string_or_template_output_exception;
mod parse_exception;
mod return_exception;
mod stop_exception;
mod template_error;
mod template_exception;
mod template_model_exception;
mod template_not_found_exception;
mod unexpected_type_exception;

pub use error_ctx::{ErrorCtx, StackFrame};
pub use flow_kind::FlowKind;
pub use template_error::{Result, TemplateError};
