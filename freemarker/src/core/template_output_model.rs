//! 模板输出模型（接口）—— 对应 Java `freemarker.core.TemplateOutputModel`
//! （`?output_format` 相关内建返回的接口：getOutputFormat() + 写为输出字符串；
//! Rust 侧由 `TModel` 标量承载，无独立 trait——Java 接口锚点文件）

/// Java 接口锚点：v1 无独立输出模型 trait，输出统一为标量字符串
#[allow(dead_code)]
pub(crate) struct JavaClassAnchor;
