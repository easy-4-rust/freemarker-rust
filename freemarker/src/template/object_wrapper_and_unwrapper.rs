//! 对象包装与解包 —— 对应 Java `freemarker.template.ObjectWrapperAndUnwrapper`
//! （Java :92 行：ObjectWrapper + unwrap 双向能力；Rust 侧 wrap 经
//! `ObjectWrapper::wrap`、unwrap 经 `SimpleObjectWrapper::unwrap`
//! —— 本 trait 合并双向接口）

use crate::template::{DynValue, ObjectWrapper, TModel};

/// 对象包装与解包（对应 ObjectWrapperAndUnwrapper.java）
pub trait ObjectWrapperAndUnwrapper: ObjectWrapper {
    /// 解包模板模型为原始值（Java `unwrap(TemplateModel)`）
    fn unwrap(&self, model: &TModel) -> DynValue;
}
