//! 遗留构造器解析配置 —— 对应 Java `freemarker.core.LegacyConstructorParserConfiguration`
//! （ParserConfiguration 实现；Configuration 构造器的旧参数包装；
//!  tagSyntax/interpolationSyntax/namingConvention/whitespaceStripping 等；
//!  Rust 由 Settings 直接承载 → 锚点）

/// Java 类锚点：`LegacyConstructorParserConfiguration` 的 Rust 语义由 Settings 承载
#[allow(dead_code)]
pub(crate) struct LegacyConstructorParserConfiguration;
