//! 模板加载器接口 —— 对应 Java `freemarker.cache.TemplateLoader`
//! （findTemplateSource/getLastModified/getReader/closeTemplateSource 语义）
//! `as_stateful` 是 Java `instanceof StatefulTemplateLoader` 检查的等价物
//! （TemplateCache.clear :645-657；见 stateful_template_loader.rs）

use crate::cache::stateful_template_loader::StatefulTemplateLoader;
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

    /// 有状态加载器下转型（对应 Java `instanceof StatefulTemplateLoader`，
    /// TemplateCache.clear :648-649）：实现 StatefulTemplateLoader 的加载器
    /// 覆写为 `Some(self)`（trait 上转型），其余保持默认 None —— 与 Java
    /// 可选接口语义一致
    fn as_stateful(&self) -> Option<&dyn StatefulTemplateLoader> {
        None
    }
}
