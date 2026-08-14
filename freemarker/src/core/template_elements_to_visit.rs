//! 待访问模板元素集合 —— 对应 Java `freemarker.core.TemplateElementsToVisit`
//! （包装 Collection<TemplateElement>；visit 指令的目标元素列表）

/// 对应 Java `TemplateElementsToVisit`（ElementKind::Visit 的子结构）
#[allow(dead_code)]
pub(crate) struct TemplateElementsToVisit;
