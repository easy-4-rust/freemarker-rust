//! 资源包本地化字符串 —— 对应 Java `freemarker.template.ResourceBundleLocalizedString`
//! （Java :54 行：基于 java.util.ResourceBundle 的 LocalizedString 实现）
//!
//! v1 差异：Rust 无 ResourceBundle——用 `HashMap<locale, text>` 近似
//! （文档化偏差；Java 的资源束查找语义（baseName + 变体回退）由调用方提供）

use crate::error::{Result, TemplateError};
use crate::template::localized_string::LocalizedString;
use crate::template::TModel;
use std::collections::HashMap;

/// 资源包本地化字符串（对应 ResourceBundleLocalizedString.java；
/// v1 用 locale→text 映射近似）
pub struct ResourceBundleLocalizedString {
    base_name: String,
    texts: HashMap<String, String>,
}

impl ResourceBundleLocalizedString {
    /// 构造（Java `ResourceBundleLocalizedString(String baseName)` :30-34；
    /// v1 的 texts 映射由调用方填充）
    pub fn new(base_name: &str) -> Self {
        ResourceBundleLocalizedString {
            base_name: base_name.to_string(),
            texts: HashMap::new(),
        }
    }

    pub fn put(&mut self, locale: &str, text: &str) {
        self.texts.insert(locale.to_string(), text.to_string());
    }

    pub fn base_name(&self) -> &str {
        &self.base_name
    }
}

impl LocalizedString for ResourceBundleLocalizedString {
    fn get_localized_string(&self, locale: &str) -> Result<TModel> {
        let text = self.texts.get(locale).ok_or_else(|| {
            TemplateError::misc(format!(
                "No resource bundle entry for locale \"{locale}\" in \"{}\"",
                self.base_name
            ))
        })?;
        Ok(TModel::from_scalar(text.clone()))
    }
}
