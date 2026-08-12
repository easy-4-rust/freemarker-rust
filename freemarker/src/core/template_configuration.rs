//! 模板级配置 —— 对应 Java `freemarker.core.TemplateConfiguration`
//! （@since 2.3.24；per-template 配置体系：匹配到模板源的设置覆盖，未设置项
//! 继承 Configuration 全局值——Java 用 isXxxSet 标志 + 父 Configuration 继承链，
//! Rust 用 Option 字段等价；解析期设置（tagSyntax/interpolationSyntax/
//! namingConvention/recognizeStandardFileExtensions/tabSize 等）v1 无对应
//! 解析参数 —— 登记 NA，见 docs/JavaRust结构对照）

use crate::builtins::format::CFormatKind;
use crate::core::{AutoEscaping, OutputFormatKind, Settings};

/// 模板级配置（对应 TemplateConfiguration.java；字段 = v1 Settings 已实现的设置项）
#[derive(Clone, Default, Debug)]
pub struct TemplateConfiguration {
    pub whitespace_stripping: Option<bool>,
    pub strict_syntax: Option<bool>,
    pub output_format: Option<OutputFormatKind>,
    /// 模板读取编码（Java `encoding` 属性；v1 映射 Settings.input_encoding）
    pub encoding: Option<String>,
    pub locale: Option<String>,
    pub number_format: Option<String>,
    pub boolean_format: Option<String>,
    pub date_format: Option<String>,
    pub time_format: Option<String>,
    pub date_time_format: Option<String>,
    pub output_encoding: Option<String>,
    pub url_escaping_charset: Option<String>,
    pub auto_escaping: Option<AutoEscaping>,
    pub c_format: Option<CFormatKind>,
    pub classic_compatible: Option<bool>,
    /// 模板层 auto imports —— 对应 Java `TemplateConfiguration.autoImports`
    /// （TemplateConfiguration extends Configurable，继承 addAutoImport；apply(Template)
    /// 时合并进 Template 对象，TemplateConfiguration.java:399-402——Rust 侧由
    /// Environment::new 复制为 t 层数据，doAutoImportsAndIncludes 使用）
    pub auto_imports: Vec<(String, String)>,
    /// 模板层 auto includes（Java TemplateConfiguration.java:404-406 同构）
    pub auto_includes: Vec<String>,
    /// 模板层 lazyImports —— 对应 Java `TemplateConfiguration.lazyImports`
    /// （apply 时仅当 Template 未自行设置才覆盖，:383-385；Rust 侧 apply_to 直接覆盖，
    /// 与现有其它设置项一致——v1 简化）
    pub lazy_imports: Option<bool>,
    /// 模板层 lazyAutoImports（Java TemplateConfiguration.java:386-388 同构；
    /// None = 未设置）
    pub lazy_auto_imports: Option<bool>,
}

impl TemplateConfiguration {
    /// 应用到设置（Java：Environment 初始化时应用模板配置——未设置项保持
    /// 继承的全局值；对应 Java `Configurable.set*` 各 setter 的赋值路径）
    pub fn apply_to(&self, s: &mut Settings) {
        if let Some(v) = &self.whitespace_stripping {
            s.whitespace_stripping = *v;
        }
        if let Some(v) = &self.strict_syntax {
            s.strict_syntax = *v;
        }
        if let Some(v) = &self.output_format {
            s.output_format = *v;
        }
        if let Some(v) = &self.encoding {
            s.input_encoding = Some(v.clone());
        }
        if let Some(v) = &self.locale {
            s.locale = v.clone();
        }
        if let Some(v) = &self.number_format {
            s.number_format = v.clone();
        }
        if let Some(v) = &self.boolean_format {
            s.boolean_format = v.clone();
        }
        if let Some(v) = &self.date_format {
            s.date_format = v.clone();
        }
        if let Some(v) = &self.time_format {
            s.time_format = v.clone();
        }
        if let Some(v) = &self.date_time_format {
            s.date_time_format = v.clone();
        }
        if let Some(v) = &self.output_encoding {
            s.output_encoding = v.clone();
        }
        if let Some(v) = &self.url_escaping_charset {
            s.url_escaping_charset = v.clone();
        }
        if let Some(v) = &self.auto_escaping {
            s.auto_escaping = *v;
        }
        if let Some(v) = &self.c_format {
            s.c_format = *v;
        }
        if let Some(v) = &self.classic_compatible {
            s.classic_compatible = *v;
        }
        if let Some(v) = &self.lazy_imports {
            s.lazy_imports = *v;
        }
        if let Some(v) = &self.lazy_auto_imports {
            s.lazy_auto_imports = Some(*v);
        }
    }

    /// 合并（Java `merge`，TemplateConfiguration.java:163-：参数中**已设置**
    /// 的项覆盖本对象值；v1 的 Option 字段即"已设置"标志）
    pub fn merge(&mut self, other: &TemplateConfiguration) {
        if other.whitespace_stripping.is_some() {
            self.whitespace_stripping = other.whitespace_stripping;
        }
        if other.strict_syntax.is_some() {
            self.strict_syntax = other.strict_syntax;
        }
        if other.output_format.is_some() {
            self.output_format = other.output_format;
        }
        if other.encoding.is_some() {
            self.encoding = other.encoding.clone();
        }
        if other.locale.is_some() {
            self.locale = other.locale.clone();
        }
        if other.number_format.is_some() {
            self.number_format = other.number_format.clone();
        }
        if other.boolean_format.is_some() {
            self.boolean_format = other.boolean_format.clone();
        }
        if other.date_format.is_some() {
            self.date_format = other.date_format.clone();
        }
        if other.time_format.is_some() {
            self.time_format = other.time_format.clone();
        }
        if other.date_time_format.is_some() {
            self.date_time_format = other.date_time_format.clone();
        }
        if other.output_encoding.is_some() {
            self.output_encoding = other.output_encoding.clone();
        }
        if other.url_escaping_charset.is_some() {
            self.url_escaping_charset = other.url_escaping_charset.clone();
        }
        if other.auto_escaping.is_some() {
            self.auto_escaping = other.auto_escaping;
        }
        if other.c_format.is_some() {
            self.c_format = other.c_format;
        }
        if other.classic_compatible.is_some() {
            self.classic_compatible = other.classic_compatible;
        }
        // 列表/惰性设置：非空（已设置）→ 覆盖（Java merge 语义同"已设置项覆盖"）
        if !other.auto_imports.is_empty() {
            self.auto_imports = other.auto_imports.clone();
        }
        if !other.auto_includes.is_empty() {
            self.auto_includes = other.auto_includes.clone();
        }
        if other.lazy_imports.is_some() {
            self.lazy_imports = other.lazy_imports;
        }
        if other.lazy_auto_imports.is_some() {
            self.lazy_auto_imports = other.lazy_auto_imports;
        }
    }

    /// 是否有任何设置（Java `hasAnyConfigurableSet`，:676-；Factory 判定
    /// 用——v1 恒为 true 也无妨，保留语义）
    pub fn is_empty(&self) -> bool {
        self.whitespace_stripping.is_none()
            && self.strict_syntax.is_none()
            && self.output_format.is_none()
            && self.encoding.is_none()
            && self.locale.is_none()
            && self.number_format.is_none()
            && self.boolean_format.is_none()
            && self.date_format.is_none()
            && self.time_format.is_none()
            && self.date_time_format.is_none()
            && self.output_encoding.is_none()
            && self.url_escaping_charset.is_none()
            && self.auto_escaping.is_none()
            && self.c_format.is_none()
            && self.classic_compatible.is_none()
            && self.auto_imports.is_empty()
            && self.auto_includes.is_empty()
            && self.lazy_imports.is_none()
            && self.lazy_auto_imports.is_none()
    }

    /// 添加模板层自动导入 —— 对应 Java `TemplateConfiguration.addAutoImport`
    /// （Configurable.java:1944-1960：同名先移除再追加）
    pub fn add_auto_import(&mut self, namespace_var_name: &str, template_name: &str) {
        self.auto_imports.retain(|(n, _)| n != namespace_var_name);
        self.auto_imports
            .push((namespace_var_name.to_string(), template_name.to_string()));
    }

    /// 添加模板层自动包含 —— 对应 Java `TemplateConfiguration.addAutoInclude`
    /// （Configurable.java:2083-2096 → :2098-2112：同层去重）
    pub fn add_auto_include(&mut self, template_name: &str) {
        self.auto_includes.retain(|n| n != template_name);
        self.auto_includes.push(template_name.to_string());
    }

    /// 设置模板层 lazyImports（Java `TemplateConfiguration.setLazyImports`）
    pub fn set_lazy_imports(&mut self, lazy: bool) {
        self.lazy_imports = Some(lazy);
    }

    /// 设置模板层 lazyAutoImports（Java `TemplateConfiguration.setLazyAutoImports(Boolean)`）
    pub fn set_lazy_auto_imports(&mut self, lazy: Option<bool>) {
        self.lazy_auto_imports = lazy;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Java 风格：Default 后逐个 setter 语义（clippy field_reassign_with_default 豁免）
    #[allow(clippy::field_reassign_with_default)]
    #[test]
    fn apply_to_overrides_and_merge() {
        let mut tc = TemplateConfiguration::default();
        tc.locale = Some("de_DE".to_string());
        tc.number_format = Some("0.00".to_string());
        let mut s = Settings::default();
        tc.apply_to(&mut s);
        assert_eq!(s.locale, "de_DE");
        assert_eq!(s.number_format, "0.00");

        let mut tc2 = TemplateConfiguration::default();
        tc2.number_format = Some("0.###".to_string());
        tc2.boolean_format = Some("c".to_string());
        tc.merge(&tc2);
        assert_eq!(tc.number_format.as_deref(), Some("0.###"));
        assert_eq!(tc.boolean_format.as_deref(), Some("c"));
        assert_eq!(tc.locale.as_deref(), Some("de_DE"), "未覆盖项保留");
        assert!(!tc.is_empty());
        assert!(TemplateConfiguration::default().is_empty());
    }
}
