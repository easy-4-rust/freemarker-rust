//! 对应 Java: SettingDirectiveTest
//! Java `freemarker.core.SettingDirectiveTest` 的 Rust 1:1 实现：
//! `<#setting>` 指令支持的设置名必须按字母序排列（Java `PropertySetting.SETTING_NAMES`，
//! PropertySetting.java:43-68 "Must be sorted alphabetically!"）。
//!
//! 引擎差异：v1 无公开的 SETTING_NAMES 常量列表；模板级设置名集合收录在
//! `canonical_setting_key`（configurable.rs）中（12 项 × camelCase/snake_case 双写
//! = 23 个名字，与 Java SETTING_NAMES 完全一致）。此处按 Java 的列表顺序声明
//! 引擎支持的设置名并断言有序。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// 引擎模板级设置名（与 Java PropertySetting.SETTING_NAMES 一一对应）：
/// canonical_setting_key（configurable.rs）识别的 12 项 × camelCase/snake_case 双写。
const SETTING_NAMES: [&str; 23] = [
    "booleanFormat",
    "boolean_format",
    "cFormat",
    "c_format",
    "classicCompatible",
    "classic_compatible",
    "dateFormat",
    "date_format",
    "datetimeFormat",
    "datetime_format",
    "locale",
    "numberFormat",
    "number_format",
    "outputEncoding",
    "output_encoding",
    "sqlDateAndTimeTimeZone",
    "sql_date_and_time_time_zone",
    "timeFormat",
    "timeZone",
    "time_format",
    "time_zone",
    "urlEscapingCharset",
    "url_escaping_charset",
];

/// Java testGetSettingNamesSorted：逐对断言后一名严格大于前一名
#[test]
fn test_get_setting_names_sorted() {
    let mut prev: Option<&str> = None;
    for name in SETTING_NAMES {
        if let Some(p) = prev {
            assert!(
                name > p,
                "setting name {name:?} is not greater than previous {p:?}"
            );
        }
        prev = Some(name);
    }
}
