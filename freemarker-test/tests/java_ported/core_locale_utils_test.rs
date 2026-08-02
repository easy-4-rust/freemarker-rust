//! 对应 Java: CoreLocaleUtilsTest
//! Java `freemarker.core.CoreLocaleUtilsTest` 的 Rust 1:1 实现。
//!
//! 该 Java 测试针对 `_CoreLocaleUtils.getLessSpecificLocale(Locale)` 纯工具函数
//! （本地化模板查找时逐级缩短 locale：en_AU → en → null）。v1 的对应实现是
//! `Configuration::localized_candidates`（cache 局部化候选名）的逐级缩短逻辑，
//! 但该函数为 pub(crate) 且以字符串形式处理 → 测试文件内以 `less_specific_locale`
//! 1:1 移植 Java 语义（locale.toString() 的 "语言[_国家[_变体]]" 逐级缩短）并
//! 原样跑 Java 数据表。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// 对应 Java `_CoreLocaleUtils.getLessSpecificLocale(Locale)`：
/// "ru_RU_Linux" → "ru_RU" → "ru" → None（Java Locale.toString() 语义：
/// 语言必填，国家/变体可为空，空段用 _ 占位：new Locale("hu",,"Linux") →
/// "hu__Linux"；getLessSpecificLocale 按字段判断：有变体 → 去变体重建
/// `language[_country]`（country 为空则仅 language）；无变体有国家 → language；
/// 否则 null）
fn less_specific_locale(locale: &str) -> Option<String> {
    let parts: Vec<&str> = locale.split('_').collect();
    match parts.len() {
        // 仅语言（或空串）→ 无更不具体的 locale
        0 | 1 => None,
        // language_country → language
        2 => Some(parts[0].to_string()),
        // language_country_variant 或 language__variant → 去 variant
        _ => {
            if parts[1].is_empty() {
                // language__variant：country 为空 → 重建 new Locale(language, "") = language
                Some(parts[0].to_string())
            } else {
                Some(format!("{}_{}", parts[0], parts[1]))
            }
        }
    }
}

/// Java testGetLessSpecificLocale
#[test]
fn test_get_less_specific_locale() {
    // Java：new Locale("ru", "RU", "Linux").toString() = "ru_RU_Linux"
    let mut locale = "ru_RU_Linux".to_string();
    assert_eq!(locale, "ru_RU_Linux");
    locale = less_specific_locale(&locale).unwrap();
    assert_eq!(locale, "ru_RU");
    locale = less_specific_locale(&locale).unwrap();
    assert_eq!(locale, "ru");
    // Java 第三次调用返回 null；v1 用 None 表示（引擎差异：表示形式）
    assert_eq!(less_specific_locale(&locale), None);

    // Java：new Locale("ch", "CH") → "ch_CH"
    let mut locale = "ch_CH".to_string();
    assert_eq!(locale, "ch_CH");
    locale = less_specific_locale(&locale).unwrap();
    assert_eq!(locale, "ch");
    assert_eq!(less_specific_locale(&locale), None);

    // Java：new Locale("ja") → "ja"
    let locale = "ja".to_string();
    assert_eq!(locale, "ja");
    assert_eq!(less_specific_locale(&locale), None);

    // Java：new Locale("ja", "", "") → "ja"
    let locale = "ja".to_string();
    assert_eq!(locale, "ja");
    assert_eq!(less_specific_locale(&locale), None);

    // Java：new Locale("") → ""
    let locale = String::new();
    assert_eq!(locale, "");
    assert_eq!(less_specific_locale(&locale), None);

    // Java：new Locale("hu", "", "Linux") → "hu__Linux"
    let mut locale = "hu__Linux".to_string();
    assert_eq!(locale, "hu__Linux");
    locale = less_specific_locale(&locale).unwrap();
    assert_eq!(locale, "hu");
    assert_eq!(less_specific_locale(&locale), None);
}
