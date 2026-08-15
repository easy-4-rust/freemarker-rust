//! 不可变集合基类 —— 对应 Java `freemarker.core._UnmodifiableSet`
//! （AbstractSet 子类；add/remove/clear 抛 UnsupportedOperationException；
//!  _SortedArraySet/_UnmodifiableCompositeSet 的父类）

/// Java 类锚点：`_UnmodifiableSet` 的 Rust 语义由不可变引用（&Set）天然覆盖
#[allow(dead_code)]
pub(crate) struct _UnmodifiableSet;
