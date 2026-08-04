//! 工厂异常 —— 对应 Java `freemarker.cache.TemplateConfigurationFactoryException`
//! （per-template 配置工厂逻辑专用错误；Java 为 checked exception，
//! Rust 侧为 newtype 错误类型，集成处转换为 TemplateError）

use std::fmt;

/// 模板配置工厂异常（对应 TemplateConfigurationFactoryException.java）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateConfigurationFactoryException(pub String);

impl fmt::Display for TemplateConfigurationFactoryException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TemplateConfigurationFactoryException {}
