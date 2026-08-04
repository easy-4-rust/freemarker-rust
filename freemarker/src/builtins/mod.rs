//! 内建函数注册表 —— 对应 `freemarker.core.BuiltInsFor*.java`（docs/05，133 个 BI）
//! 分派顺序（eval.rs）：① `builtins::lookup` 注册表（本模块）→ ② eval.rs 内建集 → ③
//! `Unknown built-in: ?{name}`。本模块只注册 eval.rs 内建集未覆盖的名称（避免遮蔽）；
//! `?replace`/`?split`/`?matches` 因需要 flags 参数（Java RegexpHelper），从 eval.rs 迁入本模块。
//!
//! 每个文件对应一个 Java 对象（一文件一对象原则）：
//! - strings.rs          → BuiltInsForStringsBasic.java（部分）
//! - strings_encoding.rs → BuiltInsForStringsEncoding.java
//! - strings_regexp.rs   → BuiltInsForStringsRegexp.java
//! - sequences.rs        → BuiltInsForSequences.java（部分）
//! - numbers.rs          → BuiltInsForNumbers.java
//! - dates.rs            → BuiltInsForDates.java
//! - multi.rs            → BuiltInsForMultipleTypes.java（string/c/cn 等）
//! - lazy.rs             → BuiltInsWithLazyConditionals.java
//! - loop_vars.rs        → BuiltInsForLoopVariables.java（轮换类）
//! - existence.rs        → BuiltInsForExistenceHandling.java（null 转换类）
//! - callables.rs        → BuiltInsForCallables.java（?with_args 基础版）

pub mod callables;
pub mod dates;
pub mod existence;
pub mod format;
pub mod hashes;
pub mod iso_date_format;
pub mod java_date_format;
pub mod lazy;
pub mod loop_vars;
pub mod markup_outputs;
pub mod multi;
pub mod nodes;
pub mod numbers;
pub mod sequences;
pub mod strings;
pub mod strings_encoding;
pub mod strings_regexp;

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
        "capitalize" => Some(strings::capitalize),
        "uncap_first" => Some(strings::uncap_first),
        "c_lower_case" => Some(strings::c_lower_case),
        "c_upper_case" => Some(strings::c_upper_case),
        "chop_linebreak" => Some(strings::chop_linebreak),
        "keep_before" => Some(strings::keep_before),
        "keep_before_last" => Some(strings::keep_before_last),
        "keep_after" => Some(strings::keep_after),
        "keep_after_last" => Some(strings::keep_after_last),
        "remove_beginning" => Some(strings::remove_beginning),
        "remove_ending" => Some(strings::remove_ending),
        "ensure_starts_with" => Some(strings::ensure_starts_with),
        "ensure_ends_with" => Some(strings::ensure_ends_with),
        "left_pad" => Some(strings::left_pad),
        "right_pad" => Some(strings::right_pad),
        "last_index_of" => Some(strings::last_index_of),
        "truncate" => Some(strings::truncate),
        "truncate_w" => Some(strings::truncate_w),
        "truncate_c" => Some(strings::truncate_c),
        "truncate_m" => Some(strings::truncate_m),
        "truncate_w_m" => Some(strings::truncate_w_m),
        "truncate_c_m" => Some(strings::truncate_c_m),
        // ---- 字符串编码（BuiltInsForStringsEncoding.java）----
        "j_string" => Some(strings_encoding::j_string),
        "js_string" => Some(strings_encoding::js_string),
        "json_string" => Some(strings_encoding::json_string),
        "url" => Some(strings_encoding::url),
        "url_path" => Some(strings_encoding::url_path),
        "rtf" => Some(strings_encoding::rtf),
        "xhtml" => Some(strings_encoding::xhtml),
        "esc" => Some(markup_outputs::esc),
        "no_esc" => Some(markup_outputs::no_esc),
        // ---- 正则（BuiltInsForStringsRegexp.java）----
        "matches" => Some(strings_regexp::matches),
        "groups" => Some(strings_regexp::groups),
        "replace" => Some(strings_regexp::replace),
        "replace_re" => Some(strings_regexp::replace),
        "split" => Some(strings_regexp::split),
        // ---- 序列（BuiltInsForSequences.java）----
        "chunk" => Some(sequences::chunk),
        "filter" => Some(sequences::filter),
        "map" => Some(sequences::map),
        "take_while" => Some(sequences::take_while),
        "drop_while" => Some(sequences::drop_while),
        "sort" => Some(sequences::sort),
        "sort_by" => Some(sequences::sort_by),
        "min" => Some(sequences::min),
        "max" => Some(sequences::max),
        "seq_index_of" => Some(sequences::seq_index_of),
        "seq_last_index_of" => Some(sequences::seq_last_index_of),
        "sequence" => Some(sequences::sequence),
        // ---- 哈希（BuiltInsForHashes.java）----
        "keys" => Some(hashes::keys),
        "values" => Some(hashes::values),
        // ---- 数字（BuiltInsForNumbers.java）----
        "abs" => Some(numbers::abs),
        "ceiling" => Some(numbers::ceiling),
        "floor" => Some(numbers::floor),
        "round" => Some(numbers::round),
        "byte" => Some(numbers::byte),
        "short" => Some(numbers::short),
        "is_nan" => Some(numbers::is_nan),
        "is_infinite" => Some(numbers::is_infinite),
        "lower_abc" => Some(numbers::lower_abc),
        "upper_abc" => Some(numbers::upper_abc),
        "number_to_date" => Some(numbers::number_to_date),
        "number_to_time" => Some(numbers::number_to_time),
        "number_to_datetime" => Some(numbers::number_to_datetime),
        // ---- 日期（BuiltInsForDates.java；iso 家族全变体对应 BuiltIn.java:175-234）----
        "date" => Some(dates::date),
        "time" => Some(dates::time),
        "datetime" => Some(dates::datetime),
        "iso" => Some(dates::iso),
        "iso_nz" => Some(dates::iso_nz),
        "iso_fz" => Some(dates::iso_fz),
        "iso_ms" => Some(dates::iso_ms),
        "iso_ms_nz" => Some(dates::iso_ms_nz),
        "iso_m" => Some(dates::iso_m),
        "iso_m_nz" => Some(dates::iso_m_nz),
        "iso_h" => Some(dates::iso_h),
        "iso_h_nz" => Some(dates::iso_h_nz),
        "iso_utc" => Some(dates::iso_utc),
        "iso_utc_fz" => Some(dates::iso_utc_fz),
        "iso_utc_nz" => Some(dates::iso_utc_nz),
        "iso_utc_ms" => Some(dates::iso_utc_ms),
        "iso_utc_ms_nz" => Some(dates::iso_utc_ms_nz),
        "iso_utc_m" => Some(dates::iso_utc_m),
        "iso_utc_m_nz" => Some(dates::iso_utc_m_nz),
        "iso_utc_h" => Some(dates::iso_utc_h),
        "iso_utc_h_nz" => Some(dates::iso_utc_h_nz),
        "iso_local" => Some(dates::iso_local),
        "iso_local_nz" => Some(dates::iso_local_nz),
        "iso_local_ms" => Some(dates::iso_local_ms),
        "iso_local_ms_nz" => Some(dates::iso_local_ms_nz),
        "iso_local_m" => Some(dates::iso_local_m),
        "iso_local_m_nz" => Some(dates::iso_local_m_nz),
        "iso_local_h" => Some(dates::iso_local_h),
        "iso_local_h_nz" => Some(dates::iso_local_h_nz),
        "date_if_unknown" => Some(dates::date_if_unknown),
        "time_if_unknown" => Some(dates::time_if_unknown),
        "datetime_if_unknown" => Some(dates::datetime_if_unknown),
        // ---- 多类型（BuiltInsForMultipleTypes.java）----
        "string" => Some(multi::string),
        "c" => Some(multi::c),
        "cn" => Some(multi::cn),
        "is_collection_ex" => Some(multi::is_collection_ex),
        "is_date_only" => Some(multi::is_date_only),
        "is_time" => Some(multi::is_time),
        "is_datetime" => Some(multi::is_datetime),
        "is_unknown_date_like" => Some(multi::is_unknown_date_like),
        "namespace" => Some(multi::namespace),
        "absolute_template_name" => Some(multi::absolute_template_name),
        "api" => Some(multi::api),
        "markup_string" => Some(multi::markup_string),
        // ---- 惰性条件（BuiltInsWithLazyConditionals.java）----
        "then" => Some(lazy::then),
        "switch" => Some(lazy::switch),
        // ---- 循环变量轮换（BuiltInsForLoopVariables.java）----
        "item_cycle" => Some(loop_vars::item_cycle),
        "item_parity" => Some(loop_vars::item_parity),
        "item_parity_cap" => Some(loop_vars::item_parity_cap),
        // ---- 存在性 null 转换（BuiltInsForExistenceHandling.java）----
        "blank_to_null" => Some(existence::blank_to_null),
        "empty_to_null" => Some(existence::empty_to_null),
        "trim_to_null" => Some(existence::trim_to_null),
        // ---- 柯里化（BuiltInsForCallables.java 基础版）----
        "with_args" => Some(callables::with_args),
        "with_args_last" => Some(callables::with_args_last),
        // ---- 节点（BuiltInsForNode.java）----
        "children" => Some(nodes::children),
        "parent" => Some(nodes::parent),
        "root" => Some(nodes::root),
        "ancestors" => Some(nodes::ancestors),
        "node_name" => Some(nodes::node_name),
        "node_type" => Some(nodes::node_type),
        "node_namespace" => Some(nodes::node_namespace),
        "next_sibling" => Some(nodes::next_sibling),
        "previous_sibling" => Some(nodes::previous_sibling),
        _ => None,
    }
}
