//! JSON 解析器 —— 对应 Java `freemarker.core.JSONParser`
//! （将 JSON 源码解析为 TemplateModel；支持 object/array/string/number/boolean/null；
//!  模块级 JSON 解析功能；Rust 由 serde_json + 模型转换承载）

/// Java 类锚点：`JSONParser` 的 Rust 语义由 serde_json + 模型转换承载
#[allow(dead_code)]
pub(crate) struct JSONParser;
