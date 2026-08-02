//! Java `freemarker.core.TemplateConfigurationWithTemplateCacheTest` 的 Rust 1:1 实现
//! （对应 Java: TemplateConfigurationWithTemplateCacheTest —— TemplateConfiguration
//!   工厂（Conditional/FirstMatch/Merging/FileNameGlobMatcher）经模板缓存作用到
//!   具体模板的 encoding/locale/plainText/booleanFormat/自定义属性）。
//!
//! 引擎差异总览：
//! - Java `TemplateConfiguration` 及 `TemplateConfigurationFactory` 家族
//!   （ConditionalTemplateConfigurationFactory / FirstMatchTemplateConfigurationFactory /
//!   MergingTemplateConfigurationFactory / FileNameGlobMatcher）引擎无 —— 模板级
//!   配置只经 `Configuration.settings` 全局生效，无 per-template 覆盖机制。
//! - `ByteArrayTemplateLoader`（按原始字节 + 编码读取）引擎无（仅 StringLoader/
//!   FileTemplateLoader，字符串已解码）→ 编码相关测试（utf-8/utf-16/iso-8859-x
//!   字节载荷）无法翻译。
//! - `Template.getEncoding()/getLocale()/getBooleanFormat()/getCustomAttribute()`
//!   引擎 Template 无这些字段（encoding 仅记录 `<#ftl encoding>` 头部声明）。
//! - `CustomAttribute` API 引擎无。
//!
//! NOT_APPLICABLE: testEncoding —— ByteArrayTemplateLoader 多编码字节载荷 +
//!   TemplateConfiguration 按文件名的 encoding 工厂（"utf8.ftl"→utf-8 等）；
//!   引擎无字节加载器与 per-template 编码配置（get_template_encoded 可用但仅
//!   影响解码，Template.encoding 字段只记录头部声明）。
//! NOT_APPLICABLE: testIncludeAndEncoding —— <#include> 的 encoding 参数在字节层
//!   重新解码各被包含模板；引擎 include 忽略 encoding 参数（StringLoader 已解码）。
//! NOT_APPLICABLE: testLocale —— ConditionalTemplateConfigurationFactory(
//!   FileNameGlobMatcher("*(de)*"), locale=GERMANY) 覆盖 getTemplate(name, locale)
//!   的请求 locale；引擎无 per-template locale（get_template_localized 仅做候选名
//!   查找，不设置模板 locale）。
//! NOT_APPLICABLE: testPlainText —— MergingTemplateConfigurationFactory 合并
//!   locale/booleanFormat 配置 + getTemplate(name, null, null, parseAsFTL=false)；
//!   引擎无该工厂与 plainText 加载模式。
//! NOT_APPLICABLE: testConfigurableSettings —— MergingTemplateConfigurationFactory
//!   合并 locale/booleanFormat/numberFormat 按文件名模式；引擎无该工厂。
//! NOT_APPLICABLE: testCustomAttributes —— TemplateConfiguration.setCustomAttribute
//!   + CustomAttribute(SCOPE_TEMPLATE) 的模板作用域自定义属性；引擎无该类。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
