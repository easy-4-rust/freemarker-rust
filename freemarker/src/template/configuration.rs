//! Configuration —— 对应 Java `freemarker.template.Configuration`
//! （加载流见 docs/02 §2.1；缓存键/延迟/局部化回退由 cache 智能体补全）

use crate::cache::{StringLoader, TemplateCache, TemplateLoader};
use crate::core::Settings;
use crate::error::{Result, TemplateError};
use crate::parser;
use crate::template::TModel;
use crate::template::Template;
use crate::template::Version;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct Configuration {
    pub settings: Settings,
    pub template_loader: Arc<dyn TemplateLoader>,
    pub cache: Mutex<TemplateCache>,
    pub shared_vars: HashMap<String, TModel>,
}

impl Clone for Configuration {
    /// 克隆共享 loader 与设置；缓存重建为空（对应 Java `Configuration.clone`：
    /// 新实例重建 TemplateCache，Configuration.java:1175-1189）
    fn clone(&self) -> Self {
        Configuration {
            settings: self.settings.clone(),
            template_loader: self.template_loader.clone(),
            cache: Mutex::new(TemplateCache::default()),
            shared_vars: self.shared_vars.clone(),
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            settings: Settings::default(),
            template_loader: Arc::new(StringLoader::default()),
            cache: Mutex::new(TemplateCache::default()),
            shared_vars: HashMap::new(),
        }
    }
}

impl Configuration {
    /// 等价 `new Configuration(Configuration.VERSION_2_3_34)` + 默认设置
    pub fn new() -> Self {
        Configuration::default()
    }

    pub fn set_shared_variable(&mut self, name: &str, model: TModel) {
        self.shared_vars.insert(name.to_string(), model);
    }

    /// 带局部化回退的取模板 —— 对应 Java `TemplateCache.lookupWithLocalizedThenAcquisitionStrategy`
    /// （TemplateCache.java:914-948）：locale 非空时按 `前缀_语言[_国家][_变体]后缀` 逐级缩短
    /// 尝试（en_AU → en → 无后缀），首个命中的使用；全部未命中退回原名。
    pub fn get_template_localized(&self, name: &str, locale: Option<&str>) -> Result<Rc<Template>> {
        if let Some(loc) = locale {
            for cand in localized_candidates(name, loc) {
                if self.template_loader.find(&cand)?.is_some() {
                    return self.get_template(&cand);
                }
            }
        }
        self.get_template(name)
    }

    /// 对应 `Configuration.getTemplate(name)` → TemplateCache 完整流程
    /// （get_or_load 内含：名称规范化、delay 延迟验证、负查找缓存；见 docs/07 §4）
    pub fn get_template(&self, name: &str) -> Result<Rc<Template>> {
        let mut cache = self.cache.lock().unwrap();
        let loaded = cache.get_or_load(name, &*self.template_loader, |n, text| {
            // 闭包内不得触碰 self.cache（锁已持有）；cfg 克隆重建空缓存（无锁）
            let cfg = Rc::new(self.clone());
            parser::parse(&cfg, n, &text).map(Rc::new)
        })?;
        // Ok(None) = 负查找缓存（Java storeNegativeLookup 后抛 TemplateNotFoundException）
        loaded.ok_or_else(|| TemplateError::NotFound {
            name: name.to_string(),
        })
    }

    pub fn version() -> Version {
        Version::V2_3_34
    }
}

/// Java TemplateCache 的局部化候选名序列：
/// "localization.ftl" + "en_AU" → ["localization_en_AU.ftl", "localization_en.ftl", "localization.ftl"]
/// （TemplateCache.java:922-946：localeName 逐级去掉最后一个 "_" 段）
fn localized_candidates(name: &str, locale: &str) -> Vec<String> {
    let last_dot = name.rfind('.');
    let (prefix, suffix) = match last_dot {
        Some(i) => (&name[..i], &name[i..]),
        None => (name, ""),
    };
    let mut out = Vec::new();
    let mut loc = format!("_{locale}");
    loop {
        out.push(format!("{prefix}{loc}{suffix}"));
        match loc.rfind('_') {
            Some(i) => loc.truncate(i),
            None => break,
        }
    }
    out
}
