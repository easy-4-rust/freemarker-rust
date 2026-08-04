//! 深度解包 —— 对应 Java `freemarker.template.utility.DeepUnwrap`
//! （Java :47- 行：模板模型 → 原生对象递归解包；v1 实现位于
//! `ObjectWrapper::unwrap`（SimpleObjectWrapper 的递归展开，
//! 对应 Java DeepUnwrap.unwrap :100-176）——本类型为公开入口）

use crate::template::{DynValue, ObjectWrapper, TModel, SIMPLE_WRAPPER};

/// 深度解包（对应 DeepUnwrap.java）
pub struct DeepUnwrap;

impl DeepUnwrap {
    /// 解包（Java `unwrap(TemplateModel)`；无法展开 → Err）
    pub fn unwrap(model: &TModel) -> crate::error::Result<DynValue> {
        SIMPLE_WRAPPER.unwrap(model)
    }

    /// 宽松解包（Java `permissiveUnwrap`；无法展开 → Null）
    pub fn permissive_unwrap(model: &TModel) -> DynValue {
        SIMPLE_WRAPPER.unwrap(model).unwrap_or(DynValue::Null)
    }
}
