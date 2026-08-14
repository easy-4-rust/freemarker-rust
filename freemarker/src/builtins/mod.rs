//! 内建函数注册表 —— 对应 `freemarker.core.BuiltInsFor*.java`（docs/05，133 个 BI）
//! 分派顺序（eval.rs）：① `builtins::lookup` 注册表（本模块）→ ② eval.rs 内建集 → ③
//! `Unknown built-in: ?{name}`。本模块只注册 eval.rs 内建集未覆盖的名称（避免遮蔽）；
//! `?replace`/`?split`/`?matches` 因需要 flags 参数（Java RegexpHelper），从 eval.rs 迁入本模块。
//!
//! 每个文件对应一个 Java 对象（一文件一对象原则），已归位到 core/：
//! - core/built_ins_for_strings_basic.rs  → BuiltInsForStringsBasic.java
//! - core/built_ins_for_strings_encoding.rs → BuiltInsForStringsEncoding.java
//! - core/built_ins_for_strings_regexp.rs → BuiltInsForStringsRegexp.java
//! - core/built_ins_for_sequences.rs      → BuiltInsForSequences.java
//! - core/built_ins_for_numbers.rs        → BuiltInsForNumbers.java
//! - core/built_ins_for_dates.rs          → BuiltInsForDates.java
//! - core/built_ins_for_multiple_types.rs → BuiltInsForMultipleTypes.java
//! - core/built_ins_with_lazy_conditionals.rs → BuiltInsWithLazyConditionals.java
//! - core/built_ins_for_loop_variables.rs → BuiltInsForLoopVariables.java
//! - core/built_ins_for_existence_handling.rs → BuiltInsForExistenceHandling.java
//! - core/built_ins_for_callables.rs      → BuiltInsForCallables.java
//! 本模块保留 format/iso_date_format/java_date_format（聚合实现，非单一 Java 类镜像）。

pub mod format;
pub mod iso_date_format;
pub mod java_date_format;

use crate::core::{Environment, Expr};
use crate::error::Result;
use crate::template::TModel;

/// 内建函数实现签名：接收目标表达式与参数表达式（**惰性**——不预求值，?then/?switch 等
/// 需要惰性参数），返回 `Option`：`None` 表示"未命中，落入下一级分派"。
pub type BuiltinFn = fn(&mut Environment, &Expr, Option<&[Expr]>) -> Result<Option<TModel>>;

/// 内建函数分派表（名称 → 实现）；未命中返回 None（eval.rs 回落到内建集）
pub fn lookup(name: &str) -> Option<BuiltinFn> {
    match name {
        // ---- 字符串基础（BuiltInsForStringsBasic.java）----
        "capitalize" => Some(crate::core::built_ins_for_strings_basic::capitalize),
        "uncap_first" => Some(crate::core::built_ins_for_strings_basic::uncap_first),
        "c_lower_case" => Some(crate::core::built_ins_for_strings_basic::c_lower_case),
        "c_upper_case" => Some(crate::core::built_ins_for_strings_basic::c_upper_case),
        "chop_linebreak" => Some(crate::core::built_ins_for_strings_basic::chop_linebreak),
        "keep_before" => Some(crate::core::built_ins_for_strings_basic::keep_before),
        "keep_before_last" => Some(crate::core::built_ins_for_strings_basic::keep_before_last),
        "keep_after" => Some(crate::core::built_ins_for_strings_basic::keep_after),
        "keep_after_last" => Some(crate::core::built_ins_for_strings_basic::keep_after_last),
        "remove_beginning" => Some(crate::core::built_ins_for_strings_basic::remove_beginning),
        "remove_ending" => Some(crate::core::built_ins_for_strings_basic::remove_ending),
        "ensure_starts_with" => Some(crate::core::built_ins_for_strings_basic::ensure_starts_with),
        "ensure_ends_with" => Some(crate::core::built_ins_for_strings_basic::ensure_ends_with),
        "left_pad" => Some(crate::core::built_ins_for_strings_basic::left_pad),
        "right_pad" => Some(crate::core::built_ins_for_strings_basic::right_pad),
        "last_index_of" => Some(crate::core::built_ins_for_strings_basic::last_index_of),
        "truncate" => Some(crate::core::built_ins_for_strings_basic::truncate),
        "truncate_w" => Some(crate::core::built_ins_for_strings_basic::truncate_w),
        "truncate_c" => Some(crate::core::built_ins_for_strings_basic::truncate_c),
        "truncate_m" => Some(crate::core::built_ins_for_strings_basic::truncate_m),
        "truncate_w_m" => Some(crate::core::built_ins_for_strings_basic::truncate_w_m),
        "truncate_c_m" => Some(crate::core::built_ins_for_strings_basic::truncate_c_m),
        // ---- 字符串编码（BuiltInsForStringsEncoding.java）----
        "j_string" => Some(crate::core::built_ins_for_strings_encoding::j_string),
        "js_string" => Some(crate::core::built_ins_for_strings_encoding::js_string),
        "json_string" => Some(crate::core::built_ins_for_strings_encoding::json_string),
        "url" => Some(crate::core::built_ins_for_strings_encoding::url),
        "url_path" => Some(crate::core::built_ins_for_strings_encoding::url_path),
        "rtf" => Some(crate::core::built_ins_for_strings_encoding::rtf),
        "xhtml" => Some(crate::core::built_ins_for_strings_encoding::xhtml),
        "esc" => Some(crate::core::built_ins_for_markup_outputs::esc),
        "no_esc" => Some(crate::core::built_ins_for_markup_outputs::no_esc),
        // ---- 正则（BuiltInsForStringsRegexp.java）----
        "matches" => Some(crate::core::built_ins_for_strings_regexp::matches),
        // ---- 字符串杂项（BuiltInsForStringsMisc.java）----
        "eval_json" => Some(crate::core::built_ins_for_strings_misc::eval_json),
        "groups" => Some(crate::core::built_ins_for_strings_regexp::groups),
        "replace" => Some(crate::core::built_ins_for_strings_regexp::replace),
        "replace_re" => Some(crate::core::built_ins_for_strings_regexp::replace),
        "split" => Some(crate::core::built_ins_for_strings_regexp::split),
        // ---- 序列（BuiltInsForSequences.java）----
        "chunk" => Some(crate::core::built_ins_for_sequences::chunk),
        "filter" => Some(crate::core::built_ins_for_sequences::filter),
        "map" => Some(crate::core::built_ins_for_sequences::map),
        "take_while" => Some(crate::core::built_ins_for_sequences::take_while),
        "drop_while" => Some(crate::core::built_ins_for_sequences::drop_while),
        "sort" => Some(crate::core::built_ins_for_sequences::sort),
        "sort_by" => Some(crate::core::built_ins_for_sequences::sort_by),
        "min" => Some(crate::core::built_ins_for_sequences::min),
        "max" => Some(crate::core::built_ins_for_sequences::max),
        "seq_index_of" => Some(crate::core::built_ins_for_sequences::seq_index_of),
        "seq_last_index_of" => Some(crate::core::built_ins_for_sequences::seq_last_index_of),
        "sequence" => Some(crate::core::built_ins_for_sequences::sequence),
        // ---- 哈希（BuiltInsForHashes.java）----
        "keys" => Some(crate::core::built_ins_for_hashes::keys),
        "values" => Some(crate::core::built_ins_for_hashes::values),
        // ---- 数字（BuiltInsForNumbers.java）----
        "abs" => Some(crate::core::built_ins_for_numbers::abs),
        "ceiling" => Some(crate::core::built_ins_for_numbers::ceiling),
        "floor" => Some(crate::core::built_ins_for_numbers::floor),
        "round" => Some(crate::core::built_ins_for_numbers::round),
        "byte" => Some(crate::core::built_ins_for_numbers::byte),
        "short" => Some(crate::core::built_ins_for_numbers::short),
        "is_nan" => Some(crate::core::built_ins_for_numbers::is_nan),
        "is_infinite" => Some(crate::core::built_ins_for_numbers::is_infinite),
        "lower_abc" => Some(crate::core::built_ins_for_numbers::lower_abc),
        "upper_abc" => Some(crate::core::built_ins_for_numbers::upper_abc),
        "number_to_date" => Some(crate::core::built_ins_for_numbers::number_to_date),
        "number_to_time" => Some(crate::core::built_ins_for_numbers::number_to_time),
        "number_to_datetime" => Some(crate::core::built_ins_for_numbers::number_to_datetime),
        // ---- 日期（BuiltInsForDates.java；iso 家族全变体对应 BuiltIn.java:175-234）----
        "date" => Some(crate::core::built_ins_for_dates::date),
        "time" => Some(crate::core::built_ins_for_dates::time),
        "datetime" => Some(crate::core::built_ins_for_dates::datetime),
        "iso" => Some(crate::core::built_ins_for_dates::iso),
        "iso_nz" => Some(crate::core::built_ins_for_dates::iso_nz),
        "iso_fz" => Some(crate::core::built_ins_for_dates::iso_fz),
        "iso_ms" => Some(crate::core::built_ins_for_dates::iso_ms),
        "iso_ms_nz" => Some(crate::core::built_ins_for_dates::iso_ms_nz),
        "iso_m" => Some(crate::core::built_ins_for_dates::iso_m),
        "iso_m_nz" => Some(crate::core::built_ins_for_dates::iso_m_nz),
        "iso_h" => Some(crate::core::built_ins_for_dates::iso_h),
        "iso_h_nz" => Some(crate::core::built_ins_for_dates::iso_h_nz),
        "iso_utc" => Some(crate::core::built_ins_for_dates::iso_utc),
        "iso_utc_fz" => Some(crate::core::built_ins_for_dates::iso_utc_fz),
        "iso_utc_nz" => Some(crate::core::built_ins_for_dates::iso_utc_nz),
        "iso_utc_ms" => Some(crate::core::built_ins_for_dates::iso_utc_ms),
        "iso_utc_ms_nz" => Some(crate::core::built_ins_for_dates::iso_utc_ms_nz),
        "iso_utc_m" => Some(crate::core::built_ins_for_dates::iso_utc_m),
        "iso_utc_m_nz" => Some(crate::core::built_ins_for_dates::iso_utc_m_nz),
        "iso_utc_h" => Some(crate::core::built_ins_for_dates::iso_utc_h),
        "iso_utc_h_nz" => Some(crate::core::built_ins_for_dates::iso_utc_h_nz),
        "iso_local" => Some(crate::core::built_ins_for_dates::iso_local),
        "iso_local_nz" => Some(crate::core::built_ins_for_dates::iso_local_nz),
        "iso_local_ms" => Some(crate::core::built_ins_for_dates::iso_local_ms),
        "iso_local_ms_nz" => Some(crate::core::built_ins_for_dates::iso_local_ms_nz),
        "iso_local_m" => Some(crate::core::built_ins_for_dates::iso_local_m),
        "iso_local_m_nz" => Some(crate::core::built_ins_for_dates::iso_local_m_nz),
        "iso_local_h" => Some(crate::core::built_ins_for_dates::iso_local_h),
        "iso_local_h_nz" => Some(crate::core::built_ins_for_dates::iso_local_h_nz),
        "date_if_unknown" => Some(crate::core::built_ins_for_dates::date_if_unknown),
        "time_if_unknown" => Some(crate::core::built_ins_for_dates::time_if_unknown),
        "datetime_if_unknown" => Some(crate::core::built_ins_for_dates::datetime_if_unknown),
        // ---- 多类型（BuiltInsForMultipleTypes.java）----
        "string" => Some(crate::core::built_ins_for_multiple_types::string),
        "c" => Some(crate::core::built_ins_for_multiple_types::c),
        "cn" => Some(crate::core::built_ins_for_multiple_types::cn),
        "is_collection_ex" => Some(crate::core::built_ins_for_multiple_types::is_collection_ex),
        "is_date_only" => Some(crate::core::built_ins_for_multiple_types::is_date_only),
        "is_time" => Some(crate::core::built_ins_for_multiple_types::is_time),
        "is_datetime" => Some(crate::core::built_ins_for_multiple_types::is_datetime),
        "is_unknown_date_like" => Some(crate::core::built_ins_for_multiple_types::is_unknown_date_like),
        "namespace" => Some(crate::core::built_ins_for_multiple_types::namespace),
        "absolute_template_name" => Some(crate::core::built_ins_for_multiple_types::absolute_template_name),
        "api" => Some(crate::core::built_ins_for_multiple_types::api),
        "markup_string" => Some(crate::core::built_ins_for_markup_outputs::markup_string),
        // ---- 惰性条件（BuiltInsWithLazyConditionals.java）----
        "then" => Some(crate::core::built_ins_with_lazy_conditionals::then),
        "switch" => Some(crate::core::built_ins_with_lazy_conditionals::switch),
        // ---- 循环变量轮换（BuiltInsForLoopVariables.java）----
        "item_cycle" => Some(crate::core::built_ins_for_loop_variables::item_cycle),
        "item_parity" => Some(crate::core::built_ins_for_loop_variables::item_parity),
        "item_parity_cap" => Some(crate::core::built_ins_for_loop_variables::item_parity_cap),
        // ---- 存在性 null 转换（BuiltInsForExistenceHandling.java）----
        "blank_to_null" => Some(crate::core::built_ins_for_existence_handling::blank_to_null),
        "empty_to_null" => Some(crate::core::built_ins_for_existence_handling::empty_to_null),
        "trim_to_null" => Some(crate::core::built_ins_for_existence_handling::trim_to_null),
        // ---- 柯里化（BuiltInsForCallables.java 基础版）----
        "with_args" => Some(crate::core::built_ins_for_callables::with_args),
        "with_args_last" => Some(crate::core::built_ins_for_callables::with_args_last),
        // ---- 节点（BuiltInsForNode.java）----
        "children" => Some(crate::core::built_ins_for_node::children),
        "parent" => Some(crate::core::built_ins_for_node::parent),
        "root" => Some(crate::core::built_ins_for_node::root),
        "ancestors" => Some(crate::core::built_ins_for_node::ancestors),
        "node_name" => Some(crate::core::built_ins_for_node::node_name),
        "node_type" => Some(crate::core::built_ins_for_node::node_type),
        "node_namespace" => Some(crate::core::built_ins_for_node::node_namespace),
        "next_sibling" => Some(crate::core::built_ins_for_node::next_sibling),
        "previous_sibling" => Some(crate::core::built_ins_for_node::previous_sibling),
        _ => None,
    }
}
