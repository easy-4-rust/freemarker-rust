//! 对象包装 —— 对应 Java `freemarker.ext.jython.JythonWrapper`
//! - wrap：Python 对象 → TModel（对应 JythonWrapper.wrap + JythonModelCache.create 类型矩阵）
//! - unwrap：TModel → Python 对象（对应 JythonWrapper.unwrap :139-170 +
//!   TemplateModelToJythonAdapter 内部类）
//!
//! 注意：核心 crate 的 `freemarker::template::ObjectWrapper` trait 面向 DynValue
//! （Rust 动态值枚举，docs/06 §4.1），无法承载 `Py<PyAny>`；故此处定义独立的
//! wrap/unwrap 方法（对应 Java JythonWrapper implements ObjectWrapper 的职责，
//! 签名按 PyO3 侧调整，docs/10 §1）。核心引擎不感知 GIL —— wrap/unwrap 调用方
//! 必须已持有 GIL（py 参数即证据；模型内部经 Python::attach 获取，见 models.rs）。

use crate::bridge::TemplateModelAdapter;
use crate::errors;
use crate::models::{self, PyObjectModel};
use freemarker::core::TzSetting;
use freemarker::template::TModel;
use freemarker::value::{DateType, TNumber};
use num_bigint::BigInt;
use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyDate, PyDateTime, PyDict, PyFloat, PyInt, PyList, PyString, PyTime, PyTuple,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 包装器内部状态 —— 对应 Java JythonWrapper 字段
/// （attributesShadowItems :64、modelCache/useCache :62-74）
/// 布尔配置用 AtomicBool：Arc 共享（模型/适配器持同一 Arc），pyclass setter 需可变。
pub(crate) struct PyObjectWrapperInner {
    pub attributes_shadow_items: AtomicBool,
    pub use_cache: AtomicBool,
    /// 模型缓存 —— 对应 Java JythonModelCache（ModelCache：同一对象 → 同一模型）。
    /// 键 = PyObject 指针（缓存持强引用，指针不会复用）；v1 仅按需启用（默认 false，
    /// 与 Java useCache 默认一致）。
    cache: Mutex<HashMap<usize, (Py<PyAny>, TModel)>>,
}

impl PyObjectWrapperInner {
    /// 豁免 arc_with_non_send_sync：PyObjectWrapperInner 由 unsendable pyclass 持有，
    /// 实际仅主线程使用（docs/10 §4 GIL 纪律）；Arc 仅为结构一致性
    #[allow(clippy::arc_with_non_send_sync)]
    pub(crate) fn new(attributes_shadow_items: bool, use_cache: bool) -> Arc<Self> {
        Arc::new(PyObjectWrapperInner {
            attributes_shadow_items: AtomicBool::new(attributes_shadow_items),
            use_cache: AtomicBool::new(use_cache),
            cache: Mutex::new(HashMap::new()),
        })
    }
}

/// Python 侧可见包装器 —— 对应 Java `JythonWrapper`（可构造，配置 attributes_shadow_items）
/// `#[pyclass(unsendable)]`：缓存含 TModel（内部 Rc，非 Send），且生命周期绑定创建线程；
/// 与 FmConfiguration/FmTemplate 一致（docs/10 §2 核心约束注释）。
#[pyclass(module = "freemarker", unsendable)]
pub struct PyObjectWrapper {
    pub(crate) inner: Arc<PyObjectWrapperInner>,
}

#[pymethods]
impl PyObjectWrapper {
    /// 对应 Java `new JythonWrapper()` + setAttributesShadowItems/setUseCache
    #[new]
    #[pyo3(signature = (attributes_shadow_items=true, use_cache=false))]
    fn new(attributes_shadow_items: bool, use_cache: bool) -> Self {
        PyObjectWrapper {
            inner: PyObjectWrapperInner::new(attributes_shadow_items, use_cache),
        }
    }

    /// 对应 Java JythonWrapper.isAttributesShadowItems / setAttributesShadowItems
    #[getter]
    fn attributes_shadow_items(&self) -> bool {
        self.inner.attributes_shadow_items.load(Ordering::Relaxed)
    }

    #[setter]
    fn set_attributes_shadow_items(&mut self, v: bool) {
        self.inner
            .attributes_shadow_items
            .store(v, Ordering::Relaxed);
    }

    /// 对应 Java ModelCache.setUseCache / isUseCache
    #[getter]
    fn use_cache(&self) -> bool {
        self.inner.use_cache.load(Ordering::Relaxed)
    }

    #[setter]
    fn set_use_cache(&mut self, v: bool) {
        self.inner.use_cache.store(v, Ordering::Relaxed);
    }
}

impl PyObjectWrapperInner {
    /// Python 对象 → TModel —— 对应 JythonWrapper.wrap（JythonWrapper.java:110-114）：
    /// null → Ok(None)（Java wrap(null) 返回 null）。
    /// `tz`：naive datetime 的解释时区（docs/10 §2；None → UTC）。
    /// `self: &Arc<Self>`：模型/适配器共享同一 Arc（缓存与设置一致性）。
    pub(crate) fn wrap(
        self: &Arc<Self>,
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        tz: Option<TzSetting>,
    ) -> PyResult<Option<TModel>> {
        if obj.is_none() {
            return Ok(None);
        }
        let ptr = obj.as_ptr() as usize;
        if self.use_cache.load(Ordering::Relaxed) {
            if let Some((_, m)) = self
                .cache
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&ptr)
            {
                return Ok(Some(m.clone()));
            }
        }
        let m = self.wrap_fresh(py, obj, tz)?;
        if self.use_cache.load(Ordering::Relaxed) {
            self.cache
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(ptr, (obj.clone().unbind(), m.clone()));
        }
        Ok(Some(m))
    }

    /// 类型分派 —— 对应 JythonModelCache.create（JythonModelCache.java:56-99，
    /// docs/10 §2 表格）。判定顺序要点：
    /// - bool 先于 int（Python bool 是 int 子类）；
    /// - datetime 先于 date（Python datetime 是 date 子类）；
    /// - str 先于序列（Python str 是序列，但按 docs/10 §2 映射为 scalar）；
    /// - callable 与其余对象 → 通用模型（JythonModel 自带 method 角色，无需专门分支）。
    fn wrap_fresh(
        self: &Arc<Self>,
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        tz: Option<TzSetting>,
    ) -> PyResult<TModel> {
        if obj.is_instance_of::<PyBool>() {
            return Ok(TModel::from_boolean(obj.extract::<bool>()?));
        }
        if obj.is_instance_of::<PyInt>() {
            return PyObjectModel::new_number(self, py, obj, tz);
        }
        if obj.is_instance_of::<PyFloat>() {
            return PyObjectModel::new_number(self, py, obj, tz);
        }
        if obj.is_instance_of::<PyString>() {
            return Ok(TModel::from_scalar(obj.extract::<String>()?));
        }
        if obj.is_instance_of::<PyDateTime>() {
            return Ok(TModel::from_date(models::datetime_to_date_value(
                py, obj, tz,
            )?));
        }
        if obj.is_instance_of::<PyDate>() {
            return Ok(TModel::from_date(models::date_to_date_value(py, obj, tz)?));
        }
        if obj.is_instance_of::<PyTime>() {
            return Ok(TModel::from_date(models::time_to_date_value(py, obj, tz)?));
        }
        if obj.is_instance_of::<PyDict>() {
            return PyObjectModel::new_hash(self, py, obj, tz);
        }
        if obj.is_instance_of::<PyList>() || obj.is_instance_of::<PyTuple>() {
            return PyObjectModel::new_sequence(self, py, obj, tz);
        }
        // 其余（含 callable）→ 通用模型（对应 JythonModel）
        PyObjectModel::new_generic(self, py, obj, tz)
    }

    /// TModel → Python 对象 —— 对应 JythonWrapper.unwrap（JythonWrapper.java:139-170，
    /// docs/10 §3 表格）：
    /// ① internal 槽位（Python 对象包装的模型）→ 原对象身份还原（对应 Java
    ///    AdapterTemplateModel.getAdaptedObject，JythonWrapper.java:140-146）；
    /// ② nothing → None；③ scalar → str；④ number → int/float（整数→int）；
    /// ⑤ boolean → bool；⑥ date → datetime；⑦ hash → dict（键值对迭代；
    ///    无 hash_ex（不可枚举）时回退通用适配器）；⑧ sequence → list；
    /// ⑨ 其余（method/组合）→ 通用适配器 TemplateModelAdapter（bridge.rs）。
    pub(crate) fn unwrap(self: &Arc<Self>, py: Python<'_>, model: &TModel) -> PyResult<Py<PyAny>> {
        if let Some(inner) = model.internal::<Py<PyAny>>() {
            return Ok(inner.clone_ref(py).into_any());
        }
        if model.is_nothing() {
            return Ok(py.None());
        }
        if let Some(s) = &model.scalar {
            let v = s.as_string().map_err(errors::template_error_to_pyerr)?;
            return Ok(PyString::new(py, &v).into_any().unbind());
        }
        if let Some(n) = &model.number {
            let v = n.as_number().map_err(errors::template_error_to_pyerr)?;
            return unwrap_number(py, &v);
        }
        if let Some(b) = &model.boolean {
            let v = b.as_boolean().map_err(errors::template_error_to_pyerr)?;
            return Ok(PyBool::new(py, v).to_owned().into_any().unbind());
        }
        if let Some(d) = &model.date {
            let v = d.as_date().map_err(errors::template_error_to_pyerr)?;
            return unwrap_date(py, &v);
        }
        // hash → dict（docs/10 §3：键值对迭代；Java 侧用适配器，Rust 决策取 dict 直转，
        // 无 hash_ex（无法枚举键）时回退适配器）
        if model.hash_ex.is_some() {
            let ex = model.hash_ex.clone().ok_or_else(|| {
                errors::template_error_to_pyerr(freemarker::error::TemplateError::misc(
                    "hash_ex slot missing",
                ))
            })?;
            let keys = ex.keys().map_err(errors::template_error_to_pyerr)?;
            let dict = PyDict::new(py);
            for k in keys {
                if let Some(v) = ex.get(&k).map_err(errors::template_error_to_pyerr)? {
                    dict.set_item(&k, self.unwrap(py, &v)?)?;
                }
            }
            return Ok(dict.into_any().unbind());
        }
        if model.hash.is_some() {
            // 可 get 但不可枚举的哈希 → 通用适配器（对应 Java TemplateModelToJythonAdapter）
            return self.unwrap_adapter(py, model);
        }
        if model.sequence.is_some() {
            let seq = model.sequence.clone().ok_or_else(|| {
                errors::template_error_to_pyerr(freemarker::error::TemplateError::misc(
                    "sequence slot missing",
                ))
            })?;
            let size = seq.size().map_err(errors::template_error_to_pyerr)?;
            let list = PyList::empty(py);
            for i in 0..size {
                let item = seq.get(i).map_err(errors::template_error_to_pyerr)?;
                list.append(self.unwrap(py, &item)?)?;
            }
            return Ok(list.into_any().unbind());
        }
        // method/组合 → 通用适配器
        self.unwrap_adapter(py, model)
    }

    /// 通用适配器构造 —— 对应 Java `new TemplateModelToJythonAdapter(model)`
    fn unwrap_adapter(self: &Arc<Self>, py: Python<'_>, model: &TModel) -> PyResult<Py<PyAny>> {
        Py::new(
            py,
            TemplateModelAdapter {
                model: model.clone(),
                wrapper: self.clone(),
            },
        )
        .map(|o| o.into_any())
    }
}

/// TNumber → Python int/float —— 对应 JythonWrapper.unwrap 数值分支
/// （JythonWrapper.java:154-166；BigDecimal 整数化后按 int；BigInteger → PyLong）
fn unwrap_number(py: Python<'_>, n: &TNumber) -> PyResult<Py<PyAny>> {
    match n {
        TNumber::Int(v) => Ok((*v as i64).into_pyobject(py)?.into_any().unbind()),
        TNumber::Long(v) => Ok(v.into_pyobject(py)?.into_any().unbind()),
        TNumber::BigInt(v) => Ok(v.into_pyobject(py)?.into_any().unbind()),
        TNumber::Float(v) => Ok(PyFloat::new(py, *v as f64).into_any().unbind()),
        TNumber::Double(v) => Ok(PyFloat::new(py, *v).into_any().unbind()),
        // Decimal：整数 → int（Java OptimizerUtil.optimizeNumberRepresentation 后
        // BigInteger → PyLong）；非整数 → float（docs/10 §6.2 决策 D5）
        TNumber::Decimal(v) => {
            if v.is_integer() {
                let bi: BigInt =
                    v.to_string()
                        .parse()
                        .map_err(|e: num_bigint::ParseBigIntError| {
                            errors::template_error_to_pyerr(freemarker::error::TemplateError::misc(
                                format!("invalid decimal integer: {e}"),
                            ))
                        })?;
                Ok(bi.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(PyFloat::new(py, v.to_string().parse().unwrap_or(f64::NAN))
                    .into_any()
                    .unbind())
            }
        }
    }
}

/// DateValue → Python datetime —— docs/10 §3 日期方向（pyo3 扩展；Java 无对应）。
/// DateTime/Date/Time 三种 kind 分别构造 datetime/date/time（Time 无 tzinfo）。
fn unwrap_date(py: Python<'_>, d: &freemarker::value::DateValue) -> PyResult<Py<PyAny>> {
    use chrono::{Datelike, Timelike};
    let naive = d.dt.naive_local();
    match d.kind {
        DateType::DateTime => {
            // chrono 特性：DateTime<FixedOffset> → PyDateTime（含 tzinfo）
            let obj = d.dt.into_pyobject(py)?;
            Ok(obj.into_any().unbind())
        }
        DateType::Date => {
            let date =
                pyo3::types::PyDate::new(py, naive.year(), naive.month() as u8, naive.day() as u8)?;
            Ok(date.into_any().unbind())
        }
        DateType::Time => {
            let time = pyo3::types::PyTime::new(
                py,
                naive.hour() as u8,
                naive.minute() as u8,
                naive.second() as u8,
                naive.and_utc().timestamp_subsec_micros(),
                None,
            )?;
            Ok(time.into_any().unbind())
        }
        DateType::Unknown => {
            // 未知类型按 datetime 输出（保守选择）
            let obj = d.dt.into_pyobject(py)?;
            Ok(obj.into_any().unbind())
        }
    }
}

// ---------------------------------------------------------------------------
// 测试：wrap/unwrap 类型矩阵（docs/10 §8 验收 2/3）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::TemplateModelAdapter;
    use crate::models::datetime_to_date_value;
    use freemarker::core::TzSetting;
    use freemarker::error::Result;
    use freemarker::template::{SimpleHash, TemplateMethodModelEx};
    use freemarker::value::{DateType, TNumber};
    use indexmap::IndexMap;
    use pyo3::types::{PyDateAccess, PyDict, PyList, PyModule, PyTzInfoAccess};
    use std::rc::Rc;

    fn wrapper() -> Arc<PyObjectWrapperInner> {
        PyObjectWrapperInner::new(true, false)
    }

    /// 求值 Python 表达式（测试辅助）
    fn eval(py: Python<'_>, code: &str) -> PyResult<Py<PyAny>> {
        let c = std::ffi::CString::new(code).unwrap();
        py.eval(c.as_c_str(), None, None).map(|b| b.unbind())
    }

    /// 定义测试类并实例化（py.eval 只接受表达式，类定义走模块编译）
    fn make_class(py: Python<'_>, code: &str) -> PyResult<Py<PyAny>> {
        let module = PyModule::from_code(py, cstr(code).as_c_str(), c"test", c"test")?;
        let cls = module.getattr("C")?;
        cls.call0().map(|b| b.unbind())
    }

    fn cstr(s: &str) -> std::ffi::CString {
        std::ffi::CString::new(s).unwrap()
    }

    // -----------------------------------------------------------------
    // wrap 矩阵
    // -----------------------------------------------------------------

    /// dict → hash 视图：get 惰性、缺失键 → None、hash_ex keys 委托 Python keys()
    #[test]
    fn wrap_dict_is_lazy_hash() {
        Python::attach(|py| {
            let w = wrapper();
            let obj = eval(py, "{'name': 'world', 'n': 42, 'nested': {'k': 1}}").unwrap();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            assert!(m.is_hash());
            assert!(m.is_hash_ex());
            let h = m.get_hash().unwrap();
            assert_eq!(
                h.get("name").unwrap().unwrap().get_scalar().unwrap(),
                "world"
            );
            assert_eq!(
                h.get("n").unwrap().unwrap().get_number().unwrap(),
                TNumber::Int(42)
            );
            // 嵌套 dict 也按 hash 包装
            let nested = h.get("nested").unwrap().unwrap();
            assert!(nested.is_hash());
            // 缺失键 → None（Java 返回 null 模型）
            assert!(h.get("missing").unwrap().is_none());
            assert!(!h.is_empty().unwrap());
            // ?keys 通道：委托 Python keys()
            let ex = m.hash_ex.as_ref().unwrap().clone();
            assert_eq!(ex.keys().unwrap(), vec!["name", "n", "nested"]);
            assert_eq!(ex.size().unwrap(), 3);
        });
    }

    /// list/tuple → sequence：get(i) 惰性索引、越界 → nothing、迭代器
    #[test]
    fn wrap_list_and_tuple_are_sequences() {
        Python::attach(|py| {
            let w = wrapper();
            let obj = eval(py, "[10, 'a', True]").unwrap();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            assert!(m.is_sequence());
            assert!(m.is_collection());
            let s = m.get_sequence().unwrap();
            assert_eq!(s.size().unwrap(), 3);
            assert_eq!(s.get(0).unwrap().get_number().unwrap(), TNumber::Int(10));
            assert_eq!(s.get(1).unwrap().get_scalar().unwrap(), "a");
            assert!(s.get(2).unwrap().is_boolean());
            // 越界 → nothing（Java PySequence.__finditem__ 越界返回 null）
            assert!(s.get(99).unwrap().is_nothing());
            // 迭代（JythonSequenceModel.iterator）
            let items: Vec<TModel> = m
                .collection
                .as_ref()
                .unwrap()
                .iterator()
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].get_number().unwrap(), TNumber::Int(10));
            assert_eq!(items[1].get_scalar().unwrap(), "a");
            // tuple 同样按序列包装
            let obj = eval(py, "(1, 2)").unwrap();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            assert!(m.is_sequence());
            assert_eq!(m.get_sequence().unwrap().size().unwrap(), 2);
        });
    }

    /// int（含大整数）/float/str/bool/None 标量矩阵
    #[test]
    fn wrap_scalar_matrix() {
        Python::attach(|py| {
            let w = wrapper();
            let m = w
                .wrap(py, eval(py, "42").unwrap().bind(py), None)
                .unwrap()
                .unwrap();
            assert_eq!(m.get_number().unwrap(), TNumber::Int(42));
            // 超出 i64 → 大整数（docs/10 §2 溢出分支）
            let m = w
                .wrap(py, eval(py, "2**70").unwrap().bind(py), None)
                .unwrap()
                .unwrap();
            assert!(matches!(m.get_number().unwrap(), TNumber::BigInt(_)));
            let m = w
                .wrap(py, eval(py, "3.5").unwrap().bind(py), None)
                .unwrap()
                .unwrap();
            assert!(matches!(m.get_number().unwrap(), TNumber::Double(v) if v == 3.5));
            let m = w
                .wrap(py, eval(py, "'hi'").unwrap().bind(py), None)
                .unwrap()
                .unwrap();
            assert!(m.is_scalar());
            assert_eq!(m.get_scalar().unwrap(), "hi");
            // bool 先于 int（Python bool 是 int 子类）
            let m = w
                .wrap(py, eval(py, "True").unwrap().bind(py), None)
                .unwrap()
                .unwrap();
            assert!(m.is_boolean());
            assert!(m.get_boolean().unwrap());
            assert!(!m.is_number());
            // None → Ok(None)（Java wrap(null)）
            let m = w
                .wrap(py, eval(py, "None").unwrap().bind(py), None)
                .unwrap();
            assert!(m.is_none());
        });
    }

    /// datetime：带 tzinfo → FixedOffset；naive → 按设置时区（docs/10 §2）
    #[test]
    fn wrap_datetime_aware_and_naive() {
        Python::attach(|py| {
            use chrono::Timelike as _;
            let w = wrapper();
            let obj = eval(
                py,
                "__import__('datetime').datetime(2024, 1, 2, 3, 4, 5, tzinfo=__import__('datetime').timezone.utc)",
            )
            .unwrap();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            let d = m.get_date().unwrap();
            assert_eq!(d.kind, DateType::DateTime);
            assert_eq!(d.dt.offset().local_minus_utc(), 0);
            assert_eq!(d.dt.hour(), 3);
            // naive → 按 tz 参数（GMT+02:00）
            let tz: TzSetting = "GMT+02:00".parse().unwrap();
            let obj = eval(py, "__import__('datetime').datetime(2024, 1, 2, 3, 4, 5)").unwrap();
            let m = w.wrap(py, obj.bind(py), Some(tz)).unwrap().unwrap();
            let d = m.get_date().unwrap();
            assert_eq!(d.dt.offset().local_minus_utc(), 2 * 3600);
            assert_eq!(d.dt.hour(), 3);
            // date-only → kind=Date
            let obj = eval(py, "__import__('datetime').date(2024, 1, 2)").unwrap();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            assert_eq!(m.get_date().unwrap().kind, DateType::Date);
        });
    }

    /// callable → 通用模型（method 角色）：exec 参数逐个 unwrap 后调用
    #[test]
    fn wrap_callable_is_method() {
        Python::attach(|py| {
            let w = wrapper();
            let obj = eval(py, "lambda x: 'hi ' + str(x)").unwrap();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            assert!(m.is_method());
            assert!(m.is_scalar());
            assert!(m.is_boolean());
            let method = m.get_method().unwrap();
            let out = method
                .exec(vec![TModel::from_scalar("bob".into())])
                .unwrap();
            assert_eq!(out.get_scalar().unwrap(), "hi bob");
            // 返回 None → nothing
            let obj = eval(py, "lambda: None").unwrap();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            let out = m.get_method().unwrap().exec(vec![]).unwrap();
            assert!(out.is_nothing());
        });
    }

    /// 自定义类 → 通用模型：getattr/getitem 双通道 × attributes_shadow_items 两模式
    #[test]
    fn wrap_custom_class_dual_channel() {
        Python::attach(|py| {
            // x 为属性；__getitem__ 对任意键返回 'item:<key>'
            let obj = make_class(
                py,
                "class C:\n    x = 'attr'\n    def __getitem__(self, k):\n        if k == 'missing':\n            raise KeyError(k)\n        return 'item:' + str(k)\n    def __str__(self):\n        return 'C!'\n    def __len__(self):\n        return 2\n",
            )
            .unwrap();
            // attributes_shadow_items=true：getattr 优先
            let w = wrapper();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            let h = m.get_hash().unwrap();
            assert_eq!(h.get("x").unwrap().unwrap().get_scalar().unwrap(), "attr");
            assert_eq!(h.get("y").unwrap().unwrap().get_scalar().unwrap(), "item:y");
            // 两通道皆缺失 → None（__getitem__ 对 'missing' 抛 KeyError）
            assert!(h.get("missing").unwrap().is_none());
            // 基础角色：scalar=str()、boolean=bool()（len 2 → True）、is_empty=False
            assert_eq!(m.get_scalar().unwrap(), "C!");
            assert!(m.get_boolean().unwrap());
            assert!(!h.is_empty().unwrap());
            // attributes_shadow_items=false：getitem 优先
            let w2 = PyObjectWrapperInner::new(false, false);
            let m = w2.wrap(py, obj.bind(py), None).unwrap().unwrap();
            let h = m.get_hash().unwrap();
            assert_eq!(h.get("x").unwrap().unwrap().get_scalar().unwrap(), "item:x");
        });
    }

    /// 属性 getter 抛非 AttributeError → TemplateError::Model（异常桥接）
    #[test]
    fn wrap_attr_error_propagates_as_model_error() {
        Python::attach(|py| {
            let obj = make_class(
                py,
                "class C:\n    @property\n    def boom(self):\n        raise ValueError('attr exploded')\n",
            )
            .unwrap();
            let w = wrapper();
            let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            let h = m.get_hash().unwrap();
            let err = h.get("boom").unwrap_err();
            match err {
                freemarker::error::TemplateError::Model { message } => {
                    assert!(message.contains("ValueError"), "{message}");
                    assert!(message.contains("attr exploded"), "{message}");
                }
                other => panic!("expected Model error, got {other:?}"),
            }
        });
    }

    /// use_cache=true：同一对象 wrap 返回同一模型（对应 Java ModelCache）
    #[test]
    fn wrap_cache_returns_same_model() {
        Python::attach(|py| {
            let w = PyObjectWrapperInner::new(true, true);
            let obj = eval(py, "{'a': 1}").unwrap();
            let m1 = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            let m2 = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
            assert!(Rc::ptr_eq(
                &m1.hash.as_ref().unwrap().clone(),
                &m2.hash.as_ref().unwrap().clone()
            ));
        });
    }

    // -----------------------------------------------------------------
    // unwrap 矩阵
    // -----------------------------------------------------------------

    /// scalar → str；number → int/float；boolean → bool；nothing → None
    #[test]
    fn unwrap_scalar_number_boolean_nothing() {
        Python::attach(|py| {
            let w = wrapper();
            let out = w.unwrap(py, &TModel::from_scalar("hi".into())).unwrap();
            assert_eq!(out.bind(py).extract::<String>().unwrap(), "hi");
            let out = w
                .unwrap(py, &TModel::from_number(TNumber::Int(42)))
                .unwrap();
            assert_eq!(out.bind(py).extract::<i64>().unwrap(), 42);
            let out = w
                .unwrap(py, &TModel::from_number(TNumber::Long(1 << 40)))
                .unwrap();
            assert_eq!(out.bind(py).extract::<i64>().unwrap(), 1 << 40);
            let out = w
                .unwrap(py, &TModel::from_number(TNumber::Double(3.5)))
                .unwrap();
            assert_eq!(out.bind(py).extract::<f64>().unwrap(), 3.5);
            // Decimal 整数 → int（docs/10 §6.2 决策 D5）
            let out = w
                .unwrap(
                    py,
                    &TModel::from_number(TNumber::Decimal(bigdecimal::BigDecimal::from(7))),
                )
                .unwrap();
            assert_eq!(out.bind(py).extract::<i64>().unwrap(), 7);
            let out = w.unwrap(py, &TModel::from_boolean(true)).unwrap();
            assert!(out.bind(py).extract::<bool>().unwrap());
            let out = w.unwrap(py, &TModel::nothing()).unwrap();
            assert!(out.bind(py).is_none());
        });
    }

    /// date → datetime（DateTime kind 带 tzinfo）
    #[test]
    fn unwrap_date_to_datetime() {
        Python::attach(|py| {
            let w = wrapper();
            let dv = datetime_to_date_value(
                py,
                eval(py, "__import__('datetime').datetime(2024, 5, 6, 7, 8, 9, tzinfo=__import__('datetime').timezone.utc)").unwrap().bind(py),
                None,
            )
            .unwrap();
            let out = w.unwrap(py, &TModel::from_date(dv)).unwrap();
            let dt = out.bind(py).cast::<PyDateTime>().unwrap();
            assert_eq!(dt.get_year(), 2024);
            assert_eq!(dt.get_month(), 5);
            assert!(dt.get_tzinfo().is_some());
        });
    }

    /// hash → dict（键值对迭代）；sequence → list
    #[test]
    fn unwrap_hash_to_dict_and_sequence_to_list() {
        Python::attach(|py| {
            let w = wrapper();
            let mut map = IndexMap::new();
            map.insert("a".to_string(), TModel::from_scalar("1".into()));
            map.insert("b".to_string(), TModel::from_number(TNumber::Int(2)));
            let out = w.unwrap(py, &TModel::from_hash(map)).unwrap();
            let d = out.bind(py).cast::<PyDict>().unwrap();
            assert_eq!(d.len(), 2);
            assert_eq!(
                d.get_item("a")
                    .unwrap()
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "1"
            );
            // sequence → list
            let seq = TModel::from_sequence(vec![
                TModel::from_scalar("x".into()),
                TModel::from_number(TNumber::Int(1)),
            ]);
            let out = w.unwrap(py, &seq).unwrap();
            let l = out.bind(py).cast::<PyList>().unwrap();
            assert_eq!(l.len(), 2);
            assert_eq!(l.get_item(0).unwrap().extract::<String>().unwrap(), "x");
        });
    }

    /// 身份还原：wrap 的 Python 对象 unwrap 后是同一对象（对应 Java
    /// AdapterTemplateModel.getAdaptedObject）
    #[test]
    fn unwrap_identity_restores_original_object() {
        Python::attach(|py| {
            let w = wrapper();
            for code in ["{'k': 1}", "[1, 2]", "42", "'s'", "lambda: 1"] {
                let obj = eval(py, code).unwrap();
                let m = w.wrap(py, obj.bind(py), None).unwrap().unwrap();
                let out = w.unwrap(py, &m).unwrap();
                assert!(out.bind(py).is(obj.bind(py)), "identity lost for {code}");
            }
        });
    }

    /// method → 通用适配器；不可枚举 hash → 通用适配器
    #[test]
    fn unwrap_method_and_hash_only_to_adapter() {
        Python::attach(|py| {
            let w = wrapper();
            struct MethodStub;
            impl TemplateMethodModelEx for MethodStub {
                fn exec(&self, _args: Vec<TModel>) -> Result<TModel> {
                    Ok(TModel::from_scalar("stub".into()))
                }
            }
            let out = w.unwrap(py, &TModel::from_method(MethodStub)).unwrap();
            assert!(out.bind(py).is_instance_of::<TemplateModelAdapter>());
            // 仅 hash 角色（无 hash_ex）→ 适配器（不可枚举键）
            let mut tm = TModel::nothing();
            tm.kind = freemarker::template::ModelKind::Hash;
            tm.hash = Some(Rc::new(SimpleHash(IndexMap::new())));
            tm.type_name = "hash";
            let out = w.unwrap(py, &tm).unwrap();
            assert!(out.bind(py).is_instance_of::<TemplateModelAdapter>());
        });
    }
}
