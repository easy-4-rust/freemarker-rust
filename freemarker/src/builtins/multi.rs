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

use crate::builtins::eval_util::{arg_count, arg_string, check_arg_count, coerce_to_string};
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
    // Java stringBI（BuiltInsForMultipleTypes.java:540）：非数字/布尔/日期/字符串 →
    // `For "?string" left-hand operand: Expected a number, date, boolean or string,
    // but this has evaluated to a {type}: ==> {target}`（jar 实测 type_string_seq 基线）
    Err(TemplateError::type_mismatch("string", m.type_name)
        .with_expected_phrase("a number, date, boolean or string")
        .with_blame_at(
            "?string",
            "left-hand operand",
            &crate::core::environment::expr_desc(target),
            &env.current_template_name,
            target.span,
        ))
}

/// Java _MessageUtil.newCantFormatUnknownTypeDateException（_MessageUtil.java:38-45/
/// 309-315：UNKNOWN_DATE_TO_STRING_ERROR_MESSAGE + UNKNOWN_DATE_TO_STRING_TIPS；
/// dateformat-iso-like 的 "Use ?date..." 与 date-type-builtins 的 "isn't known if" 断言）
fn unknown_date_type_error() -> TemplateError {
    TemplateError::misc(
        "Can't convert the date-like value to string because it isn't known if it's a date (no time part), time or date-time value.\n\n----\nTip: Use ?date, ?time, or ?datetime to tell FreeMarker the exact type.\n----\nTip: If you need a particular format only once, use ?string(pattern), like ?string('dd.MM.yyyy HH:mm:ss'), to specify which fields to display. \n----",
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

/// ?is_unknown_date_like —— Java is_dateOfTypeBI(UNKNOWN)（BuiltInsForMultipleTypes.java:291-305）
pub fn is_unknown_date_like(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    Ok(Some(TModel::from_boolean(is_type_test(
        env,
        target,
        |m| m.is_date() && m.get_date().is_ok_and(|d| d.kind == DateType::Unknown),
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

/// ?absolute_template_name —— Java BuiltInsForMultipleTypes.absolute_template_nameBI
/// 将相对模板路径解析为绝对路径。名称含 `/` → 已是绝对路径；否则拼接当前模板的目录前缀。
pub fn absolute_template_name(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("absolute_template_name", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    let name = coerce_to_string(env, &m)?;
    // 含 '/' → 已是绝对路径
    if name.contains('/') {
        return Ok(Some(TModel::from_scalar(name)));
    }
    // 否则拼接当前模板的目录前缀
    let current = &env.current_template_name;
    let dir = match current.rfind('/') {
        Some(pos) => &current[..=pos],
        None => "",
    };
    Ok(Some(TModel::from_scalar(format!("{dir}{name}"))))
}

/// ?api —— Java BuiltInsForMultipleTypes.apiBI：BeanWrapper API 访问。
/// Rust 侧不支持反射 API，始终返回错误（与 Java SimpleObjectWrapper 行为一致）。
pub fn api(
    _env: &mut Environment,
    _target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("api", args, 0, 0)?;
    Err(TemplateError::misc(
        "The \"?api\" built-in is only available when the object wrapper supports it, but the current object wrapper (SimpleObjectWrapper) doesn't."
    ))
}

/// ?markup_string —— Java BuiltInsForMarkupOutput.markup_string
/// 若目标是 markup 输出 → 提取其底层标量字符串；否则原样返回。
pub fn markup_string(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("markup_string", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.is_markup_output() {
        // 提取 markup 输出中的底层字符串
        if let Some(s) = &m.scalar {
            return Ok(Some(TModel::from_scalar(s.as_string()?)));
        }
    }
    // 非 markup 输出 → 原样返回
    Ok(Some(m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::ObjectWrapper;
    use crate::template::{Configuration, DynValue, SimpleObjectWrapper};
    use crate::value::TNumber;
    use std::sync::Arc;

    /// 渲染 `${src}` 返回输出字符串
    fn eval_out(root: DynValue, src: &str, cur_template: &str) -> Result<String> {
        let mut c = Configuration::new();
        c.settings.boolean_format = "c".to_string();
        c.settings.number_format = "0.#########".to_string();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put(cur_template, &format!("${{{src}}}"));
        let t = c.get_template(cur_template)?;
        let root_model = SimpleObjectWrapper
            .wrap(&root)?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn no_root() -> DynValue {
        DynValue::Map(vec![])
    }

    #[test]
    fn c_format_number_variants() {
        assert_eq!(format_c_number(&TNumber::Int(3)), "3");
        assert_eq!(format_c_number(&TNumber::Double(1.5)), "1.5");
    }

    #[test]
    fn api_returns_error() {
        let err = eval_out(no_root(), "'hello'?api", "t.ftl").unwrap_err();
        assert!(err.to_string().contains("?api"), "{err}");
        assert!(err.to_string().contains("SimpleObjectWrapper"), "{err}");
    }

    #[test]
    fn absolute_template_name_absolute() {
        // 名称含 '/' → 原样返回
        assert_eq!(
            eval_out(no_root(), "'/abs/path.ftl'?absolute_template_name", "t.ftl").unwrap(),
            "/abs/path.ftl"
        );
        assert_eq!(
            eval_out(no_root(), "'sub/dir/t.ftl'?absolute_template_name", "t.ftl").unwrap(),
            "sub/dir/t.ftl"
        );
    }

    #[test]
    fn absolute_template_name_relative() {
        // 相对名称 → 拼接当前模板的目录前缀
        assert_eq!(
            eval_out(
                no_root(),
                "'child.ftl'?absolute_template_name",
                "base/t.ftl"
            )
            .unwrap(),
            "base/child.ftl"
        );
        assert_eq!(
            eval_out(no_root(), "'x.ftl'?absolute_template_name", "a/b/c.ftl").unwrap(),
            "a/b/x.ftl"
        );
    }

    #[test]
    fn absolute_template_name_no_directory() {
        // 当前模板名不含 '/' → 直接拼接
        assert_eq!(
            eval_out(no_root(), "'other.ftl'?absolute_template_name", "root.ftl").unwrap(),
            "other.ftl"
        );
    }

    #[test]
    fn markup_string_non_markup() {
        // 非 markup 目标 → 原样返回
        assert_eq!(
            eval_out(no_root(), "'hello'?markup_string", "t.ftl").unwrap(),
            "hello"
        );
    }
}
