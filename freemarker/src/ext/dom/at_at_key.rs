//! @@ 特殊键枚举 —— 对应 Java `freemarker.ext.dom.AtAtKey`
//! （`@@markup`/`@@text`/`@@qname` 等特殊键的枚举；Rust 侧由 `XmlNode::atat_key` 方法承载）

/// Java 枚举锚点：`AtAtKey`（Rust 侧由 `XmlNode::atat_key` 方法的 match 分支承载）
///
/// Java `AtAtKey` 枚举了所有 `@@` 开头的特殊键，
/// Rust 的等价实现在 `xml/node.rs` 的 `atat_key` 方法中。
#[allow(dead_code)]
pub(crate) struct AtAtKey;
