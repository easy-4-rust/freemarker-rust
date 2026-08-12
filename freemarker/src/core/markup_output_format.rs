//! 标记输出格式（抽象）—— 对应 Java `freemarker.core.MarkupOutputFormat`
//! （抽象基类：getOutputFormatName + escape；Rust 侧由各具体格式文件的 escape 锚点实现）

/// 抽象锚点：Java 抽象类无实例；各具体格式（html/xml/...）的 escape 见对应文件
#[allow(dead_code)]
pub(crate) struct JavaClassAnchor;
