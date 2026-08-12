//! 首匹配模板配置工厂 —— 对应 Java `freemarker.cache.FirstMatchTemplateConfigurationFactory`
//! （按加入顺序调用子工厂，返回第一个非 None 结果；无匹配时默认报错——
//! allow_no_match 可放宽，no_match_error_details 提供更具体的错误说明）

use crate::cache::TemplateConfigurationFactory;
use crate::cache::TemplateConfigurationFactoryException;
use crate::core::TemplateConfiguration;
use std::sync::Arc;

/// 首匹配模板配置工厂（对应 FirstMatchTemplateConfigurationFactory.java）
pub struct FirstMatchTemplateConfigurationFactory {
    factories: Vec<Box<dyn TemplateConfigurationFactory>>,
    allow_no_match: bool,
    no_match_error_details: Option<String>,
}

impl FirstMatchTemplateConfigurationFactory {
    pub fn new(factories: Vec<Box<dyn TemplateConfigurationFactory>>) -> Self {
        FirstMatchTemplateConfigurationFactory {
            factories,
            allow_no_match: false,
            no_match_error_details: None,
        }
    }

    pub fn get_allow_no_match(&self) -> bool {
        self.allow_no_match
    }

    /// 无匹配是否允许（默认 false = 报错；Java :62-64）
    pub fn set_allow_no_match(&mut self, allow_no_match: bool) {
        self.allow_no_match = allow_no_match;
    }

    pub fn get_no_match_error_details(&self) -> Option<&str> {
        self.no_match_error_details.as_deref()
    }

    /// 无匹配错误的补充说明（Java :73-75；默认 None）
    pub fn set_no_match_error_details(&mut self, no_match_error_details: String) {
        self.no_match_error_details = Some(no_match_error_details);
    }

    /// 流式变体（Java `allowNoMatch(boolean)` :93-96）
    pub fn allow_no_match(mut self, allow: bool) -> Self {
        self.set_allow_no_match(allow);
        self
    }

    /// 流式变体（Java `noMatchErrorDetails(String)` :102-105）
    pub fn no_match_error_details(mut self, message: String) -> Self {
        self.set_no_match_error_details(message);
        self
    }
}

impl TemplateConfigurationFactory for FirstMatchTemplateConfigurationFactory {
    /// Java :51-81：首个非 None 子工厂结果；全部 None → allow_no_match 为假则报错
    /// （消息逐字对齐 FirstMatchTemplateConfigurationFactory.java:68-76）
    fn get(
        &self,
        source_name: &str,
    ) -> Result<Option<Arc<TemplateConfiguration>>, TemplateConfigurationFactoryException> {
        for f in &self.factories {
            if let Some(tc) = f.get(source_name)? {
                return Ok(Some(tc));
            }
        }
        if !self.allow_no_match {
            return Err(TemplateConfigurationFactoryException(format!(
                "FirstMatchTemplateConfigurationFactory has found no matching choice for source name \"{source_name}\". {}",
                match &self.no_match_error_details {
                    Some(d) => format!("Error details: {d}"),
                    None => "(Set the noMatchErrorDetails property of the factory bean to give a more specific error message. Set allowNoMatch to true if this shouldn't be an error.)".to_string(),
                }
            )));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::ConditionalTemplateConfigurationFactory;
    use crate::cache::FileExtensionMatcher;

    #[test]
    fn first_match_and_no_match_error() {
        let cfg = Arc::new(TemplateConfiguration::default());
        let cond = ConditionalTemplateConfigurationFactory::with_configuration(
            Box::new(FileExtensionMatcher::new("ftl")),
            cfg.clone(),
        );
        let first =
            FirstMatchTemplateConfigurationFactory::new(vec![Box::new(cond)]).allow_no_match(true);
        assert!(first.get("a.ftl").unwrap().is_some());
        assert!(first.get("a.txt").unwrap().is_none()); // allow_no_match=true
        let first = FirstMatchTemplateConfigurationFactory::new(vec![Box::new(
            ConditionalTemplateConfigurationFactory::with_configuration(
                Box::new(FileExtensionMatcher::new("ftl")),
                cfg,
            ),
        )])
        .no_match_error_details("nope".to_string());
        let err = first.get("a.txt").unwrap_err();
        assert!(err.0.contains("FirstMatchTemplateConfigurationFactory has found no matching choice for source name \"a.txt\""), "{}", err.0);
        assert!(err.0.contains("Error details: nope"), "{}", err.0);
    }
}
