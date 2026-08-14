//! 数组迭代器 —— 对应 Java `freemarker.core._ArrayIterator`
//! （内部工具：数组的迭代器实现；Rust 由 slice.iter() 天然覆盖）

/// Java 内部类锚点：`_ArrayIterator`（Rust 的 slice.iter() 天然覆盖）
#[allow(dead_code)]
pub(crate) struct ArrayIterator;
