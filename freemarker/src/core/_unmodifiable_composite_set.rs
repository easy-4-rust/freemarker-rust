//! 不可变复合集合 —— 对应 Java `freemarker.core._UnmodifiableCompositeSet`
//! （_UnmodifiableSet 子类；两个 Set 的逻辑并集；iterator/contains/size 合并；
//!  Rust 由两个集合的迭代器链或 HashSet 合并承载）

/// Java 类锚点：`_UnmodifiableCompositeSet` 的 Rust 语义由集合合并操作承载
#[allow(dead_code)]
pub(crate) struct _UnmodifiableCompositeSet;
