//! 集合与序列适配器 —— 对应 Java `freemarker.core.CollectionAndSequence`
//! （为集合添加序列能力，或为序列添加集合能力；用于 ?keys/?values 内建；
//!  Rust 侧由 `TModel` 的 sequence + collection 双槽位天然覆盖）

/// Java 类锚点：`CollectionAndSequence`（Rust 的 `TModel` 双槽位天然覆盖）
///
/// Java `CollectionAndSequence` 为 `TemplateCollectionModel` 添加 `TemplateSequenceModel` 能力，
/// Rust 的 `TModel` 可同时持有 sequence 和 collection 槽位，无需额外适配器。
#[allow(dead_code)]
pub(crate) struct CollectionAndSequence;
