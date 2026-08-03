//! 模板源 —— 对应 Java `freemarker.cache.TemplateLoader` 的
//! `findTemplateSource` 返回对象（Java 中 URLTemplateSource/StringTemplateSource
//! 等；本 trait 为 Rust 侧统一抽象）

use std::any::Any;

/// 模板源（对应 findTemplateSource 返回对象）
pub trait TemplateSource {
    fn name(&self) -> String;

    /// 按具体类型还原（对应 Java 内部按 instanceof 分发，如 MultiSource 委托；
    /// 默认不参与还原，实现者按需覆写）
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
}
