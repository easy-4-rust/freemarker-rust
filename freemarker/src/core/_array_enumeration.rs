//! 数组枚举器 —— 对应 Java `freemarker.core._ArrayEnumeration`
//! （Java Enumeration 适配器；将 Object[] 包装为 Enumeration；
//!  Rust 迭代器天然覆盖，无需对应实现）

/// Java 类锚点：`_ArrayEnumeration` 的 Rust 语义由 `std::slice::Iter` 天然覆盖
#[allow(dead_code)]
pub(crate) struct _ArrayEnumeration;
