//! 有序数组集合 —— 对应 Java `freemarker.core._SortedArraySet`
//! （_UnmodifiableSet 子类；基于有序数组的二分查找集合；
//!  Rust 由 BTreeSet 或 sorted Vec + binary_search 承载）

/// Java 类锚点：`_SortedArraySet` 的 Rust 语义由 `BTreeSet` 或排序 `Vec` 承载
#[allow(dead_code)]
pub(crate) struct _SortedArraySet;
