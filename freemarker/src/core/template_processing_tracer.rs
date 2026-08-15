//! 模板处理追踪接口 —— 对应 Java `freemarker.core.TemplateProcessingTracer`
//! （enterElement/exitElement 回调；TracedElement 子接口提供 getTemplate 等；
//!  当前 Rust 无调用方；未来可观测性扩展点）

/// 对应 Java `TemplateProcessingTracer`（当前 Rust 无调用方；可观测性扩展点）
#[allow(dead_code)]
pub(crate) trait TemplateProcessingTracer {
    /// 进入模板元素（Java enterElement）
    fn enter_element(&self, env: &crate::core::Environment, element: &crate::core::Element);
    /// 退出模板元素（Java exitElement）
    fn exit_element(&self, env: &crate::core::Environment, element: &crate::core::Element);
}
