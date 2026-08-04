//! 模板加载器接口 —— 对应 Java `freemarker.cache.TemplateLoader`
//! （findTemplateSource/getLastModified/getReader/closeTemplateSource 语义）
//! `reset_state` 钩子对应 Java 可选的 `StatefulTemplateLoader` 接口
//! （StatefulTemplateLoader.java）：Java 经 `instanceof` 检查后调用 resetState，
//! Rust trait 对象无法按接口向下转型——以默认空操作 + 虚分派等价替代
//! （非有状态加载器不覆写即空操作，与 Java 的 instanceof 跳过语义一致）

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

    /// 模板缓存清空回调 —— 对应 Java `StatefulTemplateLoader.resetState`
    /// （TemplateCache.clear :645-657 在清空存储后对 instanceof
    /// StatefulTemplateLoader 的加载器调用；Java 可选接口，Rust 用默认空操作
    /// 保持等价）。有状态加载器（如 MultiLoader，见其 reset_state 的传播语义）
    /// 覆写此方法重置内部状态。
    fn reset_state(&self) {}
}
