//! 查找结果 —— 对应 Java `freemarker.cache.TemplateLookupResult`
//! （查找策略的返回值：命中的源名 + 源；Java 用 `from(path, source)` 构造，
//! 另有 `isLookupSucceeded`/`getSourceName`/`getTemplateSource` 访问器）

use crate::cache::TemplateSource;

/// 查找结果（对应 TemplateLookupResult.java：命中的源名 + 源）
pub struct LookupResult {
    /// 实际命中的源名（acquisition/本地化后可能不同于请求名；Java
    ///   `TemplateLookupResult.from(path, source)` 中的 path）
    pub source_name: String,
    pub source: Box<dyn TemplateSource>,
}

impl LookupResult {
    /// 对应 `TemplateLookupResult.from(path, source)`（Java:43-52）
    pub fn from(source_name: String, source: Box<dyn TemplateSource>) -> Self {
        LookupResult {
            source_name,
            source,
        }
    }

    /// 是否查找成功（Java `isLookupSucceeded`：命中为 true，负查找为 false——
    /// v1 用 Option 表达，None 即失败）
    pub fn is_lookup_succeeded(&self) -> bool {
        true
    }

    /// 命中的源名（Java `getSourceName`）
    pub fn source_name(&self) -> &str {
        &self.source_name
    }
}
