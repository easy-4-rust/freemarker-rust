//! JSON 类格式抽象基类 —— 对应 Java `freemarker.core.AbstractJSONLikeFormat`
//! （JSON 和 JavaScript 共享的格式化逻辑；Rust 侧由 `CFormatKind` 分派承载）

/// Java 抽象类锚点：`AbstractJSONLikeFormat`（Rust 侧由 `CFormatKind::Json`/`CFormatKind::JavaScript` 分派）
#[allow(dead_code)]
pub(crate) struct AbstractJsonLikeFormat;
