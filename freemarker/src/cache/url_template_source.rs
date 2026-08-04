//! URL 模板源 —— 对应 Java `freemarker.cache.URLTemplateSource`
//! （URLTemplateLoader 返回的模板源：持 URL，lastModified 经 URLConnection
//! 探测（Java :58-79 缓存连接头信息）。
//! v1 占位：与 URLTemplateLoader 的占位实现配套（URL 加载暂不支持），
//! 结构体保留以兼容 API 设计）

use crate::cache::TemplateSource;

/// URL 模板源（对应 URLTemplateSource.java）
pub struct URLTemplateSource {
    url: String,
}

impl URLTemplateSource {
    /// 对应 `URLTemplateSource(URL)`（Java:44-56）；v1 占位——URL 未实际连接
    pub fn new(url: &str) -> Self {
        URLTemplateSource {
            url: url.to_string(),
        }
    }

    /// 对应 `getURL()`（Java:82-84）
    pub fn url(&self) -> &str {
        &self.url
    }
}

impl TemplateSource for URLTemplateSource {
    fn name(&self) -> String {
        self.url.clone()
    }
}
