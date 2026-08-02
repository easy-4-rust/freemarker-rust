//! 模板缓存 —— 对应 Java `freemarker.cache.TemplateCache`
//! （getTemplateInternal :323-463 / storeNegativeLookup :506-512 / storeCached :514-522 /
//!   setDelay :602-610 / clear :645-657 / removeTemplate :663-686 / CachedTemplate 内部类 :910-933）
//! v1 差异：键 = 规范化名称（Java 为 TemplateKey{name, locale, encoding, ...}，:826-862，
//!   v1 仅保留名称维度）；缓存条目记录源名（规范化绝对路径）与 last_modified 用于过期验证。
//! v1 加载失败直接传播错误、不缓存（Java 把异常也存入负查找条目并重抛，:448-457）

use crate::cache::{NameFormatDefault020300, TemplateNameFormat};
use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::Result;
use crate::template::Template;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// 模板缓存（对应 Java TemplateCache；键 = 规范化名称）
#[derive(Clone)]
pub struct TemplateCache {
    entries: HashMap<String, CachedEntry>,
    /// 刷新延迟（对应 Java `updateDelay`，TemplateCache.java:602-610 setDelay；
    /// 默认 1 秒，对齐 configurable.rs Settings.delay 默认 1）
    delay: Duration,
}

/// 缓存条目 —— 对应 Java `CachedTemplate` 内部类（TemplateCache.java:910-933）
/// `template == None` 表示负查找条目（Java：templateOrException=null + source=null +
/// lastModified=0，见 storeNegativeLookup :506-512；条目记录名称（键）与检查时间）
#[derive(Clone)]
struct CachedEntry {
    /// Java templateOrException：Some(模板) / None（负查找）
    template: Option<Rc<Template>>,
    /// Java source：命中的源名（规范化绝对路径/加载器源名）；负查找条目记录请求名
    source_name: String,
    /// Java lastChecked：上次验证时刻（Java 用毫秒墙钟，v1 用单调时钟）
    last_checked: Instant,
    /// Java lastModified：源的修改时间（加载器返回；负查找条目为 0）
    last_modified: i64,
}

impl Default for TemplateCache {
    fn default() -> Self {
        TemplateCache {
            entries: HashMap::new(),
            delay: Duration::from_secs(1),
        }
    }
}

impl TemplateCache {
    /// 对应 Java `getTemplateInternal`（TemplateCache.java:323-463）完整流程：
    /// 1. 名称规范化（Java 在 getTemplate 入口 normalizeRootBasedName）；
    /// 2. 缓存命中且 delay 内 → 不验证直接返回（Java:350-365；负查找条目返回 None）；
    /// 3. 命中但过期 → 重新查找源，last_modified 与源名未变则复用缓存（Java:388-396），
    ///    否则重新读取并加载（Java:434-447）；源消失 → 转存负查找（Java:377-382）；
    /// 4. 未命中 → 查找；未找到 → 负查找缓存（Java:423-426，storeNegativeLookup）。
    ///    `load` 闭包负责把源文本解析为模板（对应 Java loadTemplate :524-539）。
    ///    返回 Ok(Some(t)) 命中/加载成功；Ok(None) 负查找（Java 返回 null）。
    pub fn get_or_load(
        &mut self,
        name: &str,
        loader: &dyn TemplateLoader,
        load: impl FnOnce(&str, String) -> Result<Rc<Template>>,
    ) -> Result<Option<Rc<Template>>> {
        let normalized = NameFormatDefault020300.normalize_root_based_name(name)?;
        let now = Instant::now();
        if let Some(entry) = self.entries.get(&normalized).cloned() {
            if now.duration_since(entry.last_checked) < self.delay {
                // Java:350-365 —— delay 内不验证，直接返回缓存（负查找条目返回 None）
                return Ok(entry.template);
            }
            return self.refresh_stale(entry, &normalized, loader, now, load);
        }
        // Java:410-429 —— 缓存未命中：查找 + 加载
        match self.find_source(&normalized, loader)? {
            Some(src) => {
                let last_modified = loader.last_modified(&*src)?;
                let text = loader.read(&*src)?;
                let template = load(&normalized, text)?;
                let entry = CachedEntry {
                    template: Some(template.clone()),
                    source_name: src.name(),
                    last_checked: now,
                    last_modified,
                };
                self.entries.insert(normalized, entry);
                Ok(Some(template))
            }
            None => {
                // Java:423-426 —— 负查找缓存（storeNegativeLookup 语义 :506-512）
                self.entries.insert(
                    normalized.clone(),
                    CachedEntry {
                        template: None,
                        source_name: normalized,
                        last_checked: now,
                        last_modified: 0,
                    },
                );
                Ok(None)
            }
        }
    }

    /// 过期条目的重新验证（对应 Java:366-409）
    fn refresh_stale(
        &mut self,
        mut entry: CachedEntry,
        normalized: &str,
        loader: &dyn TemplateLoader,
        now: Instant,
        load: impl FnOnce(&str, String) -> Result<Rc<Template>>,
    ) -> Result<Option<Rc<Template>>> {
        entry.last_checked = now; // Java:371 —— 克隆条目上更新 lastChecked
        match self.find_source(normalized, loader)? {
            None => {
                // Java:377-382 —— 源被移除 → 存负查找（Java:506-512：template=null,
                // source=null, lastModified=0）
                entry.template = None;
                entry.source_name = normalized.to_string();
                entry.last_modified = 0;
                self.entries.insert(normalized.to_string(), entry);
                Ok(None)
            }
            Some(src) => {
                let last_modified = loader.last_modified(&*src)?;
                // Java:388-391 —— 源未变化（lastModified 相同且源名相同）→ 复用缓存，
                // 不重新读取；负查找条目（template=None）不在此列（Java source=null，
                // equals 必然不成立 → 走重载）
                let unchanged = entry.template.is_some()
                    && last_modified == entry.last_modified
                    && src.name() == entry.source_name;
                if unchanged {
                    let template = entry.template.clone();
                    self.entries.insert(normalized.to_string(), entry);
                    return Ok(template);
                }
                // Java:434-447 —— 源变化 → 重新读取并加载
                let text = loader.read(&*src)?;
                let template = load(normalized, text)?;
                let entry = CachedEntry {
                    template: Some(template.clone()),
                    source_name: src.name(),
                    last_checked: now,
                    last_modified,
                };
                self.entries.insert(normalized.to_string(), entry);
                Ok(Some(template))
            }
        }
    }

    /// 对应 `lookupTemplate` → `findTemplateSource`（TemplateCache.java:730-790）。
    /// v1 直接单次 find；局部化回退/acquisition 由 TemplateLookupStrategy 在调用方
    /// （Configuration）组合后替换此处的查找闭包（见 template_lookup_strategy.rs）
    fn find_source(
        &self,
        name: &str,
        loader: &dyn TemplateLoader,
    ) -> Result<Option<Box<dyn TemplateSource>>> {
        loader.find(name)
    }

    /// 兼容接口（template/configuration.rs 正在使用）：直接取缓存，
    /// 不触发查找/验证；负查找条目返回 None。对应 Java 无公开等价 API
    /// （Java getTemplate 走完整 getTemplateInternal；此处为 v1 简化入口）
    pub fn get(&self, name: &str) -> Option<Rc<Template>> {
        self.entries.get(name).and_then(|e| e.template.clone())
    }

    /// 兼容接口（template/configuration.rs 正在使用）：直接放入缓存，
    /// last_checked=now（delay 内视为新鲜），last_modified 未知记 0（下次
    /// 过期验证时以加载器真实值刷新）
    pub fn put(&mut self, name: &str, template: Rc<Template>) {
        self.entries.insert(
            name.to_string(),
            CachedEntry {
                template: Some(template),
                source_name: name.to_string(),
                last_checked: Instant::now(),
                last_modified: 0,
            },
        );
    }

    /// 对应 `clear`（TemplateCache.java:645-657）：清空全部条目，
    /// 强制后续请求重新加载（Java 还调用 StatefulTemplateLoader.resetState，v1 无）
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 对应 `removeTemplate`（TemplateCache.java:663-686）：先规范化名称再移除键。
    /// 返回是否实际存在该条目
    pub fn remove(&mut self, name: &str) -> Result<bool> {
        let normalized = NameFormatDefault020300.normalize_root_based_name(name)?;
        Ok(self.entries.remove(&normalized).is_some())
    }

    /// 对应 `setDelay`（TemplateCache.java:602-610）：设置刷新延迟
    pub fn set_delay(&mut self, delay: Duration) {
        self.delay = delay;
    }

    /// 以秒为单位设置延迟（对齐 Configuration 层 Settings.delay 秒语义，docs/07 §2 :66）
    pub fn set_delay_secs(&mut self, secs: u64) {
        self.delay = Duration::from_secs(secs);
    }

    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// 条目数（含负查找条目）
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::cache::TemplateLoader;
    use crate::template::Configuration;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
    use std::sync::Arc;

    /// 测试用模板工厂（load 闭包）：parser 尚未落地（见 parser 智能体），
    /// 这里直接构造 Template —— 缓存测试只关心 Arc 身份、名称与调用次数，
    /// 不依赖解析内容；正式链路见 configuration.rs 的 get_template
    fn load_closure(name: &str, _text: String) -> Result<Rc<Template>> {
        let cfg = Rc::new(Configuration::new());
        Ok(Rc::new(Template::new(
            name.to_string(),
            Vec::new(),
            HashMap::new(),
            cfg,
        )))
    }

    /// 记录 find 次数的 miss 加载器（负查找测试用）
    struct MissLoader {
        finds: Arc<AtomicUsize>,
    }

    impl TemplateLoader for MissLoader {
        fn find(&self, _name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
            self.finds.fetch_add(1, Ordering::SeqCst);
            Ok(None)
        }

        fn read(&self, _src: &dyn TemplateSource) -> Result<String> {
            Ok(String::new())
        }
    }

    /// 版本化加载器：可控 last_modified；每次 read 返回递增版本文本（delay 测试用）
    struct VersionedLoader {
        finds: Arc<AtomicUsize>,
        reads: Arc<AtomicUsize>,
        last_modified: Arc<AtomicI64>,
    }

    impl VersionedLoader {
        fn new() -> Self {
            VersionedLoader {
                finds: Arc::new(AtomicUsize::new(0)),
                reads: Arc::new(AtomicUsize::new(0)),
                last_modified: Arc::new(AtomicI64::new(0)),
            }
        }
    }

    impl TemplateLoader for VersionedLoader {
        fn find(&self, _name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
            self.finds.fetch_add(1, Ordering::SeqCst);
            Ok(Some(Box::new(StubSource("v.ftl".to_string()))))
        }

        fn read(&self, _src: &dyn TemplateSource) -> Result<String> {
            let v = self.reads.fetch_add(1, Ordering::SeqCst);
            Ok(format!("content-{v}"))
        }

        fn last_modified(&self, _src: &dyn TemplateSource) -> Result<i64> {
            Ok(self.last_modified.load(Ordering::SeqCst))
        }
    }

    struct StubSource(String);

    impl TemplateSource for StubSource {
        fn name(&self) -> String {
            self.0.clone()
        }
    }

    #[test]
    fn hit_within_delay_returns_same_arc() {
        let mut cache = TemplateCache::default();
        let loader = StringLoader::default();
        loader.put("a.ftl", "hello ${x}");

        let first = cache
            .get_or_load("a.ftl", &loader, load_closure)
            .unwrap()
            .expect("首次加载成功");
        // delay 内第二次 get_or_load：不调用 load 闭包（缓存直接命中），返回同一 Arc
        let second = cache
            .get_or_load("a.ftl", &loader, |_, _| panic!("delay 内不应重新加载"))
            .unwrap()
            .expect("缓存命中");
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(first.name, "a.ftl");
    }

    #[test]
    fn put_get_clear_remove() {
        let mut cache = TemplateCache::default();
        assert!(cache.is_empty());

        // put 后 get 返回同一 Arc
        let t = Rc::new(Template::new(
            "x.ftl".to_string(),
            Vec::new(),
            HashMap::new(),
            Rc::new(Configuration::new()),
        ));
        cache.put("x.ftl", t.clone());
        assert!(Rc::ptr_eq(&cache.get("x.ftl").unwrap(), &t));
        assert_eq!(cache.len(), 1);

        // clear 清空
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.get("x.ftl").is_none());

        // remove：不存在 → false
        assert!(!cache.remove("x.ftl").unwrap());

        // remove 前先做名称规范化（Java:663-686 normalizeRootBasedName）
        cache.put("y.ftl", t.clone());
        assert!(
            cache.remove("y/../y.ftl").unwrap(),
            "规范化键应能移除 y.ftl"
        );
        assert!(cache.get("y.ftl").is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn negative_lookup_cached_within_delay() {
        let mut cache = TemplateCache::default();
        cache.set_delay(Duration::from_millis(100));
        let loader = MissLoader {
            finds: Arc::new(AtomicUsize::new(0)),
        };

        // 首次 miss → find 1 次
        assert!(cache
            .get_or_load("missing.ftl", &loader, load_closure)
            .unwrap()
            .is_none());
        assert_eq!(loader.finds.load(Ordering::SeqCst), 1);
        // delay 内重复 get → 负查找命中，不再调用 find（Java:350-365 + :506-512）
        assert!(cache
            .get_or_load("missing.ftl", &loader, load_closure)
            .unwrap()
            .is_none());
        assert_eq!(
            loader.finds.load(Ordering::SeqCst),
            1,
            "delay 内负查找不应重查 loader"
        );
        // 兼容接口 get 对负查找条目返回 None
        assert!(cache.get("missing.ftl").is_none());
        assert_eq!(cache.len(), 1, "负查找条目占用一个条目");

        // 超过 delay → 重新验证（find 次数 +1）
        std::thread::sleep(Duration::from_millis(150));
        assert!(cache
            .get_or_load("missing.ftl", &loader, load_closure)
            .unwrap()
            .is_none());
        assert_eq!(
            loader.finds.load(Ordering::SeqCst),
            2,
            "delay 过期后重新查找"
        );
    }

    #[test]
    fn delay_refresh_reloads_when_last_modified_changes() {
        let mut cache = TemplateCache::default();
        cache.set_delay(Duration::from_millis(100));
        let loader = VersionedLoader::new();

        // 首次加载：find 1 次 / read 1 次
        let t1 = cache
            .get_or_load("v.ftl", &loader, load_closure)
            .unwrap()
            .unwrap();
        assert_eq!(t1.name, "v.ftl");
        assert_eq!(loader.reads.load(Ordering::SeqCst), 1);
        assert_eq!(loader.finds.load(Ordering::SeqCst), 1);

        // delay 内：不重读也不重查（Java:350-365）
        let t2 = cache
            .get_or_load("v.ftl", &loader, |_, _| panic!("delay 内不应重新加载"))
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&t1, &t2));
        assert_eq!(loader.reads.load(Ordering::SeqCst), 1);
        assert_eq!(loader.finds.load(Ordering::SeqCst), 1);

        // 过期但 last_modified 未变 → 重新查找验证（find+1），复用缓存不重读
        // （Java:388-396；注意 Java:371/:395 会在复用后更新 lastChecked，故下方需再等一个 delay）
        std::thread::sleep(Duration::from_millis(150));
        let t3 = cache
            .get_or_load("v.ftl", &loader, |_, _| panic!("源未变不应重新加载"))
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&t1, &t3), "last_modified 未变应复用缓存 Arc");
        assert_eq!(
            loader.reads.load(Ordering::SeqCst),
            1,
            "last_modified 未变不重读"
        );
        assert_eq!(loader.finds.load(Ordering::SeqCst), 2, "过期后重新查找源");

        // last_modified 变化 + 再次过期 → 重新读取并加载（Java:434-447）
        loader.last_modified.store(999, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(150));
        let t4 = cache
            .get_or_load("v.ftl", &loader, load_closure)
            .unwrap()
            .unwrap();
        assert_eq!(
            loader.reads.load(Ordering::SeqCst),
            2,
            "last_modified 变化后重读"
        );
        assert_eq!(loader.finds.load(Ordering::SeqCst), 3);
        assert!(!Rc::ptr_eq(&t1, &t4), "重载后应为新 Arc");
        assert!(t4.name.starts_with("v.ftl"), "重载后模板名应为请求名");
    }

    #[test]
    fn normalized_name_unifies_cache_key() {
        let mut cache = TemplateCache::default();
        let loader = StringLoader::default();
        loader.put("a/b.ftl", "x");

        // "a/./b.ftl" 与 "a/b.ftl" 规范化后共享同一缓存条目（Java 默认
        // TemplateNameFormat.DEFAULT_2_3_0 的 normalizeRootBasedName 处理 /./，
        // 不去冗余斜杠——"a//b.ftl" 保持原样，Java Configuration.java:1113 默认）
        let first = cache
            .get_or_load("a/./b.ftl", &loader, load_closure)
            .unwrap()
            .unwrap();
        let second = cache
            .get_or_load("a/b.ftl", &loader, |_, _| panic!("规范化键应命中"))
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(cache.len(), 1);
        // 冗余斜杠不去除（Default020300 语义）：独立缓存键
        let third = cache
            .get_or_load("a//b.ftl", &loader, load_closure)
            .unwrap();
        assert!(third.is_none(), "a//b.ftl 在 loader 中不存在（未归一化）");
    }

    #[test]
    fn malformed_name_rejected_before_lookup() {
        let mut cache = TemplateCache::default();
        let loader = MissLoader {
            finds: Arc::new(AtomicUsize::new(0)),
        };
        // "../x" 越出根 → 错误（不触达 loader）
        let e = cache
            .get_or_load("../x.ftl", &loader, load_closure)
            .err()
            .expect("越界名应报错");
        assert!(e
            .to_user_message()
            .contains("doesn't stay within the template root directory"));
        assert_eq!(
            loader.finds.load(Ordering::SeqCst),
            0,
            "非法名称不应触达 loader"
        );
    }
}
