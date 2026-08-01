//! 多类型内建 —— 对应 Java `BuiltInsForMultipleTypes.java`（string/c/cn/is_collection_ex/
//! is_date_only/is_time/is_datetime/is_unknown_date_like/namespace）
//!
//! 语义要点（Java 对照）：
//! - `?string` → stringBI：数字 → 按 number_format（无参）或 pattern（1 参）格式化；
//!   布尔 → boolean_format（无参）或 2 参 `?string('t','f')` 二选一；
//!   标量 → 原样；日期 → 按 date_format（v1 仅支持引号字面量与 iso/xs 名称——P4）；
//! - `?c`/`?cn` → AbstractCLikeBI：数字 → C 格式（format.rs）；布尔 → true/false；
//!   字符串 → CFormat.formatString（IcI ≥ 2.3.32 默认 JavaScriptOrJSONCFormat →
//!   jsStringEnc(JAVA_SCRIPT_OR_JSON, QUOTATION_MARK)）；?c 对 null 报错、?cn 返回 "null"；
//! - `?is_date_only` 等 → is_dateOfTypeBI：按 DateValue.kind 判定；
//! - `?namespace` → namespaceBI（v1：命名空间值返回命名空间模型；非变量目标报错）。

use crate::builtins::eval_util::{arg_count, arg_string, check_arg_count};
use crate::builtins::format::{format_c_number, format_number, format_number_with};
use crate::builtins::strings_encoding::{java_string_enc, js_string_enc};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use crate::value::DateType;

/// ?string —— Java BuiltInsForMultipleTypes.stringBI（BuiltInsForMultipleTypes.java:540）
pub fn string(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    let argc = arg_count(args);
    if let Some(b) = &m.boolean {
        let b = b.as_boolean()?;
        if argc == 2 {
            // Java BooleanFormatter.exec：args[bool ? 0 : 1]
            let a0 = arg_string(env, args, 0)?;
            let a1 = arg_string(env, args, 1)?;
            return Ok(Some(TModel::from_scalar(if b { a0 } else { a1 })));
        }
        if argc == 0 {
            // Java BooleanFormatter.getAsString（BuiltInsForMultipleTypes.java:561）：
            // 若布尔模型同时是标量模型（bool instanceof TemplateScalarModel），
            // 优先返回其字符串表示（如 booleanAndString → "theStringValue"），
            // 否则才按 boolean_format 格式化
            if let Some(s) = &m.scalar {
                return Ok(Some(TModel::from_scalar(s.as_string()?)));
            }
            return Ok(Some(TModel::from_scalar(
                crate::core::environment::boolean_format(env, b, true)?,
            )));
        }
        return Err(TemplateError::misc(
            "?string expects 0 or 2 arguments for booleans",
        ));
    }
    if let Some(n) = &m.number {
        let n = n.as_number()?;
        if argc == 0 {
            return Ok(Some(TModel::from_scalar(format_number(env, &n))));
        }
        if argc == 1 {
            let fmt = arg_string(env, args, 0)?;
            return Ok(Some(TModel::from_scalar(format_number_with(
                &fmt,
                &env.settings.locale,
                &n,
            ))));
        }
        return Err(TemplateError::misc(
            "?string expects 0 or 1 arguments for numbers",
        ));
    }
    if let Some(d) = &m.date {
        let d = d.as_date()?;
        // Java DateFormatter（BuiltInsForMultipleTypes.java:580-634）：
        // 无参 → date_format/time_format/datetime_format 设置；1 参 → 显式格式串
        // （"xs"/"iso"/命名模式/Java 模式；`?string.xs` 点形式由 eval.rs eval_dot 转参数）
        if d.kind == crate::value::DateType::Unknown {
            // Java：UNKNOWN 类型 + 显式 Java 模式可用（JavaTemplateDateFormatFactory 仅命名
            // 风格报 UnknownDateTypeFormattingUnsupportedException，:66-86）；
            // ISO/XS/命名风格/无参 → newCantFormatUnknownTypeDateException（dateformat-iso-like 用例）
            if argc == 1 {
                let fmt = arg_string(env, args, 0)?;
                let named = crate::builtins::java_date_format::resolve_named_style(
                    &fmt,
                    crate::value::DateType::DateTime,
                    &env.settings.locale,
                )
                .is_some();
                if !named && crate::builtins::iso_date_format::is_iso_like(&fmt).is_none() {
                    return Ok(Some(TModel::from_scalar(
                        crate::core::environment::format_date_value(env, &d, &fmt)?,
                    )));
                }
            }
            return Err(unknown_date_type_error());
        }
        if argc == 0 {
            let format = match d.kind {
                crate::value::DateType::Date => env.settings.date_format.clone(),
                crate::value::DateType::Time => env.settings.time_format.clone(),
                crate::value::DateType::DateTime => env.settings.date_time_format.clone(),
                crate::value::DateType::Unknown => unreachable!(), // 上方已报错
            };
            return Ok(Some(TModel::from_scalar(
                crate::core::environment::format_date_value(env, &d, &format)?,
            )));
        }
        if argc == 1 {
            let fmt = arg_string(env, args, 0)?;
            return Ok(Some(TModel::from_scalar(
                crate::core::environment::format_date_value(env, &d, &fmt)?,
            )));
        }
        return Err(TemplateError::misc(
            "?string expects 0 or 1 arguments for dates",
        ));
    }
    if let Some(s) = &m.scalar {
        return Ok(Some(TModel::from_scalar(s.as_string()?)));
    }
    Err(TemplateError::misc(format!(
        "?string is not applicable to a {} value",
        m.type_name
    )))
}

/// Java _MessageUtil.newCantFormatUnknownTypeDateException（dateformat-iso-like 用例）
fn unknown_date_type_error() -> TemplateError {
    TemplateError::misc(
        "The value of the following has unknown date type, but ?string.xs needs a value where it's known if it's a date (no time part), time, or date-time value. Use ?date, ?time, or ?datetime built-ins to specify the date type explicitly.",
    )
}

/// ?c —— Java cBI（AbstractCLikeBI）：数字/布尔/字符串；null → InvalidReference
pub fn c(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("c", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(n) = &m.number {
        return Ok(Some(TModel::from_scalar(format_c_number(&n.as_number()?))));
    }
    if let Some(b) = &m.boolean {
        return Ok(Some(TModel::from_scalar(
            if b.as_boolean()? { "true" } else { "false" }.to_string(),
        )));
    }
    if let Some(s) = &m.scalar {
        // JavaScriptOrJSONCFormat.formatString：jsStringEnc(JS_OR_JSON, QUOTATION_MARK)
        return Ok(Some(TModel::from_scalar(js_string_enc(
            &s.as_string()?,
            true,
        ))));
    }
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    Err(TemplateError::misc(format!(
        "?c is not applicable to a {} value",
        m.type_name
    )))
}

/// ?cn —— Java cnBI：同 ?c，但 null → "null"（CFormat.getNullString）
pub fn cn(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("cn", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Ok(Some(TModel::from_scalar("null".to_string())));
    }
    c(env, target, Some(&[]))
}

/// ?is_collection_ex —— Java is_collection_exBI
pub fn is_collection_ex(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_boolean(is_type_test(
        env,
        target,
        |m| m.is_collection_ex(),
    )?)))
}

/// ?is_date_only —— Java is_dateOfTypeBI(TemplateDateModel.DATE)
pub fn is_date_only(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_boolean(is_type_test(
        env,
        target,
        |m| m.is_date() && m.get_date().is_ok_and(|d| d.kind == DateType::Date),
    )?)))
}

/// ?is_time —— Java is_dateOfTypeBI(TemplateDateModel.TIME)
pub fn is_time(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_boolean(is_type_test(
        env,
        target,
        |m| m.is_date() && m.get_date().is_ok_and(|d| d.kind == DateType::Time),
    )?)))
}

/// ?is_datetime —— Java is_dateOfTypeBI(TemplateDateModel.DATETIME)
pub fn is_datetime(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_boolean(is_type_test(
        env,
        target,
        |m| m.is_date() && m.get_date().is_ok_and(|d| d.kind == DateType::DateTime),
    )?)))
}

/// ?is_unknown_date_like —— Java is_dateOfTypeBI(UNKNOWN)；v1 DateValue.kind 恒已知 → false
pub fn is_unknown_date_like(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_boolean(is_type_test(
        env,
        target,
        |_| false,
    )?)))
}

/// 类型测试（缺失 → false；其他错误上传；Java is_*BI 语义）
fn is_type_test(
    env: &mut Environment,
    target: &Expr,
    test: impl Fn(&TModel) -> bool,
) -> Result<bool> {
    match crate::core::eval::eval(env, target) {
        Ok(m) => Ok(test(&m)),
        Err(TemplateError::InvalidReference { .. }) => Ok(false),
        Err(e) => Err(e),
    }
}

/// ?namespace —— Java namespaceBI：目标所在命名空间（v1：目标为命名空间值则返回自身）
pub fn namespace(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if let Some(ns) = env.as_namespace(&m) {
        return Ok(Some(crate::core::environment::namespace_model(ns)));
    }
    Err(TemplateError::misc(format!(
        "?namespace is not applicable to a {} value (v1 only supports namespace values)",
        m.type_name
    )))
}

// 兼容引用（java_string_enc 供 ?c 的 Java CFormat 未来对齐；当前用 JSON 风格）
#[allow(dead_code)]
fn _java_c_format_ref(s: &str) -> String {
    java_string_enc(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::TNumber;

    #[test]
    fn c_format_number_variants() {
        assert_eq!(format_c_number(&TNumber::Int(3)), "3");
        assert_eq!(format_c_number(&TNumber::Double(1.5)), "1.5");
    }
}
