//! 模板元素数组构建器 —— 对应 Java `freemarker.core.TemplateElements`
//! （Java 文件名 TemplateElementArrayBuilder.java，类名 TemplateElements；
//!  不可变数组容器 + 计数；解析器累积子元素后一次性构建）

/// 对应 Java `TemplateElements`（解析器内部；Rust 由 Vec<Element> 承载）
#[allow(dead_code)]
pub(crate) struct TemplateElementArrayBuilder;
