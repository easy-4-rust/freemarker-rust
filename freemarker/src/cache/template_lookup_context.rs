//! 查找上下文 —— 对应 Java `freemarker.cache.TemplateLookupContext`
//! （查找策略与模板缓存之间的上下文接口：`findTemplateSource`/`contains`/
//! `isLookupSucceeded`/`negativeLookup` 等；TemplateLookupStrategy.lookup
//! 的入参）
//!
//! Rust 等价物：`FindFn` 闭包（每次调用 = 一次 TemplateLoader.findTemplateSource，
//! 对应 Java 上下文的 `findTemplateSource(String)` 方法）；策略 trait 的
//! `lookup` 经闭包参数访问缓存，语义等价（v1 文档化偏差）

use crate::cache::TemplateSource;
use crate::error::Result;

/// 模板源查找闭包类型（对应 TemplateLookupContext.findTemplateSource：
/// TemplateCache.java:897-908；type_complexity 豁免）
pub type FindFn<'a> = &'a mut dyn FnMut(&str) -> Result<Option<Box<dyn TemplateSource>>>;
