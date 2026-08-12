//! 设置集合 —— 对应 Java `freemarker.core.Configurable`
//! （全部设置项见 docs/07 §2；继承链 v1 为单一层级）

use crate::builtins::format::CFormatKind;
use crate::cache::LookupStrategyKind;
use crate::core::template_class_resolver::NewBuiltinClassResolver;
use crate::core::{AutoEscaping, OutputFormatKind};
use crate::template::Version;
use chrono::Offset as _; // fix()（TzOffset → FixedOffset）
use chrono::TimeZone as _; // offset_from_utc_datetime（TzSetting::Named）
use chrono::{FixedOffset, NaiveDateTime};
use chrono_tz::Tz;
use std::str::FromStr;

/// 时区设置 —— 对应 Java `java.util.TimeZone`（Environment.getTimeZone）：
/// IANA 名称（含 DST 规则，如 `Etc/GMT-1`、`America/New_York`）或 `GMT±HH[:mm]`/`UTC`
/// 固定偏移（Java TimeZone.getTimeZone 对 GMT 名字的解析）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TzSetting {
    /// IANA 名称（chrono-tz 含 DST 表）
    Named(Tz),
    /// 固定偏移（GMT+02 / GMT+02:00 / GMT+0230 / UTC）
    Fixed(FixedOffset),
}

impl TzSetting {
    /// 目标 UTC 时刻的固定偏移（DST 感知；对应 Java TimeZone.getOffset(epochMillis)）
    pub fn offset_at(&self, naive_utc: &NaiveDateTime) -> FixedOffset {
        match self {
            TzSetting::Named(t) => t.offset_from_utc_datetime(naive_utc).fix(),
            TzSetting::Fixed(f) => *f,
        }
    }
}

impl FromStr for TzSetting {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let t = s.trim();
        let upper = t.to_ascii_uppercase();
        if upper == "UTC" || upper == "GMT" || upper == "Z" {
            return Ok(TzSetting::Fixed(FixedOffset::east_opt(0).unwrap()));
        }
        if let Some(rest) = upper.strip_prefix("GMT") {
            // GMT+02 / GMT+02:00 / GMT+0230 / GMT-05（Java TimeZone.getTimeZone 的 GMT 名）
            let (sign, rest) = match rest.chars().next() {
                Some('+') => (1i32, &rest[1..]),
                Some('-') => (-1i32, &rest[1..]),
                _ => return Err(()),
            };
            let (h, m) = if let Some(i) = rest.find(':') {
                (
                    rest[..i].parse::<i32>().map_err(|_| ())?,
                    rest[i + 1..].parse::<i32>().map_err(|_| ())?,
                )
            } else if rest.len() == 4 {
                (
                    rest[..2].parse::<i32>().map_err(|_| ())?,
                    rest[2..].parse::<i32>().map_err(|_| ())?,
                )
            } else {
                (rest.parse::<i32>().map_err(|_| ())?, 0)
            };
            if h > 23 || m > 59 {
                return Err(());
            }
            return Ok(TzSetting::Fixed(
                FixedOffset::east_opt(sign * (h * 3600 + m * 60)).unwrap(),
            ));
        }
        // IANA 名（chrono-tz 区分大小写，用原名解析）
        t.parse::<Tz>().map(TzSetting::Named).map_err(|_| ())
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub locale: String,
    pub time_zone: TzSetting,
    /// 时区 ID 字符串（Java `TimeZone.getID()`；`.time_zone` 内置变量读数。
    /// 与 time_zone 的差异：GMT 名归一化为 `GMT±HH:MM`，IANA 名按原样）
    pub time_zone_id: String,
    pub number_format: String,
    pub boolean_format: String,
    pub date_format: String,
    pub time_format: String,
    pub date_time_format: String,
    pub output_format: OutputFormatKind,
    pub auto_escaping: AutoEscaping,
    /// C 格式变体（Java `c_format` 设置；StandardCFormats 注册表：JavaScript or
    /// JSON/JavaScript/JSON/Java/legacy/XS；默认 JavaScript or JSON）
    pub c_format: CFormatKind,
    pub whitespace_stripping: bool,
    pub strict_syntax: bool,
    pub classic_compatible: bool,
    pub incompatible_improvements: Version,
    pub output_encoding: String,
    pub url_escaping_charset: String,
    /// 循环变量为 null 时回退到上次循环（fallbackOnNullLoopVariable）
    pub fallback_on_null_loop_variable: bool,
    /// 模板更新延迟（秒）—— 对应 Configuration.setTemplateUpdateDelay（TemplateCache.setDelay
    /// 语义见 docs/07 §2 :66）；默认 1
    pub delay: u64,
    /// 局部化模板查找 —— 对应 Configuration.setLocalizedLookup（docs/07 §2 :66）；默认 true
    pub localized_lookup: bool,
    /// 模板查找策略 —— 对应 Configuration.setTemplateLookupStrategy（docs/07 §2 :63）；
    /// 默认 Default020300（本地化回退 + acquisition，见 cache/template_lookup_strategy.rs）
    pub lookup_strategy: LookupStrategyKind,
    /// 输入编码 —— 对应 Configuration.setDefaultEncoding（input_encoding 设置；
    /// None = Java 默认 "UTF-8"（Configuration.DEFAULT_TEMPLATE_ENCODING）；
    /// 模板 `<#ftl encoding=...>` 头按 WrongEncodingException 重读，get_template_encoded）
    pub input_encoding: Option<String>,
    /// 模板异常处理器 —— 对应 Configuration.setTemplateExceptionHandler（docs/09 §6.3）：
    /// `"rethrow"`（生产默认）/`"debug"`/`"html_debug"`/`"ignore"`。
    /// Java 的 DEBUG/HTML_DEBUG 在写出调试文本后仍抛出异常、IGNORE 保留已输出内容并
    /// 继续渲染——v1 在 process() 边界处理（文档化偏差，见 environment.rs process()）。
    pub template_exception_handler: String,
    /// `?new` 类解析器 —— 对应 Configuration.setNewBuiltinClassResolver
    /// （Configurable.java:1608；默认 UNRESTRICTED_RESOLVER，Configurable.java:477；
    /// 权限判定见 core::template_class_resolver）
    pub new_builtin_class_resolver: NewBuiltinClassResolver,
    /// lazyImports 设置 —— 对应 `Configurable.lazyImports`（Configurable.java:410：
    /// 默认 false，:501 initDefaults；`<#import>` 指令与 lazyAutoImports 未设置时
    /// auto imports 的惰性开关；getLazyImports :1852-1854 父链回退——v1 以
    /// Environment::new 合并后的值等价）
    pub lazy_imports: bool,
    /// lazyAutoImports 设置 —— 对应 `Configurable.lazyAutoImports`（Configurable.java:
    /// 411-412：Boolean + lazyAutoImportsSet；默认 null = 未设置 → 回退 lazyImports；
    /// getLazyAutoImports :1900-1904；doAutoImports 用
    /// `getLazyAutoImports() ?? getLazyImports()`，Configuration.java:3690-3692）
    pub lazy_auto_imports: Option<bool>,
}

/// Java `TimeZone.getTimeZone(id).getID()` 的 v1 复刻（以 Java 实测为准）：
/// - `GMT+1`/`GMT+01`/`GMT+01:00`/`GMT+0130` → 归一化 `GMT+01:00`；
/// - `UTC`/`Z` → `UTC`；`GMT` → `GMT`；
/// - 其余（IANA 名，如 `Etc/GMT-1`、`America/New_York`）按原样返回。
pub fn java_time_zone_id(s: &str) -> String {
    let t = s.trim();
    let upper = t.to_ascii_uppercase();
    if upper == "UTC" || upper == "Z" {
        return "UTC".to_string();
    }
    if upper == "GMT" {
        return "GMT".to_string();
    }
    if let Some(rest) = upper.strip_prefix("GMT") {
        let (sign, rest) = match rest.chars().next() {
            Some('+') => (1i32, &rest[1..]),
            Some('-') => (-1i32, &rest[1..]),
            _ => return t.to_string(),
        };
        let (h, m) = if let Some(i) = rest.find(':') {
            match (rest[..i].parse::<i32>(), rest[i + 1..].parse::<i32>()) {
                (Ok(h), Ok(m)) => (h, m),
                _ => return t.to_string(),
            }
        } else if rest.len() == 4 {
            match (rest[..2].parse::<i32>(), rest[2..].parse::<i32>()) {
                (Ok(h), Ok(m)) => (h, m),
                _ => return t.to_string(),
            }
        } else {
            match rest.parse::<i32>() {
                Ok(h) => (h, 0),
                _ => return t.to_string(),
            }
        };
        return format!("GMT{}{:02}:{:02}", if sign < 0 { "-" } else { "+" }, h, m);
    }
    t.to_string()
}

/// `<#setting>` 指令的设置名 → 规范键（snake_case）—— 对应 Java
/// `PropertySetting.SETTING_NAMES`（PropertySetting.java:43-68）：
/// 模板内可设置的 12 项设置均接受 camelCase 与 snake_case 两种写法
/// （`Configurable.setSetting` 同样双写匹配，Configurable.java:2671 起，
/// 如 `BOOLEAN_FORMAT_KEY_SNAKE_CASE`/`BOOLEAN_FORMAT_KEY_CAMEL_CASE`）。
/// 未知键原样返回（v1 在执行期报 "Unsupported setting"，Java 在解析期报
/// "Unknown setting name"——既有偏差，见 exec.rs exec_setting）。
/// 注：Java 另有命名约定一致性检查（同模板混用两写法 → "Naming convention
/// mismatch"，PropertySetting 实测），v1 无命名约定概念（文档化偏差，更宽松）。
pub fn canonical_setting_key(name: &str) -> &str {
    match name {
        "booleanFormat" | "boolean_format" => "boolean_format",
        "cFormat" | "c_format" => "c_format",
        "classicCompatible" | "classic_compatible" => "classic_compatible",
        "dateFormat" | "date_format" => "date_format",
        "datetimeFormat" | "datetime_format" => "datetime_format",
        // Java LOCALE_KEY 两种约定同名（Configurable.java:95-97）
        "locale" => "locale",
        "numberFormat" | "number_format" => "number_format",
        "outputEncoding" | "output_encoding" => "output_encoding",
        "sqlDateAndTimeTimeZone" | "sql_date_and_time_time_zone" => "sql_date_and_time_time_zone",
        // Java 仅 Configurable 级（模板内 `<#setting>` 报 "recognized, but changing this
        // setting from inside a template isn't supported"，jar 实测）——v1 允许模板内设置
        // （文档化偏差，exec.rs exec_setting）
        "templateExceptionHandler" | "template_exception_handler" => "template_exception_handler",
        "timeFormat" | "time_format" => "time_format",
        "timeZone" | "time_zone" => "time_zone",
        "urlEscapingCharset" | "url_escaping_charset" => "url_escaping_charset",
        other => other,
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            locale: "en_US".to_string(),
            time_zone: TzSetting::Fixed(chrono::FixedOffset::east_opt(0).unwrap()),
            time_zone_id: "GMT+00:00".to_string(),
            number_format: "number".to_string(),
            boolean_format: "true,false".to_string(),
            // Java 默认 ""（Configurable.java:450-458："" 等价 medium，见 setDateFormat javadoc）
            date_format: String::new(),
            time_format: String::new(),
            date_time_format: String::new(),
            output_format: OutputFormatKind::PlainText,
            auto_escaping: AutoEscaping::Default,
            c_format: CFormatKind::JavaScriptOrJson,
            whitespace_stripping: true,
            strict_syntax: false,
            classic_compatible: false,
            incompatible_improvements: Version::V2_3_34,
            output_encoding: "UTF-8".to_string(),
            // Java 默认 null（Configurable.java:491 "outputEncoding and urlEscapingCharset defaults to null"）；
            // 空串 = 未设置（?url 回退 UTF-8，`.url_escaping_charset` 缺失）
            url_escaping_charset: String::new(),
            fallback_on_null_loop_variable: true,
            delay: 1,
            localized_lookup: true,
            lookup_strategy: LookupStrategyKind::Default020300,
            input_encoding: None,
            // Java 默认 RETHROW_HANDLER（_TemplateAPI.getDefaultTemplateExceptionHandler）
            template_exception_handler: "rethrow".to_string(),
            // Java 默认 UNRESTRICTED_RESOLVER（Configurable.java:477）
            new_builtin_class_resolver: NewBuiltinClassResolver::Unrestricted,
            // Java lazyImports 默认 false（Configurable.java:501）；lazyAutoImports
            // 默认 null（未设置 → 回退 lazyImports，Configurable.java:411-412/1900-1904）
            lazy_imports: false,
            lazy_auto_imports: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parse_and_int() {
        let v = Version::parse("2.3.34").unwrap();
        assert_eq!(v, Version::V2_3_34);
        assert_eq!(v.to_int(), 2_003_034);
    }

    #[test]
    fn output_format_names() {
        assert_eq!(OutputFormatKind::Html.name(), "HTML");
        assert_eq!(
            OutputFormatKind::parse("html"),
            Some(OutputFormatKind::Html)
        );
        assert!(OutputFormatKind::Html.is_markup());
        assert!(!OutputFormatKind::PlainText.is_markup());
    }

    #[test]
    fn canonical_setting_key_both_conventions() {
        // Java PropertySetting.SETTING_NAMES（PropertySetting.java:43-68）：
        // 12 项模板级设置均支持 camelCase 与 snake_case 两种写法 → snake_case 规范键
        for (snake, camel) in [
            ("boolean_format", "booleanFormat"),
            ("c_format", "cFormat"),
            ("classic_compatible", "classicCompatible"),
            ("date_format", "dateFormat"),
            ("datetime_format", "datetimeFormat"),
            ("number_format", "numberFormat"),
            ("output_encoding", "outputEncoding"),
            ("sql_date_and_time_time_zone", "sqlDateAndTimeTimeZone"),
            ("time_format", "timeFormat"),
            ("time_zone", "timeZone"),
            ("url_escaping_charset", "urlEscapingCharset"),
        ] {
            assert_eq!(canonical_setting_key(snake), snake);
            assert_eq!(canonical_setting_key(camel), snake);
        }
        // locale 两种约定同名（Configurable.LOCALE_KEY，Configurable.java:95-97）
        assert_eq!(canonical_setting_key("locale"), "locale");
        // template_exception_handler：Configurable 级设置（v1 允许模板内设置——
        // Java PropertySetting 报 "recognized, but changing this setting from inside a
        // template isn't supported"，见 exec.rs exec_setting 注释），两种约定均规范化
        assert_eq!(
            canonical_setting_key("template_exception_handler"),
            "template_exception_handler"
        );
        assert_eq!(
            canonical_setting_key("templateExceptionHandler"),
            "template_exception_handler"
        );
    }

    #[test]
    fn canonical_setting_key_unknown_and_config_level_passthrough() {
        // 未知键原样返回（v1 执行期报 "Unsupported setting"；Java 解析期
        // "Unknown setting name: ..." 报错——既有偏差，exec.rs exec_setting）
        assert_eq!(canonical_setting_key("foo"), "foo");
        assert_eq!(canonical_setting_key("booolean_format"), "booolean_format");
        assert_eq!(canonical_setting_key("booleanFormatX"), "booleanFormatX");
        // 配置级设置名不在模板白名单 → 不规范化（grammar.rs 解析期另行拒绝）
        assert_eq!(
            canonical_setting_key("whitespace_stripping"),
            "whitespace_stripping"
        );
    }
}
