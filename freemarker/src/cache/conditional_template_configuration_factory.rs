//! 条件模板配置工厂 —— 对应 Java `freemarker.cache.ConditionalTemplateConfigurationFactory`
//! （匹配器命中时返回给定的 TemplateConfiguration 或子工厂的结果；未命中 → None）

use crate::cache::TemplateConfigurationFactory;
use crate::cache::TemplateConfigurationFactoryException;
use crate::cache::TemplateSourceMatcher;
use crate::core::TemplateConfiguration;
use std::sync::Arc;

/// 条件模板配置工厂（对应 ConditionalTemplateConfigurationFactory.java）
pub struct ConditionalTemplateConfigurationFactory {
    matcher: Box<dyn TemplateSourceMatcher>,
    template_configuration: Option<Arc<TemplateConfiguration>>,
    template_configuration_factory: Option<Box<dyn TemplateConfigurationFactory>>,
}

impl ConditionalTemplateConfigurationFactory {
    /// 命中 → 子工厂的结果（Java :47-57）
    pub fn with_factory(
        matcher: Box<dyn TemplateSourceMatcher>,
        template_configuration_factory: Box<dyn TemplateConfigurationFactory>,
    ) -> Self {
        ConditionalTemplateConfigurationFactory {
            matcher,
            template_configuration: None,
            template_configuration_factory: Some(template_configuration_factory),
        }
    }

    /// 命中 → 给定配置（Java :59-66）
    pub fn with_configuration(
        matcher: Box<dyn TemplateSourceMatcher>,
        template_configuration: Arc<TemplateConfiguration>,
    ) -> Self {
        ConditionalTemplateConfigurationFactory {
            matcher,
            template_configuration: Some(template_configuration),
            template_configuration_factory: None,
        }
    }
}

impl TemplateConfigurationFactory for ConditionalTemplateConfigurationFactory {
    /// Java :68-80：命中 → 子工厂结果或给定配置；未命中 → None
    fn get(
        &self,
        source_name: &str,
    ) -> Result<Option<Arc<TemplateConfiguration>>, TemplateConfigurationFactoryException> {
        if self.matcher.matches(source_name) {
            if let Some(f) = &self.template_configuration_factory {
                f.get(source_name)
            } else {
                Ok(self.template_configuration.clone())
            }
        } else {
            Ok(None)
        }
    }
}
