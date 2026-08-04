//! 有状态模板加载器 —— 对应 Java `freemarker.cache.StatefulTemplateLoader`
//! （模板缓存被清空时需同步重置自身状态的加载器；TemplateCache.clear :645-657
//! 在清空存储后对 `instanceof StatefulTemplateLoader` 的加载器调用 resetState）
//!
//! Rust 侧 `instanceof` 检查的等价物：`TemplateLoader::as_stateful` 默认返回
//! None，实现者覆写为 `Some(self)`（trait 上转型）——与 Java 的可选接口
//! 语义一致（非有状态加载器不实现即跳过）

use crate::cache::TemplateLoader;

/// 有状态模板加载器（对应 StatefulTemplateLoader.java：`Configuration.clearTemplateCache()`
/// 清空模板缓存时，若加载器实现本接口则同步重置内部状态）
pub trait StatefulTemplateLoader: TemplateLoader {
    /// 重置内部状态（Java `resetState`；如 MultiTemplateLoader 的 sticky/sick
    /// 记忆清理与对内部加载器的传播，MultiTemplateLoader.java:115-126）
    fn reset_state(&self);
}
