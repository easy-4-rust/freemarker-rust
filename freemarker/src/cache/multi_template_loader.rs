//! 多模板加载器 —— 对应 Java `freemarker.cache.MultiTemplateLoader`
//! （顺序回退 findTemplateSource :59-95；MultiSource :131-177 按来源 loader 委托
//!   getLastModified/getReader）
//! v1 差异：Java 默认 sticky（记住上次命中的 loader 优先查询，:41-42/:62-72），
//! v1 始终按构造顺序查询（等价 sticky=false，Java:233-235 setSticky）

use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::{Result, TemplateError};
use std::sync::Arc;

/// 多加载器（对应 MultiTemplateLoader；templateLoaders 数组 Java:40）
pub struct MultiLoader {
    /// 按构造顺序查询的加载器（Java:53-56）
    loaders: Vec<Arc<dyn TemplateLoader>>,
}

impl MultiLoader {
    /// 对应 `MultiTemplateLoader(TemplateLoader[])`（Java:53-56）
    pub fn new(loaders: Vec<Arc<dyn TemplateLoader>>) -> Self {
        MultiLoader { loaders }
    }

    /// 加载器数量（对应 `getTemplateLoaderCount`，Java:203-205）
    pub fn len(&self) -> usize {
        self.loaders.len()
    }

    pub fn is_empty(&self) -> bool {
        self.loaders.is_empty()
    }
}

impl TemplateLoader for MultiLoader {
    /// 对应 `findTemplateSource`（Java:59-95）：按顺序查询，第一个命中即返回；
    /// 全部 miss → Ok(None)（Java:90-94）
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        for loader in &self.loaders {
            if let Some(source) = loader.find(name)? {
                return Ok(Some(Box::new(MultiSource {
                    inner: source,
                    loader: loader.clone(),
                })));
            }
        }
        Ok(None)
    }

    /// 对应 `getReader`（Java:103-107）：委托给来源 loader
    fn read(&self, src: &dyn TemplateSource) -> Result<String> {
        let multi = downcast_multi_src(src)?;
        multi.loader.read(&*multi.inner)
    }

    /// 对应 `getReader(source, encoding)`：委托给来源 loader
    fn read_encoded(&self, src: &dyn TemplateSource, encoding: &str) -> Result<String> {
        let multi = downcast_multi_src(src)?;
        multi.loader.read_encoded(&*multi.inner, encoding)
    }

    /// 对应 `getLastModified`（Java:98-100）：委托给来源 loader
    fn last_modified(&self, src: &dyn TemplateSource) -> Result<i64> {
        let multi = downcast_multi_src(src)?;
        multi.loader.last_modified(&*multi.inner)
    }
}

/// 绑定来源 loader 的模板源（对应 Java `MultiSource`，MultiTemplateLoader.java:131-177）
pub struct MultiSource {
    inner: Box<dyn TemplateSource>,
    loader: Arc<dyn TemplateLoader>,
}

impl TemplateSource for MultiSource {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

impl MultiSource {
    /// 从 trait 对象还原（对应 Java instanceof 分发）
    fn downcast(src: &dyn TemplateSource) -> Option<&MultiSource> {
        src.as_any().and_then(|a| a.downcast_ref::<MultiSource>())
    }
}

fn downcast_multi_src(src: &dyn TemplateSource) -> Result<&MultiSource> {
    MultiSource::downcast(src).ok_or_else(|| {
        TemplateError::misc(
            "Not a MultiSource: template source was created by a different TemplateLoader",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 计数包装加载器：记录 find 次数，内容委托给 StringLoader
    struct CountingLoader {
        finds: Arc<AtomicUsize>,
        inner: Arc<StringLoader>,
    }

    impl CountingLoader {
        fn new(finds: Arc<AtomicUsize>) -> Self {
            CountingLoader {
                finds,
                inner: Arc::new(StringLoader::default()),
            }
        }
    }

    impl TemplateLoader for CountingLoader {
        fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
            self.finds.fetch_add(1, Ordering::SeqCst);
            self.inner.find(name)
        }

        fn read(&self, src: &dyn TemplateSource) -> Result<String> {
            self.inner.read(src)
        }

        fn last_modified(&self, src: &dyn TemplateSource) -> Result<i64> {
            self.inner.last_modified(src)
        }
    }

    #[test]
    fn first_hit_wins_second_not_queried() {
        let finds1 = Arc::new(AtomicUsize::new(0));
        let finds2 = Arc::new(AtomicUsize::new(0));
        let l1 = CountingLoader::new(finds1.clone());
        l1.inner.put("a.ftl", "from-first");
        let l2 = CountingLoader::new(finds2.clone());
        l2.inner.put("a.ftl", "from-second");

        let multi = MultiLoader::new(vec![Arc::new(l1), Arc::new(l2)]);
        let src = multi.find("a.ftl").unwrap().expect("第一个 loader 命中");
        assert_eq!(src.name(), "a.ftl");
        // 读与 last_modified 委托给来源 loader（Java:98-107）
        assert_eq!(multi.read(&*src).unwrap(), "from-first");
        assert_eq!(multi.last_modified(&*src).unwrap(), 0);
        assert_eq!(finds1.load(Ordering::SeqCst), 1);
        assert_eq!(
            finds2.load(Ordering::SeqCst),
            0,
            "第一个命中后不应再查第二个"
        );
    }

    #[test]
    fn first_miss_then_second_hit() {
        let finds1 = Arc::new(AtomicUsize::new(0));
        let finds2 = Arc::new(AtomicUsize::new(0));
        let l1 = CountingLoader::new(finds1.clone()); // 无 "b.ftl"
        let l2 = CountingLoader::new(finds2.clone());
        l2.inner.put("b.ftl", "from-second");

        let multi = MultiLoader::new(vec![Arc::new(l1), Arc::new(l2)]);
        let src = multi.find("b.ftl").unwrap().expect("第二个 loader 命中");
        assert_eq!(multi.read(&*src).unwrap(), "from-second");
        assert_eq!(finds1.load(Ordering::SeqCst), 1, "第一个 miss 后查第二个");
        assert_eq!(finds2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn all_miss_returns_none() {
        let finds1 = Arc::new(AtomicUsize::new(0));
        let finds2 = Arc::new(AtomicUsize::new(0));
        let l1 = CountingLoader::new(finds1.clone());
        let l2 = CountingLoader::new(finds2.clone());

        let multi = MultiLoader::new(vec![Arc::new(l1), Arc::new(l2)]);
        assert!(multi.find("nope.ftl").unwrap().is_none());
        assert_eq!(finds1.load(Ordering::SeqCst), 1);
        assert_eq!(
            finds2.load(Ordering::SeqCst),
            1,
            "全部 miss 时两个 loader 都会被查询"
        );
    }
}
