//! 模板加载器接口 —— 对应 Java `freemarker.cache.TemplateLoader`
//! （findTemplateSource/getLastModified/getReader/closeTemplateSource 语义）

use crate::error::Result;
use std::any::Any;

/// 模板源（对应 findTemplateSource 返回对象）
pub trait TemplateSource {
    fn name(&self) -> String;

    /// 按具体类型还原（对应 Java 内部按 instanceof 分发，如 MultiSource 委托；
    /// 默认不参与还原，实现者按需覆写）
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }
}

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
