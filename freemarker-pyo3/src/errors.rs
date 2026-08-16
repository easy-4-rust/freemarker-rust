//! 异常桥接 —— Python 异常（PyErr）↔ 模板错误（TemplateError）
//! 对应 Java: JythonModel 各方法把 PyException 包装为 TemplateModelException
//! （JythonModel.java getAsBoolean/get/getAsString/exec 等）；JythonWrapper.unwrap
//! 内部适配器把 TemplateModelException 转为 Py.JavaError。
//! 双向规则见 docs/10 §5：
//! - Python 侧抛错 → TemplateError::Model（消息含异常类型 + 详情 + traceback）；
//! - 模板错误 → Python 侧自定义 `freemarker.FreeMarkerError`（PyRuntimeError 子类），
//!   消息 = 完整 FreeMarker 风格错误文本（含 `[in template ...]` 定位，docs/09 §2）。

use freemarker::error::TemplateError;
use pyo3::create_exception;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

// Python 侧模板异常类 —— 对应设计决策（docs/10 §5 推荐自定义异常便于按类型捕获）
create_exception!(freemarker, FreeMarkerError, PyRuntimeError);

/// PyErr → TemplateError::Model —— 对应 Java `new TemplateModelException(PyException)`
/// （JythonModel.java:69-73 等；Python 异常保留 traceback 文本，docs/10 §5）。
/// 注意：调用方必须已持有 GIL（py 参数即证据）。
pub(crate) fn py_err_to_template_error(py: Python<'_>, err: PyErr) -> TemplateError {
    // 异常类型名（Java：PyException 的异常类名）
    let type_name = err
        .get_type(py)
        .name()
        .ok()
        .and_then(|n| n.to_cow().ok().map(|s| s.into_owned()))
        .unwrap_or_else(|| "Exception".to_string());
    // 异常详情（PyException.__str__ 语义近似）
    let value_str = err
        .value(py)
        .str()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut message = format!("{type_name}: {value_str}");
    // traceback（Java：PyException.traceback —— Python 侧排错上下文）
    if let Some(tb) = err.traceback(py) {
        if let Ok(formatted) = tb.format() {
            if !formatted.is_empty() {
                message.push('\n');
                message.push_str(&formatted);
            }
        }
    }
    TemplateError::Model { message }
}

/// TemplateError → PyErr（FreeMarkerError）—— 对应 docs/10 §5：
/// 消息 = 完整错误文本（to_user_message；含模板名/行列定位的路径已由渲染层附加）。
/// Flow 是内部流控信号（break/continue），正常渲染路径不会到达此处。
pub(crate) fn template_error_to_pyerr(err: TemplateError) -> PyErr {
    FreeMarkerError::new_err(err.to_user_message())
}
