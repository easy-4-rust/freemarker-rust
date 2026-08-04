//! 缓存存储 —— 对应 Java `freemarker.cache.CacheStorage`
//! （模板缓存的底层键值存储抽象；Java 注释：实现可以非线程安全，由
//! TemplateCache 负责同步——Rust 侧用内部 Mutex 自同步，语义等价）
//! Java 键为 TemplateKey 对象，Rust 键为规范化模板名（与 v1 TemplateCache 一致）

use crate::template::Template;
use std::rc::Rc;

/// 缓存存储（对应 CacheStorage.java：get/put/remove/clear）
pub trait CacheStorage {
    fn get(&mut self, key: &str) -> Option<Rc<Template>>;
    fn put(&mut self, key: &str, value: Rc<Template>);
    fn remove(&mut self, key: &str);
    fn clear(&mut self);
}
