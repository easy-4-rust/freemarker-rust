//! 特殊内建基类 —— 对应 Java `freemarker.core.SpecialBuiltIn`
//! （抽象类；BuiltIn 子类；标记需要特殊处理的内建函数，
//!  如 ?api、?interpret、?new 等）

/// 对应 Java `SpecialBuiltIn`（BuiltIn 子类基；Rust 由 BuiltIn 变体承载）
#[allow(dead_code)]
pub(crate) struct SpecialBuiltIn;
