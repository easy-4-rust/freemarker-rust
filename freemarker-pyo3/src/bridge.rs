//! 通用适配器 —— 对应 Java `JythonWrapper.TemplateModelToJythonAdapter`（内部类，
//! JythonWrapper.java:172-276）：把任意 TModel 暴露为 Python 对象，
//! __getitem__/__call__/__len__/__bool__ 按角色分派。
//!
//! unwrap 方向（docs/10 §3）：method/组合模型 → 本适配器；hash/sequence 在
//! 不可枚举（无 hash_ex / 无 sequence 角色）时也回退到本适配器。
//! `#[pyclass(unsendable)]`：内部 TModel 含 Rc（非 Send），与 FmConfiguration 同约束。

use crate::errors;
use crate::wrapper::PyObjectWrapperInner;
use freemarker::core::Environment;
use freemarker::template::{Configuration, TModel, Template};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyInt, PyTuple};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// 通用适配器 —— 对应 Java TemplateModelToJythonAdapter（JythonWrapper.java:172-276）
#[pyclass(module = "freemarker", unsendable)]
pub struct TemplateModelAdapter {
    pub(crate) model: TModel,
    pub(crate) wrapper: Arc<PyObjectWrapperInner>,
}

#[pymethods]
impl TemplateModelAdapter {
    /// 对应 Java TemplateModelAdapter.getTemplateModel（TemplateModelAdapter.java:34-37）
    #[getter]
    fn template_model(&self) -> String {
        // Rust 侧 TModel 无 Java 对象可返回，给出类型描述（调试辅助）
        self.model.type_name.to_string()
    }

    /// 对应 Java __finditem__(PyObject key)（JythonWrapper.java:186-215）：
    /// int（不含 bool，Python bool 是 int 子类 —— Java 中 PyBoolean 非 PyInteger，
    /// 走字符串路径）→ sequence.get(i)；其余 → 键字符串化后 hash.get(key)。
    /// 非 hash/sequence 模型 → TypeError（Java `Py.TypeError(...)`）。
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let is_int = key.is_instance_of::<PyInt>() && !key.is_instance_of::<PyBool>();
        if is_int {
            let idx: i64 = key.extract()?;
            let idx = usize::try_from(idx).map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err(format!(
                    "index must be non-negative, got {idx}"
                ))
            })?;
            let seq = self.model.sequence.clone().ok_or_else(|| {
                pyo3::exceptions::PyTypeError::new_err(format!(
                    "item lookup on non-sequence model ({})",
                    self.model.type_name
                ))
            })?;
            let item = seq.get(idx).map_err(errors::template_error_to_pyerr)?;
            return self.wrapper.unwrap(py, &item);
        }
        // 键字符串化（Java `key.toString()`，JythonWrapper.java:191）
        let key_str = key.str()?.to_string_lossy().into_owned();
        let h = self.model.hash.clone().ok_or_else(|| {
            pyo3::exceptions::PyTypeError::new_err(format!(
                "item lookup on non-hash model ({})",
                self.model.type_name
            ))
        })?;
        match h.get(&key_str).map_err(errors::template_error_to_pyerr)? {
            Some(m) => self.wrapper.unwrap(py, &m),
            None => Ok(py.None()),
        }
    }

    /// 对应 Java __call__(args, keywords)（JythonWrapper.java:217-237）：
    /// 参数逐个 wrap 回 TModel 后 exec；结果 unwrap 返回。
    /// Java 忽略 keywords（参数签名含但未使用）—— 此处对齐（v1）。
    /// `#[pyo3(signature = (*args, **kwargs))]`：收集全部位置参数为元组
    /// （pyo3 0.29 的 __call__ 特例签名，见 pyo3 guide class/call.md）
    #[pyo3(signature = (*args, **kwargs))]
    fn __call__(
        &self,
        py: Python<'_>,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Py<PyAny>> {
        // Java 忽略 keywords（JythonWrapper.__call__ 的 keywords 参数未使用）—— 对齐
        let _kwargs = kwargs;
        let method = self.model.method.clone().ok_or_else(|| {
            pyo3::exceptions::PyTypeError::new_err(format!(
                "call of non-method model ({})",
                self.model.type_name
            ))
        })?;
        let mut models = Vec::with_capacity(args.len());
        for a in args.iter() {
            match self.wrapper.wrap(py, &a, None)? {
                Some(m) => models.push(m),
                None => models.push(TModel::nothing()),
            }
        }
        // Python 侧直接调用方法模型：无渲染上下文（Java ThreadLocal env 为
        // null 时同样不可用）——构造最小空环境供 exec 使用（对应 Java
        // JythonWrapper 渲染期间调用时 ThreadLocal env 存在的场景）
        let mut sink = Vec::new();
        let cfg = Rc::new(Configuration::new());
        let t = Template::new("python-call".to_string(), Vec::new(), HashMap::new(), cfg);
        let mut env = Environment::new(&t, TModel::nothing(), &mut sink);
        let result = method
            .exec(&mut env, models)
            .map_err(errors::template_error_to_pyerr)?;
        self.wrapper.unwrap(py, &result)
    }

    /// 对应 Java __len__()（JythonWrapper.java:240-254）：sequence.size() /
    /// hash_ex.size()；其余角色返回 0
    fn __len__(&self) -> PyResult<usize> {
        if let Some(seq) = &self.model.sequence {
            return seq.size().map_err(errors::template_error_to_pyerr);
        }
        if let Some(ex) = &self.model.hash_ex {
            return ex.size().map_err(errors::template_error_to_pyerr);
        }
        Ok(0)
    }

    /// 对应 Java __nonzero__()（JythonWrapper.java:256-271）：boolean 角色 →
    /// as_boolean；sequence → size>0；hash → !is_empty；其余 false
    fn __bool__(&self) -> PyResult<bool> {
        if let Some(b) = &self.model.boolean {
            return b.as_boolean().map_err(errors::template_error_to_pyerr);
        }
        if let Some(seq) = &self.model.sequence {
            let n = seq.size().map_err(errors::template_error_to_pyerr)?;
            return Ok(n > 0);
        }
        if let Some(h) = &self.model.hash {
            let empty = h.is_empty().map_err(errors::template_error_to_pyerr)?;
            return Ok(!empty);
        }
        Ok(false)
    }
}

// ---------------------------------------------------------------------------
// 测试：通用适配器 __getitem__/__call__/__len__/__bool__（对应 Java
// TemplateModelToJythonAdapter 各方法，JythonWrapper.java:186-271）
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wrapper::PyObjectWrapperInner;
    use freemarker::error::Result;
    use freemarker::template::TemplateMethodModelEx;
    use freemarker::value::TNumber;
    use indexmap::IndexMap;
    use pyo3::exceptions::PyTypeError;
    use pyo3::types::PyInt;

    /// 两数相加的方法模型（exec 参数为模板侧 TNumber）
    struct AddMethod;
    impl TemplateMethodModelEx for AddMethod {
        fn exec(
            &self,
            _env: &mut freemarker::core::Environment,
            args: Vec<TModel>,
        ) -> Result<TModel> {
            let a = args[0].get_number()?.as_i64().unwrap();
            let b = args[1].get_number()?.as_i64().unwrap();
            Ok(TModel::from_number(TNumber::from_i64(a + b)))
        }
    }

    fn adapter(py: Python<'_>, model: TModel) -> PyResult<Py<PyAny>> {
        let w = PyObjectWrapperInner::new(true, false);
        let a = Py::new(py, TemplateModelAdapter { model, wrapper: w })?;
        Ok(a.into_any())
    }

    /// __getitem__：int → sequence.get；str → hash.get；缺失键 → None
    #[test]
    fn adapter_getitem_int_and_str() {
        Python::attach(|py| {
            let seq = TModel::from_sequence(vec![
                TModel::from_scalar("a".into()),
                TModel::from_scalar("b".into()),
            ]);
            let a = adapter(py, seq).unwrap();
            assert_eq!(
                a.bind(py)
                    .get_item(PyInt::new(py, 1))
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "b"
            );
            let hash = TModel::from_hash(IndexMap::from([(
                "k".to_string(),
                TModel::from_scalar("v".into()),
            )]));
            let a = adapter(py, hash).unwrap();
            assert_eq!(
                a.bind(py)
                    .get_item("k")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "v"
            );
            // 缺失键 → None
            assert!(a.bind(py).get_item("missing").unwrap().is_none());
            // 非 hash 模型按字符串键查找 → TypeError
            let scalar = TModel::from_scalar("x".into());
            let a = adapter(py, scalar).unwrap();
            let err = a.bind(py).get_item("k").unwrap_err();
            assert!(err.is_instance_of::<PyTypeError>(py), "{err}");
        });
    }

    /// __call__：参数逐个 wrap 回 TModel → exec → 结果 unwrap
    #[test]
    fn adapter_call_method() {
        Python::attach(|py| {
            let a = adapter(py, TModel::from_method(AddMethod)).unwrap();
            // 变体 1：call1 显式 PyTuple（PyCallArgs 的 tuple 直接语义）
            let r = a
                .bind(py)
                .call1(pyo3::types::PyTuple::new(py, [7i64, 5i64]).unwrap())
                .unwrap();
            assert_eq!(r.extract::<i64>().unwrap(), 12);
            // 变体 2：call（kwargs=None）Rust 二元组
            let r = a.bind(py).call((7i64, 5i64), None).unwrap();
            assert_eq!(r.extract::<i64>().unwrap(), 12);
            // 非 method 模型调用 → TypeError
            let scalar = TModel::from_scalar("x".into());
            let a = adapter(py, scalar).unwrap();
            let err = a.bind(py).call0().unwrap_err();
            assert!(err.is_instance_of::<PyTypeError>(py), "{err}");
        });
    }

    /// __len__：sequence.size / hash_ex.size；其余 0
    #[test]
    fn adapter_len() {
        Python::attach(|py| {
            let seq = TModel::from_sequence(vec![
                TModel::from_scalar("a".into()),
                TModel::from_scalar("b".into()),
                TModel::from_scalar("c".into()),
            ]);
            let a = adapter(py, seq).unwrap();
            let n: usize = a
                .bind(py)
                .call_method0("__len__")
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(n, 3);
            let hash = TModel::from_hash(IndexMap::from([(
                "k".to_string(),
                TModel::from_scalar("v".into()),
            )]));
            let a = adapter(py, hash).unwrap();
            let n: usize = a
                .bind(py)
                .call_method0("__len__")
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(n, 1);
            let scalar = TModel::from_scalar("x".into());
            let a = adapter(py, scalar).unwrap();
            let n: usize = a
                .bind(py)
                .call_method0("__len__")
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(n, 0);
        });
    }

    /// __bool__：boolean 角色 → as_boolean；sequence → size>0；hash → !is_empty；其余 false
    #[test]
    fn adapter_bool() {
        Python::attach(|py| {
            let b = adapter(py, TModel::from_boolean(true)).unwrap();
            assert!(b
                .bind(py)
                .call_method0("__bool__")
                .unwrap()
                .extract::<bool>()
                .unwrap());
            let b = adapter(py, TModel::from_boolean(false)).unwrap();
            assert!(!b
                .bind(py)
                .call_method0("__bool__")
                .unwrap()
                .extract::<bool>()
                .unwrap());
            let seq = TModel::from_sequence(vec![TModel::from_scalar("a".into())]);
            let b = adapter(py, seq).unwrap();
            assert!(b
                .bind(py)
                .call_method0("__bool__")
                .unwrap()
                .extract::<bool>()
                .unwrap());
            let empty = TModel::from_sequence(vec![]);
            let b = adapter(py, empty).unwrap();
            assert!(!b
                .bind(py)
                .call_method0("__bool__")
                .unwrap()
                .extract::<bool>()
                .unwrap());
            let scalar = TModel::from_scalar("x".into());
            let b = adapter(py, scalar).unwrap();
            assert!(!b
                .bind(py)
                .call_method0("__bool__")
                .unwrap()
                .extract::<bool>()
                .unwrap());
        });
    }
}
