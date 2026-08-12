//! 映射键值对迭代器 —— 对应 Java `freemarker.template.MapKeyValuePairIterator`
//! （Java :73 行：迭代 Map 条目；Rust 等价物 = `Vec<(String, TModel)>` 的迭代）

use crate::template::TModel;

/// 键值对（对应 Java `MapKeyValuePairIterator` 产出的条目）
pub struct KeyValuePair {
    pub key: String,
    pub value: TModel,
}

/// 映射键值对迭代器（对应 MapKeyValuePairIterator.java；
/// Rust 用 Vec 迭代器承载，hasNext/next 语义经 std Iterator 等价）
pub type MapKeyValuePairIterator = std::vec::IntoIter<KeyValuePair>;
