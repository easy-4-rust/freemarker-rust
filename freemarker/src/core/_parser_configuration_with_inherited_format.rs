//! 继承格式的解析器配置 —— 对应 Java `freemarker.core._ParserConfigurationWithInheritedFormat`
//! （内部工具：携带继承格式信息的解析器配置包装；Rust 侧由 `ParserConfiguration` 承载）

/// Java 内部类锚点：`_ParserConfigurationWithInheritedFormat`（Rust 侧由 `ParserConfiguration` 承载）
#[allow(dead_code)]
pub(crate) struct ParserConfigurationWithInheritedFormat;
