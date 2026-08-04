//! 包装模板模型 —— 对应 Java `freemarker.template.WrappingTemplateModel`
//! （Java :115 行：持 ObjectWrapper 的模型基类——被包装对象 + 包装器；
//! v1 用 `wrap` 构造等价）

use crate::template::{DynValue, ObjectWrapper, TModel};

/// 包装模板模型（对应 WrappingTemplateModel.java；v1 持有包装器与原始值）
pub struct WrappingTemplateModel {
    wrapper: Box<dyn ObjectWrapper>,
    wrapped: DynValue,
}

impl WrappingTemplateModel {
    /// 构造（Java :37-45；v1 的 wrapped 为 DynValue）
    pub fn new(wrapper: Box<dyn ObjectWrapper>, wrapped: DynValue) -> Self {
        WrappingTemplateModel { wrapper, wrapped }
    }

    /// 被包装值（Java `getWrappedObject()`）
    pub fn wrapped_object(&self) -> &DynValue {
        &self.wrapped
    }

    /// 包装器（Java `getObjectWrapper()`）
    pub fn object_wrapper(&self) -> &dyn ObjectWrapper {
        &*self.wrapper
    }

    /// 包装为模板模型（Java `wrap(Object)`）
    pub fn wrap(&self, obj: &DynValue) -> TModel {
        self.wrapper
            .wrap(obj)
            .ok()
            .flatten()
            .unwrap_or_else(TModel::nothing)
    }
}
