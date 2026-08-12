//! 模板源匹配器 —— 对应 Java `freemarker.cache.TemplateSourceMatcher`
//! （@since 2.3.24；per-template 配置体系的匹配条件）。
//! Java 签名 `matches(String sourceName, Object templateSource)` 的
//! templateSource 参数仅供第三方 matcher 读取源内容——内置 7 个 matcher
//! （And/Or/Not/FileExtension/FileNameGlob/PathGlob/PathRegex）均不使用，
//! Rust 侧省略该参数（docs 注明）

/// 模板源匹配器（对应 TemplateSourceMatcher.java）
pub trait TemplateSourceMatcher: Send + Sync {
    /// 源名（模板路径，相对模板存储根）是否匹配
    fn matches(&self, source_name: &str) -> bool;
}
