//! freemarker-pyo3 —— freemarker 模板引擎的 Python 绑定
//! 对应 Java `freemarker-jython25`（freemarker.ext.jython 的 pyo3 等价物）；
//! 设计见 docs/10（逐节对照：§1 API 面、§2 wrap、§3 unwrap、§4 GIL、§5 异常桥接）。
//!
//! 模块布局（一 .rs 文件对应一个 Java 对象）：
//! - wrapper.rs → JythonWrapper（PyObjectWrapper）
//! - models.rs  → JythonModel 家族（PyObjectModel：Number/Hash/Sequence/Generic）
//! - bridge.rs  → TemplateModelToJythonAdapter（TemplateModelAdapter）
//! - errors.rs  → 异常桥（PyErr ↔ TemplateError；FreeMarkerError 异常类）
//! - lib.rs     → Configuration/Template（FmConfiguration/FmTemplate）+ #[pymodule]
//!
//! 核心约束（docs/10 §2）：freemarker::Configuration / TModel 含 Rc（非 Send）→
//! 所有 #[pyclass] 标记 `unsendable`（仅创建线程可用，pyo3 运行时校验）。
//! GIL 纪律（docs/10 §4）：渲染入口（FmTemplate::process）持有单次 GIL；
//! 模型 trait 方法内部经 Python::attach 获取（可重入、零额外开销）；
//! v1 不使用 allow_threads（纯文本段暂不释放 GIL，注释于 process）。

mod bridge;
mod errors;
mod models;
mod wrapper;

// `#[pymodule] fn freemarker` 与依赖 crate 同名 → 以别名 fm 引用 crate（:: 前缀指 extern crate）
use ::freemarker as fm;

use crate::errors::{template_error_to_pyerr, FreeMarkerError};
use crate::wrapper::{PyObjectWrapper, PyObjectWrapperInner};
use fm::builtins::format::CFormatKind;
use fm::cache::{LookupStrategyKind, StringLoader};
use fm::core::{AutoEscaping, OutputFormatKind, TzSetting};
use fm::template::TModel;
use fm::Template;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// FmConfiguration —— 对应 Java `freemarker.template.Configuration`
/// （docs/10 §1 映射表：new() 默认 incompatibleImprovements=2.3.34）
/// `#[pyclass(unsendable)]`：内部 freemarker::Configuration 的 shared_vars 含
/// TModel（Rc，非 Send），且模板解析产物经 `Rc<Configuration>` 共享 —— 主线程专用。
#[pyclass(module = "freemarker", unsendable)]
pub struct FmConfiguration {
    inner: fm::Configuration,
    /// 模板文本加载器（对应 Java StringTemplateLoader；put_template 写入）
    loader: Arc<StringLoader>,
    /// Python 对象包装器（对应 Java Configuration.objectWrapper = JythonWrapper）
    wrapper: Arc<PyObjectWrapperInner>,
}

impl FmConfiguration {
    /// 当前配置时区（naive datetime 的解释时区；docs/10 §2）
    fn tz(&self) -> Option<TzSetting> {
        Some(self.inner.settings.time_zone)
    }
}

#[pymethods]
impl FmConfiguration {
    /// 对应 Java `new Configuration(Configuration.VERSION_2_3_34)`（默认设置）
    #[new]
    fn new() -> Self {
        let mut cfg = fm::Configuration::new();
        let loader = Arc::new(StringLoader::default());
        cfg.template_loader = loader.clone();
        FmConfiguration {
            inner: cfg,
            loader,
            wrapper: PyObjectWrapperInner::new(true, false),
        }
    }

    /// 对应 Java `Configuration.setObjectWrapper(wrapper)`（仅取 attributes_shadow_items
    /// 与 use_cache 配置；PyObjectWrapper 构造参数，docs/10 §1）
    fn set_object_wrapper(&mut self, wrapper: &PyObjectWrapper) {
        self.wrapper.attributes_shadow_items.store(
            wrapper
                .inner
                .attributes_shadow_items
                .load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        self.wrapper.use_cache.store(
            wrapper.inner.use_cache.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
    }

    /// 对应 Java `Configuration.setSharedVariable(name, obj)` —— 对象先经
    /// PyObjectWrapper.wrap 转 TModel（Java：wrap 后存入 sharedVariables）
    fn set_shared_variable(
        &mut self,
        py: Python<'_>,
        name: String,
        obj: Py<PyAny>,
    ) -> PyResult<()> {
        let model = self
            .wrapper
            .wrap(py, obj.bind(py), self.tz())?
            .unwrap_or_else(TModel::nothing);
        self.inner.set_shared_variable(&name, model);
        Ok(())
    }

    /// 对应 Java StringTemplateLoader.put(name, source)（测试/动态模板装载；
    /// 写入后经 Configuration.getTemplate 可见）
    fn put_template(&mut self, name: String, source: String) {
        self.loader.put(&name, &source);
    }

    /// 对应 Java `Configuration.getTemplate(name)` → Template 完整缓存流程
    /// （NotFound → FreeMarkerError）
    /// 注意：模板解析时 Configuration 被快照进 Rc<Configuration>（核心 crate 设计），
    /// 故 get_template 之后调用 set_shared_variable 不作用于该模板（需先设共享变量）。
    fn get_template(&self, name: String) -> PyResult<FmTemplate> {
        let template = self
            .inner
            .get_template(&name)
            .map_err(template_error_to_pyerr)?;
        Ok(FmTemplate {
            inner: template,
            wrapper: self.wrapper.clone(),
        })
    }

    // ===================================================================
    // 配置桥接方法 —— 对应 Java Configuration / Configurable 设置
    // ===================================================================

    // --- 格式/渲染设置（直写 Settings 字段）---

    /// 对应 Java `Configuration.setLocale(Locale)`
    fn set_locale(&mut self, locale: String) {
        self.inner.settings.locale = locale;
    }

    /// 对应 Java `Configuration.setTimeZone(TimeZone)`
    /// 解析 IANA 名称或 GMT±HH:MM 固定偏移；非法值抛 ValueError。
    fn set_time_zone(&mut self, tz: String) -> PyResult<()> {
        match tz.parse::<TzSetting>() {
            Ok(t) => {
                self.inner.settings.time_zone = t;
                self.inner.settings.time_zone_id = fm::core::java_time_zone_id(&tz);
                Ok(())
            }
            Err(()) => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid time zone: {tz}"
            ))),
        }
    }

    /// 对应 Java `Configuration.setNumberFormat(String)`
    fn set_number_format(&mut self, fmt: String) {
        self.inner.settings.number_format = fmt;
    }

    /// 对应 Java `Configuration.setBooleanFormat(String)`
    fn set_boolean_format(&mut self, fmt: String) {
        self.inner.settings.boolean_format = fmt;
    }

    /// 对应 Java `Configuration.setDateFormat(String)`
    fn set_date_format(&mut self, fmt: String) {
        self.inner.settings.date_format = fmt;
    }

    /// 对应 Java `Configuration.setTimeFormat(String)`
    fn set_time_format(&mut self, fmt: String) {
        self.inner.settings.time_format = fmt;
    }

    /// 对应 Java `Configuration.setDateTimeFormat(String)`
    fn set_date_time_format(&mut self, fmt: String) {
        self.inner.settings.date_time_format = fmt;
    }

    /// 对应 Java `Configuration.setOutputFormat(OutputFormat)`
    /// 接受 "HTML"/"XML"/"XHTML"/"PlainText"/"JavaScript"/"JSON"/"CSS"/"RTF"。
    fn set_output_format(&mut self, name: String) -> PyResult<()> {
        match OutputFormatKind::parse(&name) {
            Some(kind) => {
                self.inner.settings.output_format = kind;
                Ok(())
            }
            None => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown output format: {name}"
            ))),
        }
    }

    /// 对应 Java `Configuration.setAutoEscapingPolicy(int)`
    /// 接受 "ON"/"OFF"/"DEFAULT"（大小写不敏感）。
    fn set_auto_escaping(&mut self, policy: String) -> PyResult<()> {
        match policy.to_lowercase().as_str() {
            "on" => {
                self.inner.settings.auto_escaping = AutoEscaping::On;
                Ok(())
            }
            "off" => {
                self.inner.settings.auto_escaping = AutoEscaping::Off;
                Ok(())
            }
            "default" => {
                self.inner.settings.auto_escaping = AutoEscaping::Default;
                Ok(())
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid auto escaping policy: {policy}"
            ))),
        }
    }

    /// 对应 Java `Configuration.setCFormat(CFormat)`
    /// 接受 "JavaScript or JSON"/"JavaScript"/"JSON"/"Java"/"legacy"/"XS"。
    fn set_c_format(&mut self, name: String) -> PyResult<()> {
        match CFormatKind::parse(&name) {
            Some(kind) => {
                self.inner.settings.c_format = kind;
                Ok(())
            }
            None => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown C format: {name}"
            ))),
        }
    }

    /// 对应 Java `Configuration.setURLEscapingCharset(String)`
    fn set_url_escaping_charset(&mut self, charset: String) {
        self.inner.settings.url_escaping_charset = charset;
    }

    /// 对应 Java `Configuration.setOutputEncoding(String)`
    /// 仅存储；process() 返回 str 语义不变（UTF-8 解码）。
    fn set_output_encoding(&mut self, encoding: String) {
        self.inner.settings.output_encoding = encoding;
    }

    // --- 解析期设置 ---

    /// 对应 Java `Configuration.setStrictSyntaxMode(boolean)`
    fn set_strict_syntax(&mut self, strict: bool) {
        self.inner.settings.strict_syntax = strict;
    }

    /// 对应 Java `Configuration.setWhitespaceStripping(boolean)`
    fn set_whitespace_stripping(&mut self, strip: bool) {
        self.inner.settings.whitespace_stripping = strip;
    }

    /// 对应 Java `Configuration.setClassicCompatible(boolean)`（Configurable 级）
    fn set_classic_compatible(&mut self, compat: bool) {
        self.inner.settings.classic_compatible = compat;
    }

    /// 对应 Java `Configuration.setDefaultEncoding(String)`（input_encoding 设置）
    fn set_input_encoding(&mut self, encoding: String) {
        self.inner.settings.input_encoding = Some(encoding);
    }

    // --- 行为设置 ---

    /// 对应 Java `Configurable.setFallbackOnNullLoopVariable(boolean)`
    fn set_fallback_on_null_loop_variable(&mut self, fallback: bool) {
        self.inner.settings.fallback_on_null_loop_variable = fallback;
    }

    /// 对应 Java `Configuration.setLocalizedLookup(boolean)`
    fn set_localized_lookup(&mut self, localized: bool) {
        self.inner.settings.localized_lookup = localized;
    }

    /// 对应 Java `Configuration.setTemplateExceptionHandler(TemplateExceptionHandler)`
    /// 接受 "rethrow"/"debug"/"html_debug"/"ignore"。
    fn set_template_exception_handler(&mut self, handler: String) -> PyResult<()> {
        match handler.to_lowercase().as_str() {
            "rethrow" | "debug" | "html_debug" | "ignore" => {
                self.inner.settings.template_exception_handler = handler;
                Ok(())
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid template exception handler: {handler}"
            ))),
        }
    }

    /// 对应 Java `Configuration.setLazyImports(boolean)`
    fn set_lazy_imports(&mut self, lazy: bool) {
        self.inner.set_lazy_imports(lazy);
    }

    /// 对应 Java `Configurable.setLazyAutoImports(Boolean)`
    /// 接受 True/False/None（None = 未设置 → 回退 lazyImports）。
    fn set_lazy_auto_imports(&mut self, lazy: Option<bool>) {
        self.inner.set_lazy_auto_imports(lazy);
    }

    /// 对应 Java `Configuration.setTemplateUpdateDelay(int)`
    fn set_delay(&mut self, delay: u64) {
        self.inner.settings.delay = delay;
    }

    // --- 模板查找 ---

    /// 对应 Java `Configuration.addAutoImport(String, String)`
    /// 可多次累积（同名覆盖，对齐 Java addAutoImports）。
    fn set_auto_import(&mut self, namespace_var_name: String, template_name: String) {
        self.inner
            .add_auto_import(&namespace_var_name, &template_name);
    }

    /// 对应 Java `Configuration.addAutoInclude(String)`
    /// 可多次累积（同名去重，对齐 Java addAutoIncludes）。
    fn set_auto_include(&mut self, template_name: String) {
        self.inner.add_auto_include(&template_name);
    }

    /// 对应 Java `Configuration.setTemplateLookupStrategy(TemplateLookupStrategy)`
    /// 当前仅支持 "default"（Default020300）。
    fn set_lookup_strategy(&mut self, name: String) -> PyResult<()> {
        match name.to_lowercase().as_str() {
            "default" => {
                self.inner.settings.lookup_strategy = LookupStrategyKind::Default020300;
                Ok(())
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown lookup strategy: {name}"
            ))),
        }
    }

    // --- Getter ---

    /// 对应 Java `Configuration.getLocale()`
    #[getter]
    fn get_locale(&self) -> String {
        self.inner.settings.locale.clone()
    }

    /// 对应 Java `Configuration.getOutputEncoding()`
    #[getter]
    fn get_output_encoding(&self) -> String {
        self.inner.settings.output_encoding.clone()
    }

    /// 对应 Java `Configuration.getNumberFormat()`
    #[getter]
    fn get_number_format(&self) -> String {
        self.inner.settings.number_format.clone()
    }
}

/// FmTemplate —— 对应 Java `freemarker.template.Template`
/// `#[pyclass(unsendable)]`：内部 `Rc<Template>`（非 Send），主线程专用。
#[pyclass(module = "freemarker", unsendable)]
pub struct FmTemplate {
    inner: Rc<Template>,
    wrapper: Arc<PyObjectWrapperInner>,
}

impl FmTemplate {
    /// 当前配置时区（naive datetime 的解释时区；docs/10 §2）
    fn tz(&self) -> Option<TzSetting> {
        Some(self.inner.configuration.settings.time_zone)
    }
}

#[pymethods]
impl FmTemplate {
    /// 对应 Java `Template.getName()`
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// 对应 Java `Template.process(dataModel, out)`（docs/10 §1：渲染入口统一返回 str）
    ///
    /// GIL 纪律（docs/10 §4）：本方法持有单次 GIL —— ① wrap 根数据模型；
    /// ② 核心引擎渲染（Py 模型 trait 方法内部经 Python::attach 获取，可重入零开销）；
    /// ③ 错误经 errors.rs 桥接为 FreeMarkerError（消息含 `[in template ...]` 定位）。
    /// v1 不使用 py.allow_threads()：纯文本输出段短暂，释放/重取 GIL 收益低
    /// （docs/10 §4「v1 简单持有」）。
    fn process(&self, py: Python<'_>, root: Py<PyAny>) -> PyResult<String> {
        // ① wrap 根（Java：process 内部经 objectWrapper.wrap(rootMap)）
        let root_model = match self.wrapper.wrap(py, root.bind(py), self.tz())? {
            Some(m) => m,
            None => TModel::nothing(),
        };
        // ② 渲染到内存缓冲（核心引擎不感知 GIL）
        let mut out = Vec::new();
        self.inner
            .process(root_model, &mut out)
            .map_err(template_error_to_pyerr)?;
        // ③ UTF-8 输出转 str（Java Writer 输出语义）
        String::from_utf8(out).map_err(|e| {
            FreeMarkerError::new_err(format!("template output is not valid UTF-8: {e}"))
        })
    }
}

/// Python 模块入口 —— 对应 Java `freemarker.ext.jython` 包的对外 API 面
/// （docs/10 §1）；FreeMarkerError 注册为模块级异常类（docs/10 §5）。
#[pymodule]
fn freemarker(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FmConfiguration>()?;
    m.add_class::<FmTemplate>()?;
    m.add_class::<PyObjectWrapper>()?;
    m.add_class::<bridge::TemplateModelAdapter>()?;
    m.add("FreeMarkerError", m.py().get_type::<FreeMarkerError>())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 测试：端到端渲染 + pyclass API 面 + 双向异常桥接（docs/10 §8 验收）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    fn eval(py: Python<'_>, code: &str) -> PyResult<Py<PyAny>> {
        let c = std::ffi::CString::new(code).unwrap();
        py.eval(c.as_c_str(), None, None).map(|b| b.unbind())
    }

    /// 渲染辅助：put_template + get_template + process
    fn render(cfg: &mut FmConfiguration, name: &str, src: &str, root: &str) -> PyResult<String> {
        Python::attach(|py| {
            cfg.put_template(name.to_string(), src.to_string());
            let tmpl = cfg.get_template(name.to_string())?;
            let root_obj = eval(py, root)?;
            tmpl.process(py, root_obj)
        })
    }

    /// helloworld（黄金套件核心用例）
    #[test]
    fn helloworld_renders() {
        let mut cfg = FmConfiguration::new();
        let out = render(
            &mut cfg,
            "hello.ftl",
            "Hello, ${name}!",
            "{'name': 'world'}",
        )
        .unwrap();
        assert_eq!(out, "Hello, world!");
    }

    /// 宏
    #[test]
    fn macro_renders() {
        let mut cfg = FmConfiguration::new();
        let out = render(
            &mut cfg,
            "m.ftl",
            "<#macro greet who>Hello, ${who}!</#macro><@greet who='FM'/>",
            "{}",
        )
        .unwrap();
        assert_eq!(out, "Hello, FM!");
    }

    /// list/dict 数据（#list 迭代 + 嵌套取值 + ?keys）
    #[test]
    fn list_and_dict_data() {
        let mut cfg = FmConfiguration::new();
        let out = render(
            &mut cfg,
            "d.ftl",
            "<#list items as i>${i};</#list>|<#list data?keys as k>${k}=${data[k]};</#list>",
            "{'items': [1, 2, 3], 'data': {'a': 1, 'b': 2}}",
        )
        .unwrap();
        assert_eq!(out, "1;2;3;|a=1;b=2;");
    }

    /// Python 函数作为模板方法（参数 unwrap → Python 侧 → 结果 wrap）
    #[test]
    fn python_function_as_template_method() {
        let mut cfg = FmConfiguration::new();
        let out = render(
            &mut cfg,
            "f.ftl",
            "${greet('world')}|${double(21)}",
            "{'greet': lambda n: 'Hello ' + n, 'double': lambda x: x * 2}",
        )
        .unwrap();
        assert_eq!(out, "Hello world|42");
    }

    /// 模板内构造的数据传入 Python 函数：hash → dict（unwrap 方向）
    #[test]
    fn python_function_receives_engine_hash_as_dict() {
        let mut cfg = FmConfiguration::new();
        let out = render(
            &mut cfg,
            "f2.ftl",
            "${first_key({'x': 1, 'y': 2})}",
            "{'first_key': lambda d: sorted(d.keys())[0]}",
        )
        .unwrap();
        assert_eq!(out, "x");
    }

    /// set_shared_variable（须在 get_template 之前，见 get_template 注释）
    #[test]
    fn shared_variable() {
        Python::attach(|py| {
            let mut cfg = FmConfiguration::new();
            cfg.set_shared_variable(py, "brand".to_string(), eval(py, "'FM'").unwrap())
                .unwrap();
            let out = render(&mut cfg, "s.ftl", "Powered by ${brand}", "{}").unwrap();
            assert_eq!(out, "Powered by FM");
        });
    }

    /// 模板错误 → FreeMarkerError，消息含模板名与定位（docs/10 §5）
    #[test]
    fn template_error_is_freemarker_error_with_template_name() {
        let mut cfg = FmConfiguration::new();
        let err = render(&mut cfg, "err.ftl", "Before ${missing} after", "{}").unwrap_err();
        Python::attach(|py| {
            assert!(err.is_instance_of::<FreeMarkerError>(py), "{err}");
        });
        let msg = err.to_string();
        assert!(msg.contains("missing"), "{msg}");
        assert!(msg.contains("err.ftl"), "{msg}");
        assert!(msg.contains("[in template"), "{msg}");
    }

    /// Python 异常（模板方法调用抛错）→ FreeMarkerError，消息含异常类型（docs/10 §5）
    #[test]
    fn python_exception_bridged_into_template_error() {
        let mut cfg = FmConfiguration::new();
        let err = render(&mut cfg, "boom.ftl", "${boom()}", "{'boom': lambda: 1 / 0}").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ZeroDivisionError"), "{msg}");
        assert!(msg.contains("division by zero"), "{msg}");
        // 模型错误定位附加模板名（渲染层 attach_location，docs/09 §2）
        assert!(msg.contains("boom.ftl"), "{msg}");
    }

    /// 缺失变量定位消息
    #[test]
    fn missing_variable_error() {
        let mut cfg = FmConfiguration::new();
        let err = render(&mut cfg, "nv.ftl", "${nope}", "{}").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("nope"), "{msg}");
        assert!(msg.contains("nv.ftl"), "{msg}");
    }

    /// Python 侧真实 API 路径（Py::new + call_method 参数转换）
    #[test]
    fn pyclass_api_path() {
        Python::attach(|py| {
            let cfg = Py::new(py, FmConfiguration::new()).unwrap();
            cfg.bind(py)
                .as_any()
                .call_method1("put_template", ("t.ftl", "Hi ${name}"))
                .unwrap();
            let tmpl = cfg
                .bind(py)
                .as_any()
                .call_method1("get_template", ("t.ftl",))
                .unwrap();
            let root = eval(py, "{'name': 'py'}").unwrap().into_bound(py);
            let out = tmpl.call_method1("process", (&root,)).unwrap();
            assert_eq!(out.extract::<String>().unwrap(), "Hi py");
            // getter 暴露 name
            let name = tmpl.getattr("name").unwrap();
            assert_eq!(name.extract::<String>().unwrap(), "t.ftl");
        });
    }

    /// 模板名不存在 → NotFound 错误
    #[test]
    fn template_not_found() {
        Python::attach(|py| {
            let cfg = FmConfiguration::new();
            let err = match cfg.get_template("no_such.ftl".to_string()) {
                Err(e) => e,
                Ok(_) => panic!("expected NotFound error"),
            };
            assert!(err.is_instance_of::<FreeMarkerError>(py), "{err}");
            assert!(err.to_string().contains("no_such.ftl"));
        });
    }

    /// PyObjectWrapper pyclass：attributes_shadow_items 读写（可构造参数）
    #[test]
    fn pyobject_wrapper_pyclass() {
        Python::attach(|py| {
            // 经 Python 侧类型构造（#[new] 在 Rust 侧为私有，Python 路径等价）
            let w = py
                .get_type::<PyObjectWrapper>()
                .call1((false, false))
                .unwrap();
            let any = w.as_any();
            assert!(!any
                .getattr("attributes_shadow_items")
                .unwrap()
                .extract::<bool>()
                .unwrap());
            any.setattr("attributes_shadow_items", true).unwrap();
            assert!(any
                .getattr("attributes_shadow_items")
                .unwrap()
                .extract::<bool>()
                .unwrap());
            // set_object_wrapper 接受 PyObjectWrapper 实例
            let mut cfg = FmConfiguration::new();
            cfg.set_object_wrapper(&w.extract::<PyRef<'_, PyObjectWrapper>>().unwrap());
            let out = render(
                &mut cfg,
                "w.ftl",
                "${c.name}|${c.y}",
                "{'c': {'name': 'n', 'y': 'v'}}",
            )
            .unwrap();
            // dict 的 keys() 属性不存在于实例 → 属性通道缺失后回退 item 通道
            assert_eq!(out, "n|v");
        });
    }

    /// 根数据模型传非 dict（int）：通用模型带 hash 角色（Java JythonModel 继承链），
    /// `${x}` 走 getattr→get_item 双通道，int 的 get_item 抛 TypeError → 模型错误
    #[test]
    fn root_non_dict_reports_model_error() {
        let mut cfg = FmConfiguration::new();
        let err = render(&mut cfg, "r.ftl", "${x}", "42").unwrap_err();
        assert!(err.to_string().contains("TypeError"), "{err}");
    }

    /// 大整数 + 数值算术渲染（BigInt wrap → FTL 算术 → 输出）
    #[test]
    fn bigint_arithmetic_renders() {
        let mut cfg = FmConfiguration::new();
        let out = render(&mut cfg, "big.ftl", "${big + 1}", "{'big': 2**70}").unwrap();
        // 核心引擎对 BigInt 输出带千分位分组（format_number "number" 格式）
        assert_eq!(out, "1,180,591,620,717,411,303,425");
    }

    /// 日期渲染（datetime wrap → ?datetime 内建输出）
    #[test]
    fn datetime_renders() {
        let mut cfg = FmConfiguration::new();
        let out = render(
            &mut cfg,
            "dt.ftl",
            "${d?datetime}",
            "{'d': __import__('datetime').datetime(2024, 1, 2, 3, 4, 5, tzinfo=__import__('datetime').timezone.utc)}",
        )
        .unwrap();
        // 默认 date_time_format = medium 风格
        assert_eq!(out, "Jan 2, 2024 3:04:05 AM");
    }

    /// 渲染结果可多模板复用（模板缓存路径）
    #[test]
    fn template_reusable_across_process_calls() {
        Python::attach(|py| {
            let mut cfg = FmConfiguration::new();
            cfg.put_template("r.ftl".to_string(), "Hi ${name}".to_string());
            let tmpl = cfg.get_template("r.ftl".to_string()).unwrap();
            let r1 = tmpl
                .process(py, eval(py, "{'name': 'a'}").unwrap())
                .unwrap();
            let r2 = tmpl
                .process(py, eval(py, "{'name': 'b'}").unwrap())
                .unwrap();
            assert_eq!(r1, "Hi a");
            assert_eq!(r2, "Hi b");
        });
    }

    #[allow(dead_code)]
    fn _unused(_: &PyDict) {}
}
