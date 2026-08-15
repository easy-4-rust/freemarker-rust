//! 解析器配置接口 —— 对应 Java `freemarker.core.ParserConfiguration`
//! （getTagSyntax/getInterpolationSyntax/getNamingConvention/getWhitespaceStripping/
//!  getArithmeticEngine/getOutputFormat 等；Rust 由 Settings 承载对应配置项）

/// 对应 Java `ParserConfiguration`（Rust 由 core::Settings 承载解析器配置语义）
#[allow(dead_code)]
pub(crate) struct ParserConfiguration;
