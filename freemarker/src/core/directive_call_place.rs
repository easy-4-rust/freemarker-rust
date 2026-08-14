//! 指令调用位置接口 —— 对应 Java `freemarker.core.DirectiveCallPlace`
//! （getTemplate/getBeginLine/getEndLine/getCustomData/isNestedOutputCacheable；
//!  宏调用的源位置信息；当前 Rust 无公开 AST API → 锚点）

/// 对应 Java `DirectiveCallPlace`（Rust 当前无公开 AST 调用位置 API）
#[allow(dead_code)]
pub(crate) struct DirectiveCallPlace;
