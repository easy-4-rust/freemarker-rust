//! 模板配置工厂 —— 对应 Java `freemarker.cache.TemplateConfigurationFactory`
//! （为模板源创建/返回 TemplateConfiguration；@since 2.3.24）。
//! Java 的 `setConfiguration`/`getConfiguration` 绑定机制（绑定后不可换
//! Configuration，:52-72）——Rust 侧配置经 `Configuration.template_configurations`
//! 字段持有，无独立绑定对象，语义由持有关系保证（文档注明）

use crate::cache::TemplateConfigurationFactoryException;
use crate::core::TemplateConfiguration;
use std::sync::Arc;

/// 模板配置工厂（对应 TemplateConfigurationFactory.java）
pub trait TemplateConfigurationFactory: Send + Sync {
    /// 返回（或创建）给定模板源的模板配置（Java :42-56：匹配失败返回
    /// None——没有适用于该源的配置；工厂逻辑问题抛
    /// TemplateConfigurationFactoryException）
    fn get(
        &self,
        source_name: &str,
    ) -> Result<Option<Arc<TemplateConfiguration>>, TemplateConfigurationFactoryException>;
}
