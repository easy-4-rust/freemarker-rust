//! 对应 Java `freemarker.core` 包：解析器产物、渲染引擎、算术引擎、设置项

pub(crate) mod _misc_template_exception;
mod arithmetic_engine;
mod assignment;
mod assignment_instruction;
mod attempt_block;
mod auto_esc_block;
mod block_assignment;
mod body_instruction;
mod break_instruction;
mod break_or_continue_exception;
pub(crate) mod built_ins_for_callables;
pub(crate) mod built_ins_for_dates;
pub(crate) mod built_ins_for_existence_handling;
pub(crate) mod built_ins_for_hashes;
pub(crate) mod built_ins_for_loop_variables;
pub(crate) mod built_ins_for_markup_outputs;
pub(crate) mod built_ins_for_multiple_types;
pub(crate) mod built_ins_for_node;
pub(crate) mod built_ins_for_numbers;
pub(crate) mod built_ins_for_sequences;
pub(crate) mod built_ins_for_strings_basic;
pub mod built_ins_for_strings_encoding;
pub(crate) mod built_ins_for_strings_misc;
pub(crate) mod built_ins_for_strings_regexp;
pub(crate) mod built_ins_with_lazy_conditionals;
mod combined_markup_output_format;
mod comment;
mod common_markup_output_format;
mod compressed_block;
mod configurable;
mod continue_instruction;
mod css_output_format;
mod dollar_variable;
pub(crate) mod environment;
mod escape_block;
pub(crate) mod eval;
pub(crate) mod eval_util;
mod exec;
mod expression;
mod fallback_instruction;
mod flush_instruction;
mod ftl_header;
mod get_optional_template_method;
mod global_assignment;
mod hash_literal;
mod html_output_format;
mod if_block;
mod include;
pub(crate) mod invalid_reference_exception;
mod items;
mod iterator_block;
mod javascript_output_format;
mod json_output_format;
mod library_load;
mod local_assignment;
#[path = "macro.rs"]
mod r#macro;
mod macro_def;
mod markup_output_format;
pub(crate) mod misc_template_exception;
mod no_auto_esc_block;
mod no_escape_block;
mod non_boolean_exception;
mod non_extended_hash_exception;
mod non_hash_exception;
mod non_listable_right_unbounded_range_model_exception;
mod non_numerical_exception;
mod non_sequence_or_collection_exception;
mod non_string_exception;
mod non_string_or_template_output_exception;
mod on;
mod output_format;
mod output_format_block;
mod parse_exception;
mod plain_text_output_format;
mod property_setting;
mod range_model;
mod recurse_node;
mod return_exception;
mod return_instruction;
mod rtf_output_format;
mod sep;
mod stop_exception;
mod stop_instruction;
mod switch_block;
mod template_class_resolver;
mod template_configuration;
mod template_element;
pub(crate) mod template_exception;
mod template_markup_output_model;
mod template_model_exception;
mod template_not_found_exception;
mod template_output_model;
mod template_plain_output_model;
mod text_block;
mod transform_block;
mod trim_instruction;
mod undefined_output_format;
pub(crate) mod unexpected_type_exception;
mod unified_call;
mod visit_node;
mod xhtml_output_format;
mod xml_output_format;

pub use arithmetic_engine::{ArithmeticEngine, BigDecimalEngine};
pub use combined_markup_output_format::{CombinedMarkupOutputFormat, CombinedMarkupOutputModel};
pub use configurable::{canonical_setting_key, java_time_zone_id, Settings, TzSetting};
pub use environment::{render, Environment, MacroValue, Namespace};
pub use eval::{compare_models, eval, CmpOp};
pub use eval_util::{
    arg_count, arg_number, arg_string, check_arg_count, coerce_to_string, models_equal,
    target_string,
};
pub use exec::{exec, ExecOutcome};
pub use expression::{
    AddConcatExpression, AndExpression, BuiltinVar, Expr, ExprKind, OrExpression, RangeKind,
    StrPart,
};
pub use macro_def::{MacroDef, MacroParam};
pub use output_format::{
    escape_markup, mime_type, parse_combined_markup_format, AutoEscaping, OutputFormatKind,
};
pub use range_model::RangeSpec;
pub use template_class_resolver::{NewBuiltinClassResolver, OptInClassResolver};
pub use template_configuration::TemplateConfiguration;
pub use template_element::{AssignOp, CallTarget, CaseDef, Element, ElementKind};
