//! 合并模板配置工厂 —— 对应 Java `freemarker.cache.MergingTemplateConfigurationFactory`
//! （按加入顺序合并所有子工厂的非 None 结果——后加入者覆盖先加入者的已设置项；
//! 全部 None → None；子工厂结果合并前须绑定 Configuration——Rust 侧合并前
//! 不需要父配置（TemplateConfiguration 无继承链，直接 merge 即可））

use crate::cache::TemplateConfigurationFactory;
use crate::cache::TemplateConfigurationFactoryException;
use crate::core::TemplateConfiguration;
use std::sync::Arc;

/// 合并模板配置工厂（对应 MergingTemplateConfigurationFactory.java）
pub struct MergingTemplateConfigurationFactory {
    factories: Vec<Box<dyn TemplateConfigurationFactory>>,
}

impl MergingTemplateConfigurationFactory {
    pub fn new(factories: Vec<Box<dyn TemplateConfigurationFactory>>) -> Self {
        MergingTemplateConfigurationFactory { factories }
    }
}

impl TemplateConfigurationFactory for MergingTemplateConfigurationFactory {
    /// Java :51-75：按序收集非 None 结果，从第二个起与新合并结果 merge
    /// （Java 惰性创建 mergedTC；Rust 直接惰性合并，语义等价）
    fn get(
        &self,
        source_name: &str,
    ) -> Result<Option<Arc<TemplateConfiguration>>, TemplateConfigurationFactoryException> {
        let mut merged: Option<Arc<TemplateConfiguration>> = None;
        for f in &self.factories {
            if let Some(tc) = f.get(source_name)? {
                match &merged {
                    None => merged = Some(tc),
                    Some(m) => {
                        let mut m2 = (**m).clone();
                        m2.merge(&tc);
                        merged = Some(Arc::new(m2));
                    }
                }
            }
        }
        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ConditionalTemplateConfigurationFactory;
    use crate::cache::FileExtensionMatcher;

    // Java 风格：Default 后逐个 setter 语义（clippy field_reassign_with_default 豁免）
    #[allow(clippy::field_reassign_with_default)]
    #[test]
    fn merging_combines_settings() {
        let mut tc1 = TemplateConfiguration::default();
        tc1.number_format = Some("0.00".to_string());
        let mut tc2 = TemplateConfiguration::default();
        tc2.locale = Some("de_DE".to_string());
        let m = MergingTemplateConfigurationFactory::new(vec![
            Box::new(ConditionalTemplateConfigurationFactory::with_configuration(
                Box::new(FileExtensionMatcher::new("ftl")),
                Arc::new(tc1),
            )),
            Box::new(ConditionalTemplateConfigurationFactory::with_configuration(
                Box::new(FileExtensionMatcher::new("ftl")),
                Arc::new(tc2),
            )),
        ]);
        let out = m.get("a.ftl").unwrap().expect("命中");
        assert_eq!(out.number_format.as_deref(), Some("0.00"));
        assert_eq!(out.locale.as_deref(), Some("de_DE"));
        assert!(m.get("a.txt").unwrap().is_none());
    }
}
