//! 通用标记输出格式（抽象）—— 对应 Java `freemarker.core.CommonMarkupOutputFormat`
//! （MarkupOutputFormat 的公共抽象基类；Rust 侧由 `OutputFormatKind` 枚举承载）

use crate::core::OutputFormatKind;

/// 抽象锚点：markup 输出格式的公共接口（Java 抽象类；Rust 侧由枚举 + escape 委托实现）
#[allow(dead_code)]
pub(crate) fn is_markup(kind: OutputFormatKind) -> bool {
    kind.is_markup()
}
