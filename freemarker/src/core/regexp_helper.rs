//! 正则表达式辅助工具 —— 对应 Java `freemarker.core.RegexpHelper`
//! （parseFlagString/getPattern/checkOnlyHasNonRegexpFlags/checkRegexpFlags；
//!  RE_FLAG_CASE_INSENSITIVE/MULTILINE/COMMENTS/DOTALL/REGEXP/FIRST_ONLY 常量；
//!  Rust 语义由 built_ins_for_strings_regexp.rs 的 ReFlags 承载）

/// Java 类锚点：`RegexpHelper`（Rust 由 built_ins_for_strings_regexp.rs 的 ReFlags 承载语义）
#[allow(dead_code)]
pub(crate) struct RegexpHelper;
