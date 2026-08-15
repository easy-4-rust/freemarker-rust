//! 单次迭代集合模型 —— 对应 Java `freemarker.core.SingleIterationCollectionModel`
//! （TemplateCollectionModel 实现；iterator() 只能调用一次，第二次抛 IllegalStateException；
//!  防止重复迭代；Rust 由 IntoIterator 的消费语义天然覆盖）

/// Java 类锚点：`SingleIterationCollectionModel`（Rust 的 IntoIterator 消费语义天然覆盖）
#[allow(dead_code)]
pub(crate) struct SingleIterationCollectionModel;
