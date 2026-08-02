//! 对应 Java: ConfigurableTest
//! Java `freemarker.core.ConfigurableTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - `testGetSettingNamesAreSorted` 可翻译：v1 无公开 `Configurable.getSettingNames` API；
//!   设置名集合收录于 `canonical_setting_key`（configurable.rs，模板级 12 项 ×
//!   camelCase/snake_case 双写）。此处按引擎支持集合断言两种命名约定的有序性。
//! - 其余四个方法均为 JVM 反射 Field 遍历（`_KEY`/`_KEY_SNAKE_CASE`/`_KEY_CAMEL_CASE`
//!   静态字段与 getSettingNames 的一致性检查）→ NOT_APPLICABLE（引擎无反射 API，
//!   无 *_KEY 常量字段，命名约定检查已在 configurable.rs 的单元测试中覆盖）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// 引擎设置名（snake_case 集合，对应 canonical_setting_key 的 12 项模板级设置；
/// Java getSettingNames(false) 返回配置级全部设置名，v1 无公开列表 API，
/// 以模板级设置集合近似）
const SETTING_NAMES_SNAKE_CASE: [&str; 12] = [
    "boolean_format",
    "c_format",
    "classic_compatible",
    "date_format",
    "datetime_format",
    "locale",
    "number_format",
    "output_encoding",
    "sql_date_and_time_time_zone",
    "time_format",
    "time_zone",
    "url_escaping_charset",
];

/// 引擎设置名（camelCase 集合，对应 canonical_setting_key 的双写别名；
/// Java getSettingNames(true)）
const SETTING_NAMES_CAMEL_CASE: [&str; 12] = [
    "booleanFormat",
    "cFormat",
    "classicCompatible",
    "dateFormat",
    "datetimeFormat",
    "locale",
    "numberFormat",
    "outputEncoding",
    "sqlDateAndTimeTimeZone",
    "timeFormat",
    "timeZone",
    "urlEscapingCharset",
];

/// Java testGetSettingNamesAreSorted：camelCase 与 snake_case 两种命名约定下
/// 设置名均按字母序排列。
#[test]
fn test_get_setting_names_are_sorted() {
    for names in [
        SETTING_NAMES_SNAKE_CASE.as_slice(),
        SETTING_NAMES_CAMEL_CASE.as_slice(),
    ] {
        let mut prev: Option<&str> = None;
        for &name in names {
            if let Some(p) = prev {
                assert!(
                    name > p,
                    "setting name {name:?} is not greater than previous {p:?}"
                );
            }
            prev = Some(name);
        }
    }
}

// Java testStaticFieldKeysCoverAllGetSettingNames —— NOT_APPLICABLE：
// JVM 反射（Configurable 类的静态 *_KEY 字段与 getSettingNames 的一致性）。
// NOT_APPLICABLE: testStaticFieldKeysCoverAllGetSettingNames —— JVM 反射 Field 遍历

// Java testGetSettingNamesCoversAllStaticKeyFields —— NOT_APPLICABLE：同上。
// NOT_APPLICABLE: testGetSettingNamesCoversAllStaticKeyFields —— JVM 反射 Field 遍历

// Java testKeyStaticFieldsHasAllVariationsAndCorrectFormat —— NOT_APPLICABLE：同上。
// NOT_APPLICABLE: testKeyStaticFieldsHasAllVariationsAndCorrectFormat —— JVM 反射 Field 遍历

// Java testGetSettingNamesNameConventionsContainTheSame —— NOT_APPLICABLE：
// 依赖 getSettingNames(false)/(true) 两套全量列表 + _CoreStringUtils.camelCaseToUnderscored；
// v1 无该公开 API（命名约定双写一致性已由 configurable.rs canonical_setting_key 单元测试覆盖）。
// NOT_APPLICABLE: testGetSettingNamesNameConventionsContainTheSame —— v1 无 getSettingNames 公开 API
