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

use crate::builtins::format::{
    format_c_number, format_c_string, format_number, format_number_with, CFormatKind,
};
use crate::builtins::strings_encoding::java_string_enc;
use crate::cache::TemplateNameFormat;
use crate::core::eval_util::{arg_count, arg_string, check_arg_count, coerce_to_string};
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
            return Ok(Some(TModel::from_scalar(format_number(env, &n)?)));
        }
        if argc == 1 {
            let fmt = arg_string(env, args, 0)?;
            return Ok(Some(TModel::from_scalar(format_number_with(
                &fmt,
                &env.settings.locale,
                &n,
            )?)));
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
    let c_format = env.settings.c_format;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(n) = &m.number {
        return Ok(Some(TModel::from_scalar(format_c_number(
            &n.as_number()?,
            c_format,
        ))));
    }
    if let Some(b) = &m.boolean {
        return Ok(Some(TModel::from_scalar(
            if b.as_boolean()? { "true" } else { "false" }.to_string(),
        )));
    }
    if let Some(s) = &m.scalar {
        // CFormat.formatString（按 c_format 变体：JS_OR_JSON/JS/Java/XS 转义）
        return Ok(Some(TModel::from_scalar(format_c_string(
            &s.as_string()?,
            c_format,
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

/// ?cn —— Java cnBI：同 ?c，但 null → getNullString（默认 "null"；XSCFormat → ""）
pub fn cn(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("cn", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        // XSCFormat.getNullString() = ""（XSCFormat.java:67-70：XSD 无 null 字面量）
        let null_str = if env.settings.c_format == CFormatKind::Xs {
            ""
        } else {
            "null"
        };
        return Ok(Some(TModel::from_scalar(null_str.to_string())));
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

/// ?absolute_template_name —— Java BuiltInsForStringsMisc.absolute_template_nameBI
/// （BuiltInsForStringsMisc.java:143-185）：把目标字符串解析为绝对模板名。
/// 语义：`resolvePath(base)` = `rootBasedToAbsoluteName(toFullTemplateName(base, target))`
/// （Environment.java:3326-3352 → TemplateNameFormat）——目标含 "://"（位置 > 0）
/// 或为带 scheme 基准的绝对路径 → scheme 名原样；"/" 开头 → 基准的 scheme 前缀
/// （无 scheme 则去前导 "/"）；相对 → 基准所在目录拼接。
/// 无实参 → 基准 = 表达式所在模板名（Java `getTemplate().getName()` —— 词法模板，
/// :165-167）；1 个实参 → 基准 = 该实参（Java 方法形态 AbsoluteTemplateNameResult.exec
/// :158-162，checkMethodArgCount(args, 1)）。
pub fn absolute_template_name(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    let name = coerce_to_string(env, &m)?;
    let base = match args {
        // Java 方法形态：`'a/b'?absolute_template_name('dir/f')`（exec(args) 恰 1 实参）
        Some(as_) => {
            if as_.len() != 1 {
                return Err(TemplateError::misc(format!(
                    "?absolute_template_name(...) expects 1 argument but has received {}.",
                    if as_.is_empty() {
                        "none".to_string()
                    } else {
                        as_.len().to_string()
                    }
                )));
            }
            arg_string(env, args, 0)?
        }
        // Java 标量形态：基准 = 表达式所在模板名（getTemplate().getName()）
        None => env.lexical_template_name.clone(),
    };
    Ok(Some(TModel::from_scalar(resolve_template_path(
        &name, &base,
    )?)))
}

/// 绝对名解析（Java absolute_template_nameBI.resolvePath，BuiltInsForStringsMisc.
/// java:172-181）：toFullTemplateName（baseName==null → 原样）→
/// rootBasedToAbsoluteTemplateName
fn resolve_template_path(path_to_resolve: &str, base: &str) -> Result<String> {
    // Java Environment.toFullTemplateName（:3326-3333）：isClassicCompatible ||
    // baseName==null → 原样返回（无名模板 —— v1 无名模板名 ""，此处按 null 处理）
    let full = if base.is_empty() {
        path_to_resolve.to_string()
    } else {
        crate::cache::NameFormatDefault020300.to_root_based_name(base, path_to_resolve)?
    };
    // Java rootBasedToAbsoluteTemplateName（Environment.java:3350-3352）
    crate::cache::NameFormatDefault020300.root_based_name_to_absolute_name(&full)
}

/// ?api —— Java BuiltInsForMultipleTypes.apiBI：对象包装器 API 访问。
/// Java 侧为反射 API 表面（TemplateModelWithAPISupport.getAPI）；Rust 引擎自身
/// 不支持反射——目标模型带 `api` 槽位（由包装方提供视图）时返回 API 视图，
/// 否则与 Java SimpleObjectWrapper（无 API 支持）行为一致报错。
pub fn api(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("api", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(api) = &m.api {
        return Ok(Some(api.api_view()?));
    }
    Err(TemplateError::misc(
        "The \"?api\" built-in is only available when the object wrapper supports it, but the current object wrapper (SimpleObjectWrapper) doesn't."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::ObjectWrapper;
    use crate::template::{Configuration, DynValue, SimpleObjectWrapper};
    use crate::value::TNumber;
    use indexmap::IndexMap;
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
        assert_eq!(
            format_c_number(&TNumber::Int(3), CFormatKind::JavaScriptOrJson),
            "3"
        );
        assert_eq!(
            format_c_number(&TNumber::Double(1.5), CFormatKind::JavaScriptOrJson),
            "1.5"
        );
    }

    #[test]
    fn api_returns_error() {
        let err = eval_out(no_root(), "'hello'?api", "t.ftl").unwrap_err();
        assert!(err.to_string().contains("?api"), "{err}");
        assert!(err.to_string().contains("SimpleObjectWrapper"), "{err}");
    }

    #[test]
    fn api_returns_view_and_has_api() {
        // 带 api 槽位的模型：?api 返回视图（Map API 的 get 方法），?has_api true
        use crate::template::TemplateApiSupport;
        struct View;
        impl TemplateApiSupport for View {
            fn api_view(&self) -> Result<TModel> {
                Ok(TModel::from_scalar("view".to_string()))
            }
        }
        // api 槽位挂在目标值上（?api 求值 x 后返回 api_view；?has_api 按槽位判定）
        let mut x = TModel::from_scalar("val".to_string());
        x.api = Some(std::rc::Rc::new(View));
        let mut h = IndexMap::new();
        h.insert("x".to_string(), x);
        let m = TModel::from_hash(h);
        // ?api 求值 x 后返回 api_view；?has_api 按 api 槽位判定
        let mut c = Configuration::new();
        c.settings.boolean_format = "c".to_string();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", "${x?api} ${x?has_api}");
        let t = c.get_template("t.ftl").unwrap();
        let mut out = Vec::new();
        t.process(m, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "view true");
    }

    #[test]
    fn has_api_false_without_slot() {
        assert_eq!(
            eval_out(no_root(), "'s'?has_api", "t.ftl").unwrap(),
            "false"
        );
        assert_eq!(eval_out(no_root(), "1?has_api", "t.ftl").unwrap(), "false");
    }

    #[test]
    fn absolute_template_name_absolute() {
        // Java AbsoluteTemplateNameBITest.basicsTest：'/a/b' → 绝对路径去前导 "/" 后
        // 转绝对名（"/a/b"）；相对名 → 根相对转绝对（前加 "/"）
        assert_eq!(
            eval_out(no_root(), "'/abs/path.ftl'?absolute_template_name", "t.ftl").unwrap(),
            "/abs/path.ftl"
        );
        assert_eq!(
            eval_out(no_root(), "'sub/dir/t.ftl'?absolute_template_name", "t.ftl").unwrap(),
            "/sub/dir/t.ftl"
        );
    }

    #[test]
    fn absolute_template_name_relative() {
        // 相对名称 → 以当前模板所在目录为基准解析（Java toFullTemplateName）
        assert_eq!(
            eval_out(
                no_root(),
                "'child.ftl'?absolute_template_name",
                "base/t.ftl"
            )
            .unwrap(),
            "/base/child.ftl"
        );
        assert_eq!(
            eval_out(no_root(), "'x.ftl'?absolute_template_name", "a/b/c.ftl").unwrap(),
            "/a/b/x.ftl"
        );
    }

    #[test]
    fn absolute_template_name_no_directory() {
        // 当前模板名不含 '/' → 根相对解析
        assert_eq!(
            eval_out(no_root(), "'other.ftl'?absolute_template_name", "root.ftl").unwrap(),
            "/other.ftl"
        );
    }

    #[test]
    fn absolute_template_name_with_base_arg() {
        // 方法形态：'a/b'?absolute_template_name('dir/f') → 以实参为基准
        // （Java AbsoluteTemplateNameBITest.basicsTest :52-57）
        for base in ["dir/f", "/dir/f", "dir/", "/dir/"] {
            assert_eq!(
                eval_out(
                    no_root(),
                    &format!("'a/b'?absolute_template_name('{base}')"),
                    "t.ftl"
                )
                .unwrap(),
                "/dir/a/b",
                "base: {base}"
            );
        }
        // 基准带 scheme → 相对名以 scheme 目录为基准、绝对名保留 scheme 前缀
        assert_eq!(
            eval_out(
                no_root(),
                "'a/b'?absolute_template_name('schema://dir/f')",
                "t.ftl"
            )
            .unwrap(),
            "schema://dir/a/b"
        );
        assert_eq!(
            eval_out(
                no_root(),
                "'/a/b'?absolute_template_name('schema://dir/f')",
                "t.ftl"
            )
            .unwrap(),
            "schema://a/b"
        );
    }

    #[test]
    fn markup_string_non_markup_errors() {
        // Java BuiltInForMarkupOutput._eval（BuiltInForMarkupOutput.java:31-34）：
        // 目标非 markup 输出 → NonMarkupOutputException（"left-hand operand:
        // Expected a markup output value..."）；v1 旧实现原样返回目标（已修正）
        let err = eval_out(no_root(), "'hello'?markup_string", "t.ftl").unwrap_err();
        assert!(err.to_string().contains("markup output"), "{err}");
    }
}
