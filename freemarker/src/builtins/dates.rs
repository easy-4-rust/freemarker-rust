//! 日期内建 —— 对应 Java `BuiltInsForDates.java`（iso_* 家族 + date_if_unknown 等）
//! 与 `BuiltInsForMultipleTypes.dateBI`（?date/?time/?datetime；BuiltInsForMultipleTypes.java:120-227）
//!
//! 语义要点（Java 对照）：
//! - `?iso_utc[_fz|_nz][_ms|_m|_h]` / `?iso_local...` → iso_utc_or_local_BI
//!   （BuiltInsForDates.java:150-178）：DateUtil.dateToISO8601String，UTC 或环境时区；
//! - `?iso[_nz|_nz][_ms|_m|_h]` → iso_BI（:79-144）：带显式时区参数的方法；
//! - 偏移显示（AbstractISOBI.shouldShowOffset :202-212）：date-only 恒不显示；
//!   nz/fz 强制关闭/开启；默认非 SQL 时间显示（IcI ≥ 2.3.21 的 SQL 时间不显示）；
//! - `?date`/`?time`/`?datetime` → dateBI：日期目标同型返回/可转换；
//!   字符串目标按 date_format 设置或显式模式解析（ISO/XS/Java 模式）。

use crate::builtins::iso_date_format::{Accuracy, IsoSpec};
use crate::core::eval_util::{arg_count, arg_string, check_arg_count};
use crate::core::{Environment, Expr, TzSetting};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::value::{DateType, DateValue};
use chrono::FixedOffset;
use std::str::FromStr;

/// ?iso 家族参数表（对应 BuiltIn.java:175-234 的注册表）
#[derive(Clone, Copy)]
struct IsoVariant {
    /// Some(true)=UTC、Some(false)=本地、None=?iso 显式时区参数
    use_utc: Option<bool>,
    /// Some(true)=fz、Some(false)=nz、None=默认
    show_offset: Option<bool>,
    accuracy: Accuracy,
}

fn variant(name: &str) -> Option<IsoVariant> {
    let (use_utc, mut rest) = if let Some(r) = name.strip_prefix("iso_utc") {
        (Some(true), r)
    } else if let Some(r) = name.strip_prefix("iso_local") {
        (Some(false), r)
    } else {
        let r = name.strip_prefix("iso")?;
        (None, r)
    };
    let mut show_offset = None;
    // 默认 ACCURACY_SECONDS（BuiltIn.java:175-234：仅 _ms 变体为 MILLISECONDS）
    let mut accuracy = Accuracy::Seconds;
    if let Some(r) = rest.strip_prefix("_fz") {
        show_offset = Some(true);
        rest = r;
    } else if let Some(r) = rest.strip_prefix("_nz") {
        show_offset = Some(false);
        rest = r;
    }
    if let Some(r) = rest.strip_prefix("_ms") {
        // Java：内建 _ms = ACCURACY_MILLISECONDS（最少位数，非强制 3 位；
        // 仅 ISO 格式串参数 "ms" 为强制，见 iso_date_format.parse_iso_params）
        accuracy = Accuracy::Milliseconds;
        rest = r;
    } else if let Some(r) = rest.strip_prefix("_m") {
        accuracy = Accuracy::Minutes;
        rest = r;
    } else if let Some(r) = rest.strip_prefix("_h") {
        accuracy = Accuracy::Hours;
        rest = r;
    }
    // 精度后可再带 nz（iso_utc_ms_nz / iso_h_nz；BuiltIn.java 注册表）
    if show_offset.is_none() {
        if let Some(r) = rest.strip_prefix("_nz") {
            show_offset = Some(false);
            rest = r;
        }
    }
    if !rest.is_empty() {
        return None;
    }
    Some(IsoVariant {
        use_utc,
        show_offset,
        accuracy,
    })
}

/// 目标日期（Java BuiltInForDate：非日期报错）
fn target_date(env: &mut Environment, target: &Expr) -> Result<DateValue> {
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    m.get_date()
}

/// iso 家族统一实现 —— 对应 iso_utc_or_local_BI / iso_BI.calculateResult
fn iso_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    v: IsoVariant,
) -> Result<Option<TModel>> {
    check_arg_count("iso", args, 0, if v.use_utc.is_none() { 1 } else { 0 })?;
    let d = target_date(env, target)?;
    // 显式时区参数（仅 ?iso 家族）
    let explicit_tz: Option<TzSetting> = if arg_count(args) > 0 {
        let name = arg_string(env, args, 0)?;
        Some(match TzSetting::from_str(&name) {
            Ok(t) => t,
            Err(_) => {
                return Err(TemplateError::misc(format!(
                    "The time zone string specified for ?iso(...) is not recognized as a valid time zone name: \"{name}\""
                )))
            }
        })
    } else {
        None
    };
    let tz = match (v.use_utc, explicit_tz) {
        (_, Some(t)) => t,
        (Some(true), None) => TzSetting::Fixed(FixedOffset::east_opt(0).unwrap()),
        (Some(false), None) => env.settings.time_zone,
        (None, None) => env.settings.time_zone,
    };
    // AbstractISOBI.shouldShowOffset（BuiltInsForDates.java:202-212）：
    // date-only 恒不显示；nz/fz 强制；sql 值默认在 IcI ≥ 2.3.21 不显示（2.3.20 显示）
    let show_zone_offset = if d.kind == DateType::Date {
        Some(false)
    } else if v.show_offset.is_none() && d.is_sql {
        Some(env.settings.incompatible_improvements.to_int() < 2_003_021)
    } else {
        v.show_offset
    };
    let spec = IsoSpec {
        accuracy: v.accuracy,
        show_zone_offset,
        force_utc: Some(false),
    };
    let s = crate::builtins::iso_date_format::format_iso_like_with_tz(&d, &spec, false, &tz)?;
    Ok(Some(TModel::from_scalar(s)))
}

/// 通用变体实现（builtins::lookup 按名称注册；对应 BuiltIn.java:175-234 全部 iso_* 名）
fn iso_variant_impl(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    name: &str,
) -> Result<Option<TModel>> {
    match variant(name) {
        Some(v) => iso_impl(env, target, args, v),
        None => Ok(None),
    }
}

/// iso 家族注册包装（名称 → 变体解析）
macro_rules! iso_bi {
    ($name:ident, $variant:literal) => {
        pub fn $name(
            env: &mut Environment,
            target: &Expr,
            args: Option<&[Expr]>,
        ) -> Result<Option<TModel>> {
            iso_variant_impl(env, target, args, $variant)
        }
    };
}

iso_bi!(iso, "iso");
iso_bi!(iso_nz, "iso_nz");
iso_bi!(iso_fz, "iso_fz");
iso_bi!(iso_ms, "iso_ms");
iso_bi!(iso_ms_nz, "iso_ms_nz");
iso_bi!(iso_m, "iso_m");
iso_bi!(iso_m_nz, "iso_m_nz");
iso_bi!(iso_h, "iso_h");
iso_bi!(iso_h_nz, "iso_h_nz");
iso_bi!(iso_utc, "iso_utc");
iso_bi!(iso_utc_fz, "iso_utc_fz");
iso_bi!(iso_utc_nz, "iso_utc_nz");
iso_bi!(iso_utc_ms, "iso_utc_ms");
iso_bi!(iso_utc_ms_nz, "iso_utc_ms_nz");
iso_bi!(iso_utc_m, "iso_utc_m");
iso_bi!(iso_utc_m_nz, "iso_utc_m_nz");
iso_bi!(iso_utc_h, "iso_utc_h");
iso_bi!(iso_utc_h_nz, "iso_utc_h_nz");
iso_bi!(iso_local, "iso_local");
iso_bi!(iso_local_nz, "iso_local_nz");
iso_bi!(iso_local_ms, "iso_local_ms");
iso_bi!(iso_local_ms_nz, "iso_local_ms_nz");
iso_bi!(iso_local_m, "iso_local_m");
iso_bi!(iso_local_m_nz, "iso_local_m_nz");
iso_bi!(iso_local_h, "iso_local_h");
iso_bi!(iso_local_h_nz, "iso_local_h_nz");

/// ?date_if_unknown / ?time_if_unknown / ?datetime_if_unknown —— Java dateType_if_unknownBI
/// （BuiltInsForDates.java:45-74）：仅 UNKNOWN 类型补上目标类型，其余原样返回
fn if_unknown_impl(env: &mut Environment, target: &Expr, kind: DateType) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if !m.is_date() {
        return Err(TemplateError::misc(format!(
            "?{} is not applicable to a {} value",
            match kind {
                DateType::Date => "date_if_unknown",
                DateType::Time => "time_if_unknown",
                DateType::DateTime => "datetime_if_unknown",
                DateType::Unknown => "date_if_unknown",
            },
            m.type_name
        )));
    }
    let d = m.get_date()?;
    if d.kind == DateType::Unknown {
        return Ok(Some(TModel::from_date(DateValue::new(d.dt, kind))));
    }
    Ok(Some(m))
}

pub fn date_if_unknown(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    if_unknown_impl(env, target, DateType::Date)
}

pub fn time_if_unknown(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    if_unknown_impl(env, target, DateType::Time)
}

pub fn datetime_if_unknown(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    if_unknown_impl(env, target, DateType::DateTime)
}

/// ?date / ?time / ?datetime —— Java BuiltInsForMultipleTypes.dateBI（:120-227）：
/// 日期目标：同型原样；DATETIME → 目标型；其余报错；
/// 字符串目标：按 date_format/time_format/datetime_format 设置（无参）或显式模式（1 参）解析。
pub fn date_builtin(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    kind: DateType,
) -> Result<Option<TModel>> {
    check_arg_count("date", args, 0, 1)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    if m.is_date() {
        let d = m.get_date()?;
        if d.kind == kind {
            return Ok(Some(m));
        }
        // Java :198-211：unknown 与 datetime 可转任意类型；date/time 互转报错
        if d.kind == DateType::DateTime || d.kind == DateType::Unknown {
            return Ok(Some(TModel::from_date(DateValue::new(d.dt, kind))));
        }
        return Err(TemplateError::misc(format!(
            "Cannot convert {} to {}",
            d.kind.name(),
            kind.name()
        )));
    }
    if let Ok(s) = m.get_scalar() {
        let format = if arg_count(args) > 0 {
            arg_string(env, args, 0)?
        } else {
            match kind {
                DateType::Date => env.settings.date_format.clone(),
                DateType::Time => env.settings.time_format.clone(),
                DateType::DateTime => env.settings.date_time_format.clone(),
                DateType::Unknown => env.settings.date_time_format.clone(),
            }
        };
        let d = crate::core::environment::parse_date_value(env, &s, kind, &format)?;
        return Ok(Some(TModel::from_date(d)));
    }
    Err(TemplateError::misc(format!(
        "?{} is not applicable to a {} value",
        match kind {
            DateType::Date => "date",
            DateType::Time => "time",
            DateType::DateTime => "datetime",
            DateType::Unknown => "date",
        },
        m.type_name
    )))
}

pub fn date(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    date_builtin(env, target, args, DateType::Date)
}

pub fn time(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    date_builtin(env, target, args, DateType::Time)
}

pub fn datetime(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    date_builtin(env, target, args, DateType::DateTime)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_variant_parse() {
        let v = variant("iso_utc").unwrap();
        assert_eq!(v.use_utc, Some(true));
        let v = variant("iso_local_ms_nz").unwrap();
        assert_eq!(v.use_utc, Some(false));
        assert_eq!(v.show_offset, Some(false));
        assert_eq!(v.accuracy, Accuracy::Milliseconds);
        let v = variant("iso_m").unwrap();
        assert_eq!(v.use_utc, None);
        assert_eq!(v.accuracy, Accuracy::Minutes);
        let v = variant("iso_utc_fz").unwrap();
        assert_eq!(v.show_offset, Some(true));
        assert!(variant("iso_x").is_none());
        assert!(variant("nope").is_none());
    }
}
