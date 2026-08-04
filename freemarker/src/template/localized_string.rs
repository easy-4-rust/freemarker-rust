//! 本地化字符串 —— 对应 Java `freemarker.template.LocalizedString`
//! （Java :55 行：抽象——按 locale 返回本地化字符串的模板模型）

use crate::error::Result;
use crate::template::TModel;

/// 本地化字符串（对应 LocalizedString.java）
pub trait LocalizedString {
    /// 按 locale 取本地化字符串（Java `getLocalizedString(Locale)`；
    /// Rust 用 locale 字符串 ID）
    fn get_localized_string(&self, locale: &str) -> Result<TModel>;
}
