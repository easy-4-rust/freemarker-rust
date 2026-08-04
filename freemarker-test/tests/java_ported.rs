//! Java 测试逻辑的 Rust 1:1 实现（freemarker-core/src/test + freemarker-jython25/src/test）
//!
//! 每个 Java 测试类对应一个模块文件（tests/java_ported/<类名>.rs），测试函数与
//! Java 测试方法同名、同断言、错误消息逐字对齐。共享辅助见 util.rs
//! （对应 freemarker-test-utils 的 TemplateTest 基类）。

#[path = "java_ported/absolute_template_name_bi_test.rs"]
mod absolute_template_name_bi_test;
#[path = "java_ported/actual_naming_convention_test.rs"]
mod actual_naming_convention_test;
#[path = "java_ported/actual_tag_syntax_test.rs"]
mod actual_tag_syntax_test;
#[path = "java_ported/args_special_variable_test.rs"]
mod args_special_variable_test;
#[path = "java_ported/arithmetic_engine_test.rs"]
mod arithmetic_engine_test;
#[path = "java_ported/ast_based_error_messages.rs"]
mod ast_based_error_messages;
#[path = "java_ported/ast_test.rs"]
mod ast_test;
#[path = "java_ported/attempt_logging_test.rs"]
mod attempt_logging_test;
#[path = "java_ported/boolean_format_environment_caching_test.rs"]
mod boolean_format_environment_caching_test;
#[path = "java_ported/break_and_continue_placement.rs"]
mod break_and_continue_placement;
#[path = "java_ported/c_and_cn_built_in_test.rs"]
mod c_and_cn_built_in_test;
#[path = "java_ported/c_format_template_test.rs"]
mod c_format_template_test;
#[path = "java_ported/c_template_number_format_test.rs"]
mod c_template_number_format_test;
#[path = "java_ported/caller_template_name_test.rs"]
mod caller_template_name_test;
#[path = "java_ported/camel_case.rs"]
mod camel_case;
#[path = "java_ported/canonical_form.rs"]
mod canonical_form;
#[path = "java_ported/capturing_assignment_test.rs"]
mod capturing_assignment_test;
#[path = "java_ported/classic_compatible_test.rs"]
mod classic_compatible_test;
#[path = "java_ported/coercion_to_textual_test.rs"]
mod coercion_to_textual_test;
#[path = "java_ported/combined_markup_output_format_test.rs"]
mod combined_markup_output_format_test;
#[path = "java_ported/concatenated_sequence_test.rs"]
mod concatenated_sequence_test;
#[path = "java_ported/configurable_test.rs"]
mod configurable_test;
#[path = "java_ported/configuration_test.rs"]
mod configuration_test;
#[path = "java_ported/constants_test.rs"]
mod constants_test;
#[path = "java_ported/core_locale_utils_test.rs"]
mod core_locale_utils_test;
#[path = "java_ported/custom_attribute_test.rs"]
mod custom_attribute_test;
#[path = "java_ported/date_format_test.rs"]
mod date_format_test;
#[path = "java_ported/date_util_test.rs"]
mod date_util_test;
#[path = "java_ported/deep_unwrap_test.rs"]
mod deep_unwrap_test;
#[path = "java_ported/default_truncate_builtin_algorithm_test.rs"]
mod default_truncate_builtin_algorithm_test;
#[path = "java_ported/directive_call_place_test.rs"]
mod directive_call_place_test;
#[path = "java_ported/encoding_override.rs"]
mod encoding_override;
#[path = "java_ported/end_tag_syntax.rs"]
mod end_tag_syntax;
#[path = "java_ported/environment_custom_state_test.rs"]
mod environment_custom_state_test;
#[path = "java_ported/environment_get_template_variants_test.rs"]
mod environment_get_template_variants_test;
#[path = "java_ported/error_message_parity.rs"]
mod error_message_parity;
#[path = "java_ported/eval_json_built_in_test.rs"]
mod eval_json_built_in_test;
#[path = "java_ported/exception_test.rs"]
mod exception_test;
#[path = "java_ported/extended_decimal_format_test.rs"]
mod extended_decimal_format_test;
#[path = "java_ported/file_template_loader_test.rs"]
mod file_template_loader_test;
#[path = "java_ported/filter_bi_test.rs"]
mod filter_bi_test;
#[path = "java_ported/get_optional_template_method_test.rs"]
mod get_optional_template_method_test;
#[path = "java_ported/get_source_test.rs"]
mod get_source_test;
#[path = "java_ported/header_parsing.rs"]
mod header_parsing;
#[path = "java_ported/html_output_format_test.rs"]
mod html_output_format_test;
#[path = "java_ported/include_and_import_test.rs"]
mod include_and_import_test;
#[path = "java_ported/include_and_import_configurable_layers_test.rs"]
mod include_and_import_configurable_layers_test;
#[path = "java_ported/incude_from_nameless_test.rs"]
mod incude_from_nameless_test;
#[path = "java_ported/interpolation_syntax.rs"]
mod interpolation_syntax;
#[path = "java_ported/interpret_and_eval_template_name_test.rs"]
mod interpret_and_eval_template_name_test;
#[path = "java_ported/interpret_setting_inheritance_test.rs"]
mod interpret_setting_inheritance_test;
#[path = "java_ported/iterator_issues_test.rs"]
mod iterator_issues_test;
#[path = "java_ported/javacc_exception_as_eof_fix_test.rs"]
mod javacc_exception_as_eof_fix_test;
#[path = "java_ported/json_parser_test.rs"]
mod json_parser_test;
#[path = "java_ported/lambda_parsing.rs"]
mod lambda_parsing;
#[path = "java_ported/lamda_and_escape_test.rs"]
mod lamda_and_escape_test;
#[path = "java_ported/lazily_generated_collection_test.rs"]
mod lazily_generated_collection_test;
#[path = "java_ported/legacy_fm_parser_constructors_test.rs"]
mod legacy_fm_parser_constructors_test;
#[path = "java_ported/list_break_continue.rs"]
mod list_break_continue;
#[path = "java_ported/list_errors_test.rs"]
mod list_errors_test;
#[path = "java_ported/list_with_stream_like_builtins_test.rs"]
mod list_with_stream_like_builtins_test;
#[path = "java_ported/map_bi_test.rs"]
mod map_bi_test;
#[path = "java_ported/min_max_bi_test.rs"]
mod min_max_bi_test;
#[path = "java_ported/misc_error_messages.rs"]
mod misc_error_messages;
#[path = "java_ported/mistakenly_public_import_apis_test.rs"]
mod mistakenly_public_import_apis_test;
#[path = "java_ported/mistakenly_public_macro_apis_test.rs"]
mod mistakenly_public_macro_apis_test;
#[path = "java_ported/multi_template_loader_test.rs"]
mod multi_template_loader_test;
#[path = "java_ported/null_configuration_test.rs"]
mod null_configuration_test;
#[path = "java_ported/null_transparency_test.rs"]
mod null_transparency_test;
#[path = "java_ported/number_bi_test.rs"]
mod number_bi_test;
#[path = "java_ported/number_format_test.rs"]
mod number_format_test;
#[path = "java_ported/number_util_test.rs"]
mod number_util_test;
#[path = "java_ported/object_builder_settings_test.rs"]
mod object_builder_settings_test;
#[path = "java_ported/opt_in_template_class_resolver_test.rs"]
mod opt_in_template_class_resolver_test;
#[path = "java_ported/output_format_test.rs"]
mod output_format_test;
#[path = "java_ported/parse_time_parameter_bi_error_messages.rs"]
mod parse_time_parameter_bi_error_messages;
#[path = "java_ported/parsing_error_messages.rs"]
mod parsing_error_messages;
#[path = "java_ported/rtf_output_format_test.rs"]
mod rtf_output_format_test;
#[path = "java_ported/runtime_environment_reporter_test.rs"]
mod runtime_environment_reporter_test;
#[path = "java_ported/sep_parsing_bug.rs"]
mod sep_parsing_bug;
#[path = "java_ported/sequence_built_in_test.rs"]
mod sequence_built_in_test;
#[path = "java_ported/setting_directive_test.rs"]
mod setting_directive_test;
#[path = "java_ported/simple_object_wrapper_test.rs"]
mod simple_object_wrapper_test;
#[path = "java_ported/special_variable_test.rs"]
mod special_variable_test;
#[path = "java_ported/sql_time_zone_test.rs"]
mod sql_time_zone_test;
#[path = "java_ported/static_object_wrappers_test.rs"]
mod static_object_wrappers_test;
#[path = "java_ported/string_built_in_test.rs"]
mod string_built_in_test;
#[path = "java_ported/string_literal_interpolation_test.rs"]
mod string_literal_interpolation_test;
#[path = "java_ported/string_util_test.rs"]
mod string_util_test;
#[path = "java_ported/switch_test.rs"]
mod switch_test;
#[path = "java_ported/tab_size.rs"]
mod tab_size;
#[path = "java_ported/tag_syntax_variations.rs"]
mod tag_syntax_variations;
#[path = "java_ported/take_while_and_drop_while_bi_test.rs"]
mod take_while_and_drop_while_bi_test;
#[path = "java_ported/templat_get_encoding_test.rs"]
mod templat_get_encoding_test;
#[path = "java_ported/template_cache_test.rs"]
mod template_cache_test;
#[path = "java_ported/template_configuration_factory_test.rs"]
mod template_configuration_factory_test;
#[path = "java_ported/template_configuration_test.rs"]
mod template_configuration_test;
#[path = "java_ported/template_configuration_with_template_cache_test.rs"]
mod template_configuration_with_template_cache_test;
#[path = "java_ported/template_constructors_test.rs"]
mod template_constructors_test;
#[path = "java_ported/template_language_version_test.rs"]
mod template_language_version_test;
#[path = "java_ported/template_lookup_strategy_test.rs"]
mod template_lookup_strategy_test;
#[path = "java_ported/template_model_util_test.rs"]
mod template_model_util_test;
#[path = "java_ported/template_name_format_test.rs"]
mod template_name_format_test;
#[path = "java_ported/template_name_special_variables_test.rs"]
mod template_name_special_variables_test;
#[path = "java_ported/template_processing_tracer_test.rs"]
mod template_processing_tracer_test;
#[path = "java_ported/template_source_matcher_test.rs"]
mod template_source_matcher_test;
#[path = "java_ported/template_transform_model_test.rs"]
mod template_transform_model_test;
#[path = "java_ported/thread_interrupting_support_test.rs"]
mod thread_interrupting_support_test;
#[path = "java_ported/truncate_built_in_test.rs"]
mod truncate_built_in_test;
#[path = "java_ported/type_error_messages.rs"]
mod type_error_messages;
#[path = "java_ported/unchecked_exception_handling_test.rs"]
mod unchecked_exception_handling_test;
#[path = "java_ported/unclosed_comment.rs"]
mod unclosed_comment;
#[path = "java_ported/util.rs"]
pub mod util;
#[path = "java_ported/version_test.rs"]
mod version_test;
#[path = "java_ported/whitespace_stripping.rs"]
mod whitespace_stripping;
#[path = "java_ported/with_args_built_in_test.rs"]
mod with_args_built_in_test;
#[path = "java_ported/xhtml_output_format_test.rs"]
mod xhtml_output_format_test;
#[path = "java_ported/xml_output_format_test.rs"]
mod xml_output_format_test;
