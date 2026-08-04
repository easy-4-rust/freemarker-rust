//! 适配器模板模型 —— 对应 Java `freemarker.template.AdapterTemplateModel`
//! （Java :49 行：包装 Java 对象的模型可还原被包装对象——getWrappedObject）

/// 适配器模板模型（对应 AdapterTemplateModel.java）
pub trait AdapterTemplateModel {
    /// 还原被包装对象（Java `getWrappedObject()`；Rust 侧包装方自定还原类型）
    fn wrapped_object(&self) -> &dyn std::any::Any;
}
