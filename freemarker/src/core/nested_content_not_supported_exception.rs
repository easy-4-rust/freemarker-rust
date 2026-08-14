//! 不支持嵌套内容异常 —— 对应 Java `freemarker.core.NestedContentNotSupportedException`
//! （指令不支持 body 但调用方传入了 body 时抛出；
//!  ThreadInterruptionCheck 注入可能导致误判——Java 用 check() 静态方法过滤）

use crate::error::TemplateError;

/// Java `NestedContentNotSupportedException` 的 Rust 入口
#[allow(dead_code)]
pub(crate) fn new() -> TemplateError {
    TemplateError::misc("This directive doesn't support nested content.")
}
