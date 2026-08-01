//! 数字内建 —— 对应 Java `BuiltInsForNumbers.java`（abs/ceiling/floor/round/byte/short/
//! is_nan/is_infinite/lower_abc/upper_abc/number_to_date 家族；int/long/float/double 在
//! eval.rs 内建集）。?c/?cn 见 multi.rs。
//!
//! 语义要点（Java 对照）：
//! - abs → absBI：按类型取绝对值（负数才新建，正数返回原模型）；
//! - ceiling/floor/round → 以 `new BigDecimal(doubleValue).divide(1, 0, ROUND_*)` 语义
//!   （round = +0.5 后 ROUND_FLOOR，即 HALF_UP）；
//! - byte/short → 类型转换（原类型则原样返回）；
//! - is_nan/is_infinite → NumberUtil.isNaN/isInfinite（Double/Float 判定）；
//! - lower_abc/upper_abc → StringUtil.toLowerABC/toUpperABC（1→a、27→aa）；
//! - number_to_date 家族 → new SimpleDate(new Date(safeToLong(num)), dateType)
//!   （safeToLong：HALF_UP 取整 + 64 位范围检查）。

use crate::builtins::eval_util::target_string;
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::value::{DateType, DateValue, TNumber};
use bigdecimal::BigDecimal;
use chrono::TimeZone;
use std::str::FromStr;

/// ?abs —— Java absBI（BuiltInsForNumbers.java:57）：负数取绝对值，正数原样
pub fn abs(env: &mut Environment, target: &Expr, _args: Option<&[Expr]>) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    let out = match &n {
        TNumber::Int(v) => {
            if *v < 0 {
                TModel::from_number(TNumber::Int(v.wrapping_neg()))
            } else {
                return Ok(Some(m));
            }
        }
        TNumber::Long(v) => {
            if *v < 0 {
                TModel::from_number(TNumber::Long(v.wrapping_neg()))
            } else {
                return Ok(Some(m));
            }
        }
        TNumber::BigInt(v) => {
            if v.sign() == num_bigint::Sign::Minus {
                TModel::from_number(TNumber::BigInt(-v.clone()))
            } else {
                return Ok(Some(m));
            }
        }
        TNumber::Decimal(d) => {
            if d.sign() == num_bigint::Sign::Minus {
                TModel::from_number(TNumber::Decimal(-d.clone()))
            } else {
                return Ok(Some(m));
            }
        }
        TNumber::Float(v) => {
            if *v < 0.0 {
                TModel::from_number(TNumber::Float(-v))
            } else {
                return Ok(Some(m));
            }
        }
        TNumber::Double(v) => {
            if *v < 0.0 {
                TModel::from_number(TNumber::Double(-v))
            } else {
                return Ok(Some(m));
            }
        }
    };
    Ok(Some(out))
}

/// 取整（Java ceilingBI/floorBI/roundBI：BigDecimal(num.doubleValue()).divide(1, 0, mode)）
fn round_impl(env: &mut Environment, target: &Expr, mode: RoundKind) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    let d = n.as_big_decimal();
    let one = BigDecimal::from(1i32);
    let out = match mode {
        RoundKind::Ceiling => (d / &one).with_scale_round(0, bigdecimal::RoundingMode::Ceiling),
        RoundKind::Floor => (d / &one).with_scale_round(0, bigdecimal::RoundingMode::Floor),
        RoundKind::Round => {
            // Java：new BigDecimal(doubleValue).add(0.5).divide(1, 0, ROUND_FLOOR)
            let half = BigDecimal::from_str("0.5").unwrap();
            ((d + half) / one).with_scale_round(0, bigdecimal::RoundingMode::Floor)
        }
    };
    Ok(Some(TModel::from_number(TNumber::Decimal(out))))
}

#[derive(Clone, Copy)]
enum RoundKind {
    Ceiling,
    Floor,
    Round,
}

pub fn ceiling(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    round_impl(env, target, RoundKind::Ceiling)
}

pub fn floor(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    round_impl(env, target, RoundKind::Floor)
}

pub fn round(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    round_impl(env, target, RoundKind::Round)
}

/// ?byte —— Java byteBI：Byte 转换（截断到 8 位）
pub fn byte(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    match &n {
        TNumber::Int(v) if (*v as i8 as i32) == *v => return Ok(Some(m)),
        _ => {}
    }
    // Java num.byteValue()：截断（向零）后按 8 位回绕 —— 用 trunc_i64 截断
    // （f64 越界饱和同 JVM d2l），再 `as i8` 回绕（不同于 `as f64 as i8` 的饱和）
    Ok(Some(TModel::from_number(TNumber::Int(
        crate::core::eval::trunc_i64(&n).unwrap_or(0) as i8 as i32,
    ))))
}

/// ?short —— Java shortBI：Short 转换（截断到 16 位）
pub fn short(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    match &n {
        TNumber::Int(v) if (*v as i16 as i32) == *v => return Ok(Some(m)),
        _ => {}
    }
    // Java num.shortValue()：截断（向零）后按 16 位回绕
    Ok(Some(TModel::from_number(TNumber::Int(
        crate::core::eval::trunc_i64(&n).unwrap_or(0) as i16 as i32,
    ))))
}

/// ?is_nan —— Java NumberUtil.isNaN
pub fn is_nan(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    let b = match &n {
        TNumber::Float(v) => v.is_nan(),
        TNumber::Double(v) => v.is_nan(),
        _ => false,
    };
    Ok(Some(TModel::from_boolean(b)))
}

/// ?is_infinite —— Java NumberUtil.isInfinite
pub fn is_infinite(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    let b = match &n {
        TNumber::Float(v) => v.is_infinite(),
        TNumber::Double(v) => v.is_infinite(),
        _ => false,
    };
    Ok(Some(TModel::from_boolean(b)))
}

/// 列号 → 字母（Java StringUtil.toLowerABC/toUpperABC：1→a、26→z、27→aa）
fn to_abc(n: i64, upper: bool) -> Result<String> {
    if n <= 0 {
        return Err(TemplateError::misc(format!(
            "The left side operand of to ?{} must be at least 1, but was {n}.",
            if upper { "upper_abc" } else { "lower_abc" }
        )));
    }
    let mut out = Vec::new();
    let mut v = n;
    while v > 0 {
        v -= 1;
        let c = (v % 26) as u8 + if upper { b'A' } else { b'a' };
        out.push(c as char);
        v /= 26;
    }
    out.reverse();
    Ok(out.into_iter().collect())
}

fn abc_impl(env: &mut Environment, target: &Expr, upper: bool) -> Result<Option<TModel>> {
    let key = if upper { "upper_abc" } else { "lower_abc" };
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    // Java abcBI（BuiltInsForNumbers.java）：NumberUtil.toIntExact —— 非整数值报错
    // "Can't convert 1.00001 to type Integer without loss."
    let i = crate::core::eval::to_int_exact(&n).ok_or_else(|| {
        TemplateError::misc(format!(
            "The left side operand value isn't compatible with ?{key}: Can't convert {} to type Integer without loss.",
            n.to_plain_string()
        ))
    })?;
    Ok(Some(TModel::from_scalar(to_abc(i, upper)?)))
}

pub fn lower_abc(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    abc_impl(env, target, false)
}

pub fn upper_abc(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    abc_impl(env, target, true)
}

/// ?number_to_date / ?number_to_time / ?number_to_datetime —— Java number_to_dateBI：
/// 毫秒时间戳（safeToLong：HALF_UP 取整 + 范围检查）→ 日期
fn number_to_date_impl(
    env: &mut Environment,
    target: &Expr,
    kind: DateType,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let n = m.get_number()?;
    let millis = match &n {
        // Java safeToLong：Double/Float HALF_UP 取整 + 64 位范围检查（超范围报错）
        TNumber::Double(v) => {
            let d = v.round();
            if d > i64::MAX as f64 || d < i64::MIN as f64 {
                return Err(TemplateError::misc(format!(
                    "Number doesn't fit into a 64 bit signed integer (long): {d}"
                )));
            }
            d as i64
        }
        TNumber::Float(v) => {
            let d = v.round();
            if d > i64::MAX as f32 || d < i64::MIN as f32 {
                return Err(TemplateError::misc(format!(
                    "Number doesn't fit into a 64 bit signed integer (long): {d}"
                )));
            }
            d as i64
        }
        TNumber::Decimal(d) => {
            let rounded = d.with_scale_round(0, bigdecimal::RoundingMode::HalfUp);
            let i = rounded.as_bigint_and_scale().0;
            i64::try_from(i.as_ref()).map_err(|_| {
                TemplateError::misc("Number doesn't fit into a 64 bit signed integer (long)")
            })?
        }
        other => crate::core::eval::trunc_i64(other).ok_or_else(|| {
            TemplateError::misc("Number doesn't fit into a 64 bit signed integer (long)")
        })?,
    };
    let dt = chrono::Utc
        .timestamp_millis_opt(millis)
        .single()
        .ok_or_else(|| {
            TemplateError::misc("Number doesn't fit into a 64 bit signed integer (long)")
        })?
        .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
    Ok(Some(TModel::from_date(DateValue::new(dt, kind))))
}

pub fn number_to_date(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    number_to_date_impl(env, target, DateType::Date)
}

pub fn number_to_time(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    number_to_date_impl(env, target, DateType::Time)
}

pub fn number_to_datetime(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    number_to_date_impl(env, target, DateType::DateTime)
}

// 兼容引用（target_string 在本文件用于字符串目标的数字内建——v1 不适用，保留占位）
#[allow(dead_code)]
fn _unused(env: &mut Environment, target: &Expr) -> Result<String> {
    target_string(env, target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abc_conversion() {
        assert_eq!(to_abc(1, false).unwrap(), "a");
        assert_eq!(to_abc(26, false).unwrap(), "z");
        assert_eq!(to_abc(27, false).unwrap(), "aa");
        assert_eq!(to_abc(28, false).unwrap(), "ab");
        assert_eq!(to_abc(100, false).unwrap(), "cv");
        assert_eq!(to_abc(1, true).unwrap(), "A");
        assert!(to_abc(0, false).is_err());
    }

    /// 渲染辅助（?lower_abc/?upper_abc 的 Java 错误消息逐字核对，jar 实测）
    fn render_out(src: &str) -> crate::error::Result<String> {
        use crate::cache::StringLoader;
        use crate::template::{Configuration, ObjectWrapper, SimpleObjectWrapper};
        let mut c = Configuration::new();
        let loader = std::sync::Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", src);
        let t = c.get_template("t.ftl")?;
        let root = SimpleObjectWrapper
            .wrap(&crate::template::DynValue::Map(vec![]))?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn abc_exact_integer_semantics() {
        // Java abcBI（BuiltInsForNumbers.java）：NumberUtil.toIntExact——
        // 非整数值报 "Can't convert 1.00001 to type Integer without loss."
        let err = render_out("${1.00001?lower_abc}").unwrap_err().to_string();
        assert!(
            err.contains(
                "The left side operand value isn't compatible with ?lower_abc: Can't convert 1.00001 to type Integer without loss."
            ),
            "{err}"
        );
        // n <= 0 → "must be at least 1, but was {n}."（含 '0'/'at least 1'）
        let err = render_out("<#assign n=-1>${n?lower_abc}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("The left side operand of to ?lower_abc must be at least 1, but was -1."),
            "{err}"
        );
        let err = render_out("${0?lower_abc}").unwrap_err().to_string();
        assert!(
            err.contains("The left side operand of to ?lower_abc must be at least 1, but was 0."),
            "{err}"
        );
        // 整数值浮点（1.0）可接受
        assert_eq!(render_out("${1.0?lower_abc}").unwrap(), "a");
    }
}
