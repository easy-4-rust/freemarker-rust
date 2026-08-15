//! 线程中断支持后处理器 —— 对应 Java `freemarker.core.ThreadInterruptionSupportTemplatePostProcessor`
//!
//! Java 行为：遍历 AST，在每个 `isNestedBlockRepeater()` 的 TemplateElement 前注入
//! `ThreadInterruptionCheck` 元素。该元素在执行时检查 `Thread.currentThread().isInterrupted()`，
//! 若线程被中断则抛出 `TemplateProcessingThreadInterruptedException`（FlowControlException 子类）。
//!
//! # Rust 语义降级
//!
//! Rust 没有 Java 风格的线程中断机制（`Thread.interrupt()` / `Thread.interrupted()` /
//! `Thread.isInterrupted()`）。`std::thread::park_timeout` 和 `AtomicBool` 可以实现
//! 协作取消，但需要调用方显式设置标志——这与 Java 的隐式中断检查语义不同。
//!
//! 因此，本实现按以下策略降级：
//! 1. **无操作实现**：`post_process` 不修改 AST，直接返回 Ok。
//! 2. **文档化差异**：Rust 侧若需线程中断语义，应使用 `tokio::select!` 或
//!    `CancellationToken`（tokio-util）在 async 上下文中实现协作取消。
//! 3. **未来扩展点**：可通过 `AtomicBool` 取消标志注入类似检查元素。
//!
//! # Java 兼容性风险（JavaDoc 原文摘要）
//!
//! - `TemplateDateModel` 实现方若显式检查 body 是否为 null，可能误报"不支持 body"。
//!   应使用 `NestedContentNotSupportedException.check()` 替代简单 null 检查。
//! - `DirectiveCallPlace.isNestedOutputCacheable()` 将始终返回 false（注入的检查元素不可缓存）。
//! - AST 探查软件会看到注入的 `ThreadInterruptionCheck` 元素。

use crate::core::template_post_processor::TemplatePostProcessor;
use crate::core::template_post_processor_exception::TemplatePostProcessorException;
use crate::template::Template;

/// 线程中断支持后处理器（对应 Java `ThreadInterruptionSupportTemplatePostProcessor`）
///
/// Java 实现在 AST 中注入 `ThreadInterruptionCheck` 元素以支持线程中断检查。
/// Rust 无线程中断机制，本实现为**无操作**（post_process 直接返回 Ok）。
///
/// 若需协作取消语义，建议使用：
/// - tokio: `tokio::select!` + `tokio::sync::watch` 或 `CancellationToken`
/// - std: `Arc<AtomicBool>` 取消标志 + 定期检查
#[allow(dead_code)]
pub struct ThreadInterruptionSupportTemplatePostProcessor;

impl TemplatePostProcessor for ThreadInterruptionSupportTemplatePostProcessor {
    /// 无操作实现（Rust 无线程中断机制）
    ///
    /// Java 行为：遍历 AST，在 `isNestedBlockRepeater()` 元素前注入中断检查。
    /// Rust 降级：不修改 AST。差异原因见模块级文档。
    fn post_process(&self, _template: &mut Template) -> Result<(), TemplatePostProcessorException> {
        // 无操作：Rust 无线程中断机制，无法实现 Thread.interrupted() 语义。
        // 若需协作取消，应在调用方（如 tokio task）中实现。
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::Configuration;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn thread_interruption_processor_is_noop() {
        let processor = ThreadInterruptionSupportTemplatePostProcessor;
        let cfg = Rc::new(Configuration::default());
        let mut template = Template::new("test.ftl".to_string(), Vec::new(), HashMap::new(), cfg);
        let original_len = template.root.len();

        // post_process 应成功且不修改 AST
        processor.post_process(&mut template).unwrap();
        assert_eq!(template.root.len(), original_len);
    }
}
