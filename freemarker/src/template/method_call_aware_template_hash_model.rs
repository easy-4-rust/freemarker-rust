//! 方法调用感知哈希 —— 对应 Java `freemarker.template.MethodCallAwareTemplateHashModel`
//! （Java :138 行：哈希模型可感知被方法调用语法（`obj.method(...)`）访问——
//! 调用时经 `get(String)` 返回方法模型而非普通值）

/// 方法调用感知哈希（对应 MethodCallAwareTemplateHashModel.java）
pub trait MethodCallAwareTemplateHashModel {
    /// Java `get(String key)` 的方法调用形态：键访问发生在方法调用
    /// 语法中时返回 true 分支语义（v1 由调用方实现区分）
    fn is_method_call(&self) -> bool {
        false
    }
}
