//! 模板加载器接口 —— 对应 Java `freemarker.cache.TemplateLoader`
//! （findTemplateSource/getLastModified/getReader/closeTemplateSource 语义）

use crate::cache::TemplateSource;
use crate::error::Result;

/// 模板加载器（对应 TemplateLoader 接口）
pub trait TemplateLoader: Send + Sync {
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>>;
    fn read(&self, src: &dyn TemplateSource) -> Result<String>;

    /// 按指定字符集读取 —— 对应 Java `TemplateLoader.getReader(source, encoding)`
    /// （TemplateCache.loadTemplate :524-581 的 parseAsFTL 分支）。默认实现退回
    /// `read`（UTF-8 假定），需要 encoding 语义的实现者覆写。
    fn read_encoded(&self, src: &dyn TemplateSource, encoding: &str) -> Result<String> {
        let _ = encoding;
        self.read(src)
    }

    fn last_modified(&self, src: &dyn TemplateSource) -> Result<i64> {
        let _ = src;
        Ok(0)
    }
}
