//! Python 对象模型家族 —— 对应 Java `freemarker.ext.jython.JythonModel` 及其子类
//! （JythonModel.java / JythonHashModel.java / JythonSequenceModel.java / JythonNumberModel.java）
//!
//! Rust 侧以单一结构 PyObjectModel + kind 枚举表达该继承链（docs/02 §4.1 角色槽位设计）：
//! - Number   → 对应 JythonNumberModel（TemplateNumberModel + JythonModel 基础角色）
//! - Hash     → 对应 JythonHashModel（另加 TemplateHashModelEx：keys() 委托 Python 方法）
//! - Sequence → 对应 JythonSequenceModel（另加 TemplateSequenceModel/CollectionModel）
//! - Generic  → 对应 JythonModel（scalar+boolean+hash+method 四角色）
//!
//! 基础角色（继承自 Java JythonModel，全 kind 共享）：
//! - as_string  = str(obj)         （JythonModel.getAsString，JythonModel.java:78-84）
//! - as_boolean = bool(obj)        （JythonModel.getAsBoolean，:66-73）
//! - get(key)   = getattr/get_item 双通道，按 attributes_shadow_items 定序（:94-120）
//! - is_empty   = len(obj) == 0    （JythonModel.isEmpty，:126-132）
//! - exec(args) = obj.__call__(...)（JythonModel.exec，:138-166，参数逐个 unwrap）
//!
//! GIL 纪律（docs/10 §4）：核心引擎不感知 GIL；本文件所有 trait 方法内部经
//! Python::attach 获取（可重入：渲染入口已持有 GIL 时零开销，见 attach 的
//! AttachGuard::Assumed 分支）；`Py<PyAny>` 满足 Send，可安全存入 TModel.internal。

use crate::errors;
use crate::wrapper::PyObjectWrapperInner;
use freemarker::core::TzSetting;
use freemarker::error::Result;
use freemarker::template::TModel;
use freemarker::template::{
    TemplateBooleanModel, TemplateCollectionModel, TemplateHashModel, TemplateHashModelEx,
    TemplateMethodModelEx, TemplateNumberModel, TemplateScalarModel, TemplateSequenceModel,
};
use freemarker::value::{DateType, DateValue, TNumber};
use num_bigint::BigInt;
use pyo3::exceptions::{PyAttributeError, PyIndexError, PyKeyError};
use pyo3::prelude::*;
use pyo3::types::{PyDateTime, PyFloat, PyTuple, PyTzInfoAccess};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// 模型种类 —— 对应 Java 模型类层次（JythonModelCache.create 的分派结果）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PyModelKind {
    /// 对应 JythonNumberModel
    Number,
    /// 对应 JythonHashModel
    Hash,
    /// 对应 JythonSequenceModel
    Sequence,
    /// 对应 JythonModel（通用）
    Generic,
}

/// Python 对象模型 —— 对应 Java `JythonModel` 家族
#[derive(Clone)]
pub(crate) struct PyObjectModel {
    pub obj: Py<PyAny>,
    pub wrapper: Arc<PyObjectWrapperInner>,
    pub kind: PyModelKind,
    /// Python 类名（错误消息用；Java getPythonClassName / getModelClass）
    pub class_name: String,
    /// naive datetime 的解释时区（docs/10 §2：naive → 按设置时区；None → UTC）
    pub tz: Option<TzSetting>,
}

impl PyObjectModel {
    /// 模型构造 —— 对应 JythonModelCache.create（JythonModelCache.java:56-99）的
    /// 角色槽位装配 + Java JythonModel 继承链的基础角色（scalar/boolean/hash/method）。
    /// `internal` 槽位承载原 Python 对象：unwrap 时身份还原（对应 Java
    /// AdapterTemplateModel.getAdaptedObject / WrapperTemplateModel.getWrappedObject）。
    fn build(
        wrapper: &Arc<PyObjectWrapperInner>,
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        tz: Option<TzSetting>,
        kind: PyModelKind,
        type_name: &'static str,
    ) -> PyResult<TModel> {
        let m = PyObjectModel {
            obj: obj.clone().unbind(),
            wrapper: wrapper.clone(),
            kind,
            class_name: py_class_name(py, obj),
            tz,
        };
        let rc = Rc::new(m);
        let mut tm = TModel {
            // JythonModel 基础角色
            scalar: Some(rc.clone()),
            boolean: Some(rc.clone()),
            hash: Some(rc.clone()),
            method: Some(rc.clone()),
            // 身份还原：unwrap 返回原对象（对应 Java AdapterTemplateModel）
            internal: Some(Rc::new(obj.clone().unbind())),
            type_name,
            ..TModel::nothing()
        };
        match kind {
            PyModelKind::Number => {
                tm.number = Some(rc);
                tm.kind = freemarker::template::ModelKind::Number;
            }
            PyModelKind::Hash => {
                tm.hash_ex = Some(rc);
                tm.kind = freemarker::template::ModelKind::Hash;
            }
            PyModelKind::Sequence => {
                tm.sequence = Some(rc.clone());
                tm.collection = Some(rc);
                tm.kind = freemarker::template::ModelKind::Sequence;
            }
            PyModelKind::Generic => {
                tm.kind = freemarker::template::ModelKind::Wrapped;
            }
        }
        Ok(tm)
    }

    /// 数值模型 —— 对应 JythonNumberModel（int/float）
    pub(crate) fn new_number(
        wrapper: &Arc<PyObjectWrapperInner>,
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        tz: Option<TzSetting>,
    ) -> PyResult<TModel> {
        Self::build(wrapper, py, obj, tz, PyModelKind::Number, "number")
    }

    /// 哈希模型 —— 对应 JythonHashModel（dict）
    pub(crate) fn new_hash(
        wrapper: &Arc<PyObjectWrapperInner>,
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        tz: Option<TzSetting>,
    ) -> PyResult<TModel> {
        Self::build(wrapper, py, obj, tz, PyModelKind::Hash, "extended_hash")
    }

    /// 序列模型 —— 对应 JythonSequenceModel（list/tuple）
    pub(crate) fn new_sequence(
        wrapper: &Arc<PyObjectWrapperInner>,
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        tz: Option<TzSetting>,
    ) -> PyResult<TModel> {
        Self::build(wrapper, py, obj, tz, PyModelKind::Sequence, "sequence")
    }

    /// 通用模型 —— 对应 JythonModel（callable 与其余对象）
    pub(crate) fn new_generic(
        wrapper: &Arc<PyObjectWrapperInner>,
        py: Python<'_>,
        obj: &Bound<'_, PyAny>,
        tz: Option<TzSetting>,
    ) -> PyResult<TModel> {
        Self::build(wrapper, py, obj, tz, PyModelKind::Generic, "python_object")
    }

    /// 在 GIL 内执行（PyErr → TemplateError::Model 桥接，errors.rs）
    /// 可重入：渲染入口持有 GIL 时（docs/10 §4）Python::attach 直接 Assumed，零开销。
    fn with_py<T>(
        &self,
        f: impl FnOnce(Python<'_>, &Bound<'_, PyAny>) -> PyResult<T>,
    ) -> Result<T> {
        Python::attach(|py| {
            let obj = self.obj.bind(py);
            f(py, obj).map_err(|e| errors::py_err_to_template_error(py, e))
        })
    }

    /// 成员查找 —— 对应 JythonModel.get :94-120。
    /// 两模式：
    /// - dict（PyModelKind::Hash）：直接 get_item —— docs/10 §2「SimpleHash 视图：
    ///   get_item 惰性」；Java 的 JythonHashModel 继承 JythonModel 的双通道查找，
    ///   会令 `d.keys` 命中 dict 的 keys 方法而非键 —— 设计文档明确按 SimpleHash
    ///   视图处理（记录于 §6 语义差异）；
    /// - 其余 kind：双通道 getattr/get_item，attributes_shadow_items=true（默认）
    ///   先 getattr 后 get_item；false 反序（对应 Java JythonModel.get）。
    ///   AttributeError/KeyError 视为"缺失"（Java __findattr__/__finditem__ 返回 null）；
    ///   其余 PyErr 为真实错误 → TemplateError::Model。两通道皆缺失 → Ok(None)
    ///   （Java wrap(null) 返回 null 模型）。
    fn lookup_member(&self, key: &str) -> Result<Option<TModel>> {
        self.with_py(|py, obj| {
            let v = if self.kind == PyModelKind::Hash {
                // dict：SimpleHash 视图（直接 get_item）
                try_getitem(py, obj, key)?
            } else {
                let attr_first = self.wrapper.attributes_shadow_items.load(Ordering::Relaxed);
                let first = if attr_first {
                    try_getattr(py, obj, key)?
                } else {
                    try_getitem(py, obj, key)?
                };
                if first.is_some() {
                    first
                } else if attr_first {
                    try_getitem(py, obj, key)?
                } else {
                    try_getattr(py, obj, key)?
                }
            };
            match v {
                // wrap 对 Python None 返回 Ok(None)（Java wrap(null) → null 模型）
                Some(v) => self.wrapper.wrap(py, v.bind(py), self.tz),
                None => Ok(None),
            }
        })
    }
}

/// getattr，AttributeError → Ok(None)（Java __findattr__ 返回 null 语义）
fn try_getattr(py: Python<'_>, obj: &Bound<'_, PyAny>, key: &str) -> PyResult<Option<Py<PyAny>>> {
    match obj.getattr(key) {
        Ok(v) => Ok(Some(v.unbind())),
        Err(e) if e.is_instance_of::<PyAttributeError>(py) => Ok(None),
        Err(e) => Err(e),
    }
}

/// get_item，KeyError → Ok(None)（Java __finditem__ 返回 null 语义）
fn try_getitem(py: Python<'_>, obj: &Bound<'_, PyAny>, key: &str) -> PyResult<Option<Py<PyAny>>> {
    match obj.get_item(key) {
        Ok(v) => Ok(Some(v.unbind())),
        Err(e) if e.is_instance_of::<PyKeyError>(py) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Python 类名（错误消息；Java getPythonClassName）
fn py_class_name(_py: Python<'_>, obj: &Bound<'_, PyAny>) -> String {
    obj.get_type()
        .name()
        .ok()
        .and_then(|n| n.to_str().ok().map(str::to_owned))
        .unwrap_or_else(|| "object".to_string())
}

/// 由 Python int/float 提取 TNumber —— 对应 JythonNumberModel.getAsNumber
/// （docs/10 §2：int 溢出 → 大整数 BigInt；float → Double）
fn extract_tnumber(_py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<TNumber> {
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(TNumber::from_i64(i));
    }
    if obj.is_instance_of::<PyFloat>() {
        return Ok(TNumber::Double(obj.extract::<f64>()?));
    }
    // 超出 i64 的 Python 大整数：str(n) → BigInt（docs/10 §2）
    let s = obj.str()?.to_string_lossy().into_owned();
    BigInt::from_str(&s)
        .map(TNumber::BigInt)
        .map_err(|_| pyo3::exceptions::PyValueError::new_err(format!("not an integer: {s}")))
}

// ---------------------------------------------------------------------------
// 角色 trait 实现（对应 Java JythonModel 家族实现的 TemplateModel 接口）
// ---------------------------------------------------------------------------

/// 对应 Java JythonModel.getAsString（str(obj)）
impl TemplateScalarModel for PyObjectModel {
    fn as_string(&self) -> Result<String> {
        self.with_py(|_py, obj| obj.str().map(|s| s.to_string_lossy().into_owned()))
    }
}

/// 对应 Java JythonModel.getAsBoolean（bool(obj)）
impl TemplateBooleanModel for PyObjectModel {
    fn as_boolean(&self) -> Result<bool> {
        self.with_py(|_py, obj| obj.is_truthy())
    }
}

/// 对应 Java JythonNumberModel.getAsNumber
impl TemplateNumberModel for PyObjectModel {
    fn as_number(&self) -> Result<TNumber> {
        self.with_py(extract_tnumber)
    }
}

/// 对应 Java JythonModel（TemplateHashModel 角色）
impl TemplateHashModel for PyObjectModel {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        self.lookup_member(key)
    }
    /// Java JythonModel.isEmpty：len(obj) == 0（JythonModel.java:126-132）
    fn is_empty(&self) -> Result<bool> {
        let n = self.with_py(|_py, obj| obj.len())?;
        Ok(n == 0)
    }
}

/// 对应 Java JythonHashModel（TemplateHashModelEx 角色；仅 dict 模型装配此槽位）
impl TemplateHashModelEx for PyObjectModel {
    /// Java JythonHashModel.size：__len__()
    fn size(&self) -> Result<usize> {
        self.with_py(|_py, obj| obj.len())
    }
    /// Java JythonHashModel.keys：__findattr__("keys")；py3 无 keySet（docs/10 §6.4），
    /// 失败消息对齐（含类名，JythonHashModel.java:83-97）
    fn keys(&self) -> Result<Vec<String>> {
        self.with_py(|py, obj| {
            let method = match obj.getattr("keys") {
                Ok(m) => m,
                Err(e) if e.is_instance_of::<PyAttributeError>(py) => {
                    // Java 回退 keySet（py2 遗留，py3 无）；此处仅保留回退以对齐失败消息
                    match obj.getattr("keySet") {
                        Ok(m) => m,
                        Err(_) => {
                            return Err(pyo3::exceptions::PyAttributeError::new_err(format!(
                                "'?keys' is not supported as there is no 'keys' nor 'keySet' attribute on an instance of {}",
                                self.class_name
                            )))
                        }
                    }
                }
                Err(e) => return Err(e),
            };
            let result = method.call0()?;
            let mut out = Vec::new();
            for item in result.try_iter()? {
                out.push(item?.str()?.to_string_lossy().into_owned());
            }
            Ok(out)
        })
    }
}

/// 对应 Java JythonSequenceModel（TemplateSequenceModel 角色；仅 list/tuple 装配）
impl TemplateSequenceModel for PyObjectModel {
    /// Java JythonSequenceModel.get(int)：__finditem__(index)；
    /// 越界返回 null（Java PySequence 语义）→ Rust 以 nothing 表达（缺失）
    fn get(&self, index: usize) -> Result<TModel> {
        self.with_py(|py, obj| match obj.get_item(index) {
            Ok(v) => self
                .wrapper
                .wrap(py, &v, self.tz)
                .map(|m| m.unwrap_or_else(TModel::nothing)),
            Err(e) if e.is_instance_of::<PyIndexError>(py) => Ok(TModel::nothing()),
            Err(e) => Err(e),
        })
    }
    /// Java JythonSequenceModel.size：__len__()
    fn size(&self) -> Result<usize> {
        self.with_py(|_py, obj| obj.len())
    }
}

/// 对应 Java JythonSequenceModel.iterator（JythonSequenceModel.java:80-95：
/// 0..size 索引迭代）
impl TemplateCollectionModel for PyObjectModel {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        let n = self.with_py(|_py, obj| obj.len())?;
        // Py<T> 的 Clone 要求线程已附加 → 在 attach 上下文内克隆
        let model = Python::attach(|_| self.clone());
        let mut i = 0usize;
        Ok(Box::new(std::iter::from_fn(move || {
            if i >= n {
                return None;
            }
            let idx = i;
            i += 1;
            // 显式 trait 分派（PyObjectModel 同时实现 TemplateHashModel 与
            // TemplateSequenceModel 的 get，此处取序列角色）
            Some(<PyObjectModel as TemplateSequenceModel>::get(&model, idx))
        })))
    }
}

/// 对应 Java JythonModel.exec（JythonModel.java:138-166）：
/// 参数逐个 unwrap 回 Python 对象后调用 __call__（0 参 / 1 参 / n 参统一以
/// 元组 call1 表达；Java 分支等价）；结果 wrap 回 TModel（null → nothing）。
impl TemplateMethodModelEx for PyObjectModel {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        self.with_py(|py, obj| {
            let mut pyargs = Vec::with_capacity(args.len());
            for a in &args {
                pyargs.push(self.wrapper.unwrap(py, a)?);
            }
            let tuple = PyTuple::new(py, &pyargs)?;
            let result = obj.call1(&tuple)?;
            self.wrapper
                .wrap(py, &result, self.tz)
                .map(|m| m.unwrap_or_else(TModel::nothing))
        })
    }
}

/// Python 对象 → DateValue —— docs/10 §2 日期矩阵（pyo3 扩展，Java 无直接对应）：
/// 带 tzinfo → FixedOffset（固定偏移直接提取；pytz 等动态时区经 utcoffset() 计算）；
/// naive → 按设置时区（tz 参数；None → UTC）。
pub(crate) fn datetime_to_date_value(
    _py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    tz: Option<TzSetting>,
) -> PyResult<DateValue> {
    use chrono::{DateTime, FixedOffset, NaiveDateTime};
    let dt = obj.cast::<PyDateTime>()?;
    if dt.get_tzinfo().is_some() {
        // 带 tzinfo：优先 chrono 直接提取（FixedOffset tzinfo）；失败（pytz 等）
        // 时经 datetime.utcoffset() 取该时刻偏移
        if let Ok(d) = obj.extract::<DateTime<FixedOffset>>() {
            return Ok(DateValue::new(d, DateType::DateTime));
        }
        let delta = obj.call_method0("utcoffset")?;
        if !delta.is_none() {
            let dur: chrono::Duration = delta.extract()?;
            if let Some(off) = FixedOffset::east_opt(dur.num_seconds() as i32) {
                let naive: NaiveDateTime = obj.extract()?;
                return Ok(DateValue::new(
                    DateTime::from_naive_utc_and_offset(naive, off),
                    DateType::DateTime,
                ));
            }
        }
        // utcoffset 不可用：按 naive 路径处理（罕见）
    }
    // naive：按设置时区解释（Java：DateModel 无时区问题；pyo3 扩展决策 docs/10 §2）
    // naive 视为目标时区的本地时间（offset_at 取该时刻偏移，from_local_datetime 定本地）
    let naive: NaiveDateTime = obj.extract()?;
    let off = match tz {
        Some(t) => t.offset_at(&naive),
        None => FixedOffset::east_opt(0).unwrap(),
    };
    use chrono::TimeZone as _;
    Ok(DateValue::new(
        off.from_local_datetime(&naive).single().unwrap(),
        DateType::DateTime,
    ))
}

/// Python date（无时间部分）→ DateValue（kind=Date）
pub(crate) fn date_to_date_value(
    _py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    tz: Option<TzSetting>,
) -> PyResult<DateValue> {
    use chrono::{FixedOffset, NaiveDate};
    let naive: NaiveDate = obj.extract()?;
    let midnight = naive.and_hms_opt(0, 0, 0).unwrap();
    let off = match tz {
        Some(t) => t.offset_at(&midnight),
        None => FixedOffset::east_opt(0).unwrap(),
    };
    use chrono::TimeZone as _;
    Ok(DateValue::new(
        off.from_local_datetime(&midnight).single().unwrap(),
        DateType::Date,
    ))
}

/// Python time（无日期部分）→ DateValue（kind=Time；日期部分取 1970-01-01，
/// 核心 time_format 只读时分秒，见 builtins/java_date_format.rs）
pub(crate) fn time_to_date_value(
    _py: Python<'_>,
    obj: &Bound<'_, PyAny>,
    tz: Option<TzSetting>,
) -> PyResult<DateValue> {
    use chrono::{FixedOffset, NaiveDate, NaiveTime};
    let naive_time: NaiveTime = obj.extract()?;
    let base = NaiveDate::from_ymd_opt(1970, 1, 1)
        .unwrap()
        .and_time(naive_time);
    let off = match tz {
        Some(t) => t.offset_at(&base),
        None => FixedOffset::east_opt(0).unwrap(),
    };
    use chrono::TimeZone as _;
    Ok(DateValue::new(
        off.from_local_datetime(&base).single().unwrap(),
        DateType::Time,
    ))
}
