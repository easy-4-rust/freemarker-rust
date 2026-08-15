//! list else 分支 —— 对应 Java `freemarker.core.ElseOfList`
//! （ListElseContainer 的第二子元素；序列为空时执行 body）

/// 对应 Java `ElseOfList`（ElementKind::List 的 else_ 子结构）
#[allow(dead_code)]
pub(crate) struct ElseOfList;
