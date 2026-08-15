//! Configuration —— 对应 Java `freemarker.template.Configuration`
//! （加载流见 docs/02 §2.1；缓存键/延迟/局部化回退由 cache 智能体补全）

use crate::cache::TemplateNameFormat;
use crate::cache::{
    NameFormatDefault020300, StringLoader, TemplateCache, TemplateConfigurationFactory,
    TemplateLoader,
};
use crate::core::template_post_processor::{TemplatePostProcessor, TemplatePostProcessorRegistry};
use crate::core::Settings;
use crate::error::{Result, TemplateError};
use crate::parser;
use crate::template::TModel;
use crate::template::Template;
use crate::template::Version;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct Configuration {
    pub settings: Settings,
    pub template_loader: Arc<dyn TemplateLoader>,
    pub cache: Mutex<TemplateCache>,
    pub shared_vars: HashMap<String, TModel>,
    /// 自动导入（ns 名 → 模板路径）—— 对应 Java `Configuration.addAutoImport`
    /// （autoImports 映射；每次渲染前 importLib，Environment.process :322
    /// doAutoImportsAndIncludes → Configuration.doAutoImports :3687-3713）
    pub auto_imports: Vec<(String, String)>,
    /// 自动包含 —— 对应 Java `Configuration.addAutoInclude`（autoIncludes 列表；
    /// doAutoIncludes :3715-3742；addAutoInclude 同层去重——先 remove 再 add，
    /// Configurable.java:2098-2112）
    pub auto_includes: Vec<String>,
    /// per-template 配置工厂 —— 对应 Java `Configuration.setTemplateConfigurations`
    /// （@since 2.3.24；模板加载时按源名匹配，结果应用到模板渲染设置；
    /// Java 的 factory 绑定 Configuration 机制 Rust 侧无对应——Arc 共享持有，
    /// 文档注明）
    pub template_configurations: Option<Arc<dyn TemplateConfigurationFactory>>,
    /// 模板后处理器链 —— 对应 Java `Configuration.templatePostProcessors`
    /// （模板解析/缓存完成后依次执行 AST 变换；
    ///  addTemplatePostProcessor/removeTemplatePostProcessor 管理；
    ///  RefCell 提供内部可变性——配置阶段单线程修改，get_template 时只读借用）
    pub(crate) post_processors: RefCell<TemplatePostProcessorRegistry>,
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
            auto_imports: self.auto_imports.clone(),
            auto_includes: self.auto_includes.clone(),
            template_configurations: self.template_configurations.clone(),
            // 后处理器不随 clone 复制（Java Configuration.clone 同样不复制 postProcessors）
            post_processors: RefCell::new(TemplatePostProcessorRegistry::default()),
        }
    }
}

impl Default for Configuration {
    fn default() -> Self {
        // Java Configuration.loadBuiltInSharedVariables（Configuration.java:1192-1197）：
        // 预置共享变量（capture_output/compress/html_escape/normalize_newlines/xml_escape）。
        // compress/html_escape/normalize_newlines 用真实变换实现
        // （utility_transforms.rs）；capture_output/xml_escape 为文档化偏差
        // （capture_output 需 v1 输出捕获；xml_escape 实体集同 HTMLEnc 差异见
        // StringUtil.XMLEnc vs HTMLEnc——v1 用空变换，见 docs/10）
        let mut shared_vars: HashMap<String, TModel> = HashMap::new();
        for name in ["capture_output", "xml_escape"] {
            shared_vars.insert(
                name.to_string(),
                TModel::from_transform(PredefinedTransform),
            );
        }
        shared_vars.insert(
            "compress".to_string(),
            TModel::from_transform(crate::template::utility::StandardCompressTransform),
        );
        shared_vars.insert(
            "html_escape".to_string(),
            TModel::from_transform(crate::template::utility::HtmlEscapeTransform),
        );
        shared_vars.insert(
            "normalize_newlines".to_string(),
            TModel::from_transform(crate::template::utility::NormalizeNewlinesTransform),
        );
        Configuration {
            settings: Settings::default(),
            template_loader: Arc::new(StringLoader::default()),
            cache: Mutex::new(TemplateCache::default()),
            shared_vars,
            auto_imports: Vec::new(),
            auto_includes: Vec::new(),
            template_configurations: None,
            post_processors: RefCell::new(TemplatePostProcessorRegistry::default()),
        }
    }
}

/// 预置共享变量变换 —— 对应 Java `freemarker.template.utility` 的 HtmlEscape 等
/// （v1：自身无输出，body 透传；capture_output/xml_escape 行为为文档化偏差）
struct PredefinedTransform;
impl crate::template::TemplateTransformModel for PredefinedTransform {
    fn transform(&self, _env: &mut crate::core::Environment) -> Result<()> {
        Ok(())
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

    /// 添加自动导入 —— 对应 Java `Configuration.addAutoImport(String, String)`
    /// （Configurable.java:1944-1960）：同名先移除再追加（移到插入序末尾；
    /// autoImports 顺序即 import 执行顺序）
    pub fn add_auto_import(&mut self, namespace_var_name: &str, template_name: &str) {
        self.auto_imports.retain(|(n, _)| n != namespace_var_name);
        self.auto_imports
            .push((namespace_var_name.to_string(), template_name.to_string()));
    }

    /// 移除自动导入 —— 对应 Java `Configuration.removeAutoImport`（Configurable.java
    /// :1966-1974：不存在则无事发生）
    pub fn remove_auto_import(&mut self, namespace_var_name: &str) {
        self.auto_imports.retain(|(n, _)| n != namespace_var_name);
    }

    /// 添加自动包含 —— 对应 Java `Configuration.addAutoInclude(String)`
    /// （Configurable.java:2083-2096 → :2098-2112：同层去重——已存在先移除再追加）
    pub fn add_auto_include(&mut self, template_name: &str) {
        self.auto_includes.retain(|n| n != template_name);
        self.auto_includes.push(template_name.to_string());
    }

    /// 移除自动包含 —— 对应 Java `Configuration.removeAutoInclude`（Configurable.java
    /// :2175-2186）
    pub fn remove_auto_include(&mut self, template_name: &str) {
        self.auto_includes.retain(|n| n != template_name);
    }

    /// 设置 lazyImports —— 对应 Java `Configuration.setLazyImports(boolean)`
    /// （Configurable.java:1882-1889；写入 settings，Environment::new 继承）
    pub fn set_lazy_imports(&mut self, lazy: bool) {
        self.settings.lazy_imports = lazy;
    }

    /// 设置 lazyAutoImports —— 对应 Java `Configuration.setLazyAutoImports(Boolean)`
    /// （Configurable.java:1912-1920；null = 未设置 → 回退 lazyImports）
    pub fn set_lazy_auto_imports(&mut self, lazy: Option<bool>) {
        self.settings.lazy_auto_imports = lazy;
    }

    /// 设置 per-template 配置工厂 —— 对应 Java `Configuration.setTemplateConfigurations`
    /// （Java 会触发 factory.setConfiguration 绑定；Rust 侧 Arc 共享持有即为绑定，
    /// 无重复绑定检查）
    pub fn set_template_configurations(
        &mut self,
        factory: Option<Arc<dyn TemplateConfigurationFactory>>,
    ) {
        self.template_configurations = factory;
    }

    /// 按源名匹配并应用 per-template 配置（Java：TemplateCache.loadTemplate 加载
    /// 后 `cfg.getTemplateConfiguration` + Template 携带；工厂异常 → 加载失败）
    fn apply_template_configuration(&self, source_name: &str, t: &mut Template) -> Result<()> {
        let Some(factory) = &self.template_configurations else {
            return Ok(());
        };
        match factory.get(source_name) {
            Ok(Some(tc)) => t.template_configuration = Some((*tc).clone()),
            Ok(None) => {}
            Err(e) => {
                return Err(TemplateError::misc(format!(
                    "Failed to get template configuration for source name \"{source_name}\": {e}"
                )))
            }
        }
        Ok(())
    }

    /// 添加模板后处理器 —— 对应 Java `Configuration.addTemplatePostProcessor`
    ///
    /// 处理器按添加顺序执行。每次 get_template 加载新模板（非缓存命中）时，
    /// 注册的后处理器链会依次对模板 AST 执行变换。
    pub fn add_template_post_processor(&self, processor: Box<dyn TemplatePostProcessor>) {
        self.post_processors.borrow_mut().add(processor);
    }

    /// 移除指定位置的后处理器 —— 对应 Java `Configuration.removeTemplatePostProcessor`
    ///
    /// 返回 true 表示成功移除，false 表示索引越界。
    pub fn remove_template_post_processor(&self, index: usize) -> bool {
        self.post_processors.borrow_mut().remove(index)
    }

    /// 清空模板缓存 —— 对应 Java `Configuration.clearTemplateCache()`
    /// （→ TemplateCache.clear :645-657：清空存储；若加载器实现
    /// StatefulTemplateLoader 则同步调用其 resetState —— Java 的 instanceof
    /// 检查 → TemplateLoader::as_stateful 下转型）
    pub fn clear_template_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
        if let Some(sl) = self.template_loader.as_stateful() {
            sl.reset_state();
        }
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

    /// 按指定字符集取模板 —— 对应 Java `Configuration.getTemplate(name, locale, null,
    /// encoding, parseAsFTL=true, ignoreMissing=false)` 的加载路径
    /// （TemplateCache.loadTemplate :524-581）：按 encoding 读取并解析；
    /// `<#ftl encoding=...>` 头声明的编码与读取编码不同（大小写不敏感）→
    /// WrongEncodingException → 按声明编码重读（TemplateCache.java:551-559、
    /// FTL.jj:4625-4631）。encoding=None 时用 input_encoding 设置（缺省 UTF-8，
    /// Java 的 cfg default encoding / 当前模板继承机制见 docs/08 §5.2 偏差）。
    /// v1 不走模板缓存（Java TemplateKey 含 encoding+parse，见 P6 优化项）。
    pub fn get_template_encoded(&self, name: &str, encoding: Option<&str>) -> Result<Rc<Template>> {
        // Java getTemplateInternal（TemplateCache.java:323-341）：先按模板名格式规范化
        // （"/included.ftl" → "included.ftl"）
        let normalized = NameFormatDefault020300.normalize_root_based_name(name)?;
        let mut used =
            encoding.unwrap_or(self.settings.input_encoding.as_deref().unwrap_or("UTF-8"));
        let src =
            self.template_loader
                .find(&normalized)?
                .ok_or_else(|| TemplateError::NotFound {
                    name: name.to_string(),
                })?;
        let mut text = self.template_loader.read_encoded(&*src, used)?;
        let mut t = parser::parse(&Rc::new(self.clone()), &normalized, &text)?;
        // Java FTL.jj:4625-4631：模板内声明编码 vs 构造时读取编码（equalsIgnoreCase）
        if let Some(decl) = &t.encoding {
            if !decl.eq_ignore_ascii_case(used) {
                used = decl;
                text = self.template_loader.read_encoded(&*src, used)?;
                t = parser::parse(&Rc::new(self.clone()), &normalized, &text)?;
            }
        }
        // per-template 配置（Java TemplateCache.loadTemplate 的
        // getTemplateConfiguration 应用点）
        self.apply_template_configuration(&normalized, &mut t)?;
        // 后处理器链（Java TemplateCache.loadTemplate 的 postProcess 调用点；
        // 在模板配置应用后、入缓存前执行 AST 变换）
        if let Err(e) = self.post_processors.borrow().apply_all(&mut t) {
            return Err(TemplateError::misc(format!(
                "Template post-processor failed for \"{normalized}\": {e}"
            )));
        }
        Ok(Rc::new(t))
    }

    /// 对应 `Configuration.getTemplate(name)` → TemplateCache 完整流程
    /// （get_or_load 内含：名称规范化、delay 延迟验证、负查找缓存；见 docs/07 §4）
    pub fn get_template(&self, name: &str) -> Result<Rc<Template>> {
        let mut cache = self.cache.lock().unwrap();
        let loaded = cache.get_or_load(name, &*self.template_loader, |n, text| {
            // 闭包内不得触碰 self.cache（锁已持有）；cfg 克隆重建空缓存（无锁）
            let cfg = Rc::new(self.clone());
            let mut t = parser::parse(&cfg, n, &text)?;
            // per-template 配置（Java TemplateCache.loadTemplate 应用点；
            // 失败时模板尚未入缓存）
            self.apply_template_configuration(n, &mut t)?;
            // 后处理器链（Java TemplateCache.loadTemplate 的 postProcess 调用点；
            // 在模板配置应用后、入缓存前执行 AST 变换）
            if let Err(e) = self.post_processors.borrow().apply_all(&mut t) {
                return Err(TemplateError::misc(format!(
                    "Template post-processor failed for \"{n}\": {e}"
                )));
            }
            Ok(Rc::new(t))
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
pub(crate) fn localized_candidates(name: &str, locale: &str) -> Vec<String> {
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
