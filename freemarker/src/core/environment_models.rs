//! 模型值构造与下沉、输出字符串化与格式化分派（docs/09 §2）
//! （对应 Java EvalUtil.coerceModelToStringOrMarkup / Environment.getTemplateDateFormat 家族）。

use super::environment_loop::LocalEntry;
use super::environment_macros::{LambdaValue, MacroValue, Namespace};
use super::Environment;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateHashModel, TemplateHashModelEx};
use indexmap::IndexMap;
use std::rc::Rc;

/// 命名空间值模型（Java Namespace 作为 TemplateHashModel 值；FTL 类型 extended_hash）
pub fn namespace_model(ns: Rc<Namespace>) -> TModel {
    let h: Rc<dyn TemplateHashModel> = ns.clone();
    let ex: Rc<dyn TemplateHashModelEx> = ns.clone();
    TModel {
        hash: Some(h),
        hash_ex: Some(ex),
        internal: Some(ns),
        type_name: "extended_hash",
        kind: crate::template::ModelKind::Hash,
        ..TModel::nothing()
    }
}

/// `.vars` 快照 —— 对应 Java `BuiltinVariable.VarsHash`（BuiltinVariable.java:330-337：
/// get(key) = env.getVariable 完整解析链）。v1 以快照近似活视图：按
/// 局部上下文（循环变量）> 宏帧局部变量 > 当前命名空间 > 全局命名空间 > 根模型 >
/// 共享变量的优先级合并（低优先级先入、高优先级覆盖）。
pub fn vars_snapshot(env: &Environment) -> IndexMap<String, TModel> {
    let mut map = IndexMap::new();
    // 根数据模型
    if let Some(ex) = &env.root.hash_ex {
        let keys = ex.keys().unwrap_or_default();
        for k in keys {
            if let Ok(Some(v)) = ex.get(&k) {
                map.insert(k, v);
            }
        }
    }
    // 共享变量
    for (k, v) in &env.template.configuration.shared_vars {
        map.insert(k.clone(), v.clone());
    }
    // 全局命名空间
    for k in env.get_global_namespace().var_names() {
        if let Some(v) = env.get_global_namespace().get_member(&k) {
            map.insert(k, v);
        }
    }
    // 当前命名空间
    for k in env.get_current_namespace().var_names() {
        if let Some(v) = env.get_current_namespace().get_member(&k) {
            map.insert(k, v);
        }
    }
    // 宏帧局部变量
    if let Some(frame) = env.macro_frames.last() {
        for (k, v) in frame.locals.borrow().iter() {
            map.insert(k.clone(), v.clone());
        }
    }
    // 局部上下文（循环变量/body 参数）
    for entry in env.local_stack.iter().rev() {
        if let LocalEntry::Loop(lc) = entry {
            let c = lc.borrow();
            if !c.var_name.is_empty() {
                if let Some(v) = c.get(&c.var_name, env.settings.fallback_on_null_loop_variable) {
                    map.insert(c.var_name.clone(), v);
                }
                if let Some(v2) = &c.var2_name {
                    if let Some(v) = c.get(v2, env.settings.fallback_on_null_loop_variable) {
                        map.insert(v2.clone(), v);
                    }
                }
            }
        } else if let LocalEntry::Body(bc) = entry {
            for (k, v) in bc.vars.iter() {
                map.insert(k.clone(), v.clone());
            }
        }
    }
    map
}

/// 宏/函数值模型（Java Macro 对象；`?is_macro` 依据 kind 判定）
pub fn macro_model(mv: Rc<MacroValue>) -> TModel {
    TModel {
        internal: Some(mv),
        type_name: "macro",
        kind: crate::template::ModelKind::Macro,
        ..TModel::nothing()
    }
}

/// lambda 值模型（Java LocalLambdaExpression 求值结果；v1 仅存槽位）
pub fn lambda_model(params: Vec<String>, body: Rc<Expr>) -> TModel {
    TModel {
        internal: Some(Rc::new(LambdaValue { params, body })),
        type_name: "lambda",
        kind: crate::template::ModelKind::Lambda,
        ..TModel::nothing()
    }
}

/// 布尔格式的 true/false 字符串（Java `getTrueStringValue`/`getFalseStringValue`；
/// "true,false" 遗留默认 → None——视为未设置）
pub(crate) fn boolean_format_strings(env: &Environment) -> Option<(String, String)> {
    let format = env.settings.boolean_format.as_str();
    if format == "true,false" {
        // Java parseBooleanFormat（Configurable.java:1087-1090）：BOOLEAN_FORMAT_LEGACY_DEFAULT
        // → null（视为未设置，即使显式设置）→ getTrueStringValue null → 报错
        None
    } else if format == "c" {
        Some(("true".to_string(), "false".to_string()))
    } else {
        format
            .split_once(',')
            .map(|(t, f)| (t.to_string(), f.to_string()))
    }
}

/// 布尔格式化 —— 对应 Java `Environment.formatBoolean`（Environment.java:1795）：
/// - "true,false"（BOOLEAN_FORMAT_LEGACY_DEFAULT）→ parseBooleanFormat 返回 null
///   （Configurable.java:1087-1090，显式设置亦然）→ 视为未设置：
///   fallback=false（插值/字符串拼接路径）报 legacy 错误（jar 实测 `${false}` 默认配置报错）；
///   fallback=true（?string 路径）返回 true/false；
/// - "c" → C 格式 true/false（Java parseBooleanFormat 空数组 → CFormat.getTrue/FalseString）；
/// - 其余 → 按首个逗号切分（Java indexOf(',')；不做 trim）。
pub(crate) fn boolean_format(env: &Environment, b: bool, fallback: bool) -> Result<String> {
    let format = env.settings.boolean_format.as_str();
    if format == "true,false" {
        if fallback {
            return Ok(if b {
                "true".to_string()
            } else {
                "false".to_string()
            });
        }
        return Err(TemplateError::misc(
            "Can't convert boolean to string automatically, because the \"boolean_format\" setting was \"true,false\", which is the legacy deprecated default, and we treat it as if no format was set. This is the default configuration; you should provide the format explicitly for each place where you print a boolean.\n\n----\nTip: Write something like myBool?string('yes', 'no') to specify boolean formatting in place.\n----\nTip: If you want \"true\"/\"false\" result as you are generating computer-language output (not for direct human consumption), then use \"?c\", like ${myBool?c}. (If you always generate computer-language output, then it's might be reasonable to set the \"boolean_format\" setting to \"c\" instead.)\n----\nTip: If you need the same two values on most places, the programmers can set the \"boolean_format\" setting to something like \"yes,no\". However, then it will be easy to unwillingly format booleans like that.\n----",
        ));
    }
    if format == "c" {
        return Ok(if b {
            "true".to_string()
        } else {
            "false".to_string()
        });
    }
    match format.split_once(',') {
        Some((t, f)) => Ok(if b { t.to_string() } else { f.to_string() }),
        None => Err(TemplateError::misc(format!(
            "Setting value must be a string that contains two comma-separated values for true and false, or it must be \"c\", but it was {format:?}."
        ))),
    }
}

/// 模型 → 输出字符串 —— 对应 Java 插值/字符串拼接的模型转字符串规则
/// （Java EvalUtil.coerceModelToStringOrMarkup）：
/// - 标量原样；数字按 number_format（"number" → canonical plain、"c" → C 格式、
///   其余 → DecimalFormat 子集，见 builtins/format.rs）；
/// - 布尔：classic-compatible 模式（Java EvalUtil.coerceModelToTextualCommon :486-518：
///   true → "true"、false → ""；双角色标量模型优先返回字符串），否则 boolean_format；
/// - 日期按 date_format/time_format/date_time_format 设置（Java formatDateToPlainText）。
pub(crate) fn model_to_string(env: &mut Environment, m: &TModel) -> Result<String> {
    if m.is_nothing() {
        // Java EvalUtil.coerceModelToTextualCommon :482-494：tm == null → classic 兼容
        // 模式回退空串；strict 模式为 InvalidReferenceException（本引擎 strict 的
        // 缺失变量在解析层已抛 Err，此处仅显式 null 值会到达）
        if env.settings.classic_compatible {
            return Ok(String::new());
        }
    }
    if let Some(s) = &m.scalar {
        return s.as_string();
    }
    if let Some(n) = &m.number {
        return n
            .as_number()
            .and_then(|n| crate::builtins::format::format_number(env, &n));
    }
    if let Some(b) = &m.boolean {
        let bv = b.as_boolean()?;
        // Java coerceModelToTextualCommon：classic 模式布尔 → "true"/""（先于 formatBoolean）
        if env.settings.classic_compatible {
            return Ok(if bv {
                "true".to_string()
            } else {
                String::new()
            });
        }
        // Java EvalUtil.coerceModelToStringOrMarkup：formatBoolean(value, false)
        return boolean_format(env, bv, false);
    }
    if let Some(d) = &m.date {
        let d = d.as_date()?;
        let format = match d.kind {
            crate::value::DateType::Date => env.settings.date_format.clone(),
            crate::value::DateType::Time => env.settings.time_format.clone(),
            crate::value::DateType::DateTime => env.settings.date_time_format.clone(),
            // Java newCantFormatUnknownTypeDateException（_MessageUtil.java:38-45）：
            // 未知类型须先 ?date/?time/?datetime；消息含 UNKNOWN_DATE_TO_STRING_TIPS
            crate::value::DateType::Unknown => {
                return Err(TemplateError::misc(
                    "Can't convert the date-like value to string because it isn't known if it's a date (no time part), time or date-time value.\n\n----\nTip: Use ?date, ?time, or ?datetime to tell FreeMarker the exact type.\n----\nTip: If you need a particular format only once, use ?string(pattern), like ?string('dd.MM.yyyy HH:mm:ss'), to specify which fields to display. \n----",
                ))
            }
        };
        return format_date_value(env, &d, &format);
    }
    Err(TemplateError::type_mismatch(
        "string-like value",
        m.type_name,
    ))
}

/// 日期格式化分派 —— 对应 Java `Environment.getTemplateDateFormat` 的格式串解析
/// （getTemplateDateFormatWithoutCache :2304-2333）：
/// `xs...` → XML Schema 格式、`iso...` → ISO 8601 格式、其余 → Java 模式
/// （命名模式 short/medium/long/full 或 SimpleDateFormat 子集）
pub(crate) fn format_date_value(
    env: &Environment,
    d: &crate::value::DateValue,
    format_string: &str,
) -> Result<String> {
    use crate::builtins::iso_date_format::{format_iso_like, is_iso_like, parse_iso_params};
    // Java Environment.java:2336-2346（getTemplateDateFormatWithoutCache）：`@name`
    // → 自定义日期格式查找；v1 无注册机制 → 一律 UndefinedCustomFormatException
    // （date/time/datetime 三种格式串共用此检查，消息统一 "No custom date format..."）
    if let Some(name) = crate::builtins::format::custom_format_name(format_string) {
        return Err(TemplateError::misc(format!(
            "No custom date format was defined with name {}",
            crate::builtins::format::j_quote(&name)
        )));
    }
    // Java Environment.java:2184：格式串无效 → "Can't create ... based on format string" 包装
    // （dateformat-iso-like 用例断言消息含 "format string"）
    let r: Result<String> = (|| match is_iso_like(format_string) {
        Some((prefix_len, xs_mode)) => {
            let spec = parse_iso_params(format_string, prefix_len, xs_mode)?;
            format_iso_like(d, &spec, xs_mode, &env.settings.time_zone)
        }
        None => {
            let locale = env.settings.locale.as_str();
            let pattern = crate::builtins::java_date_format::resolve_named_style(
                format_string,
                d.kind,
                locale,
            )
            .unwrap_or_else(|| format_string.to_string());
            crate::builtins::java_date_format::format_java(
                &pattern,
                d,
                locale,
                &env.settings.time_zone,
            )
        }
    })();
    r.map_err(|e| {
        TemplateError::misc(format!(
            "Can't create date/time/datetime format based on format string \"{format_string}\". Reason given: {e}"
        ))
    })
}

/// 日期解析分派 —— 对应 Java `TemplateDateFormat.parse`（ISO/XS/Java 三种格式）
pub(crate) fn parse_date_value(
    env: &Environment,
    s: &str,
    kind: crate::value::DateType,
    format_string: &str,
) -> Result<crate::value::DateValue> {
    use crate::builtins::iso_date_format::{is_iso_like, parse_iso_like, parse_iso_params};
    match is_iso_like(format_string) {
        Some((prefix_len, xs_mode)) => {
            let spec = parse_iso_params(format_string, prefix_len, xs_mode)?;
            parse_iso_like(s, kind, &spec, &env.settings.time_zone, xs_mode)
        }
        None => {
            let locale = env.settings.locale.as_str();
            let pattern =
                crate::builtins::java_date_format::resolve_named_style(format_string, kind, locale)
                    .unwrap_or_else(|| format_string.to_string());
            crate::builtins::java_date_format::parse_java(
                &pattern,
                s,
                kind,
                locale,
                &env.settings.time_zone,
            )
        }
    }
}

/// 表达式求值后转输出字符串（`<#include path>`、`<#stop msg>` 等；缺失 → InvalidReference）
pub(crate) fn eval_to_string(env: &mut Environment, e: &Expr) -> Result<String> {
    let m = crate::core::eval::eval(env, e)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(expr_desc(e)));
    }
    model_to_string(env, &m)
}

/// 表达式描述（Java `getCanonicalForm()` 的 v1 简化子集；错误消息用）
pub(crate) fn expr_desc(e: &Expr) -> String {
    use crate::core::ExprKind as K;
    match &e.kind {
        K::Str(s) => format!("\"{}\"", s),
        K::Num(n) => n.to_plain_string(),
        K::Bool(b) => b.to_string(),
        K::Ident(n) => n.clone(),
        K::Dot { target, name } => format!("{}.{}", expr_desc(target), name),
        K::DynKey { target, key } => format!("{}[{}]", expr_desc(target), expr_desc(key)),
        K::Call { callee, args } => format!(
            "{}({})",
            expr_desc(callee),
            args.iter().map(expr_desc).collect::<Vec<_>>().join(", ")
        ),
        K::Paren(i) => format!("({})", expr_desc(i)),
        K::Not(i) => format!("!{}", expr_desc(i)),
        K::UnaryMinus(i) => format!("-{}", expr_desc(i)),
        K::Add(a, b) => format!("{} + {}", expr_desc(a), expr_desc(b)),
        K::Sub(a, b) => format!("{} - {}", expr_desc(a), expr_desc(b)),
        K::Mul(a, b) => format!("{} * {}", expr_desc(a), expr_desc(b)),
        K::Div(a, b) => format!("{} / {}", expr_desc(a), expr_desc(b)),
        K::Mod(a, b) => format!("{} % {}", expr_desc(a), expr_desc(b)),
        K::Eq(a, b) => format!("{} == {}", expr_desc(a), expr_desc(b)),
        K::NotEq(a, b) => format!("{} != {}", expr_desc(a), expr_desc(b)),
        K::Gt(a, b) => format!("{} > {}", expr_desc(a), expr_desc(b)),
        K::Gte(a, b) => format!("{} >= {}", expr_desc(a), expr_desc(b)),
        K::Lt(a, b) => format!("{} < {}", expr_desc(a), expr_desc(b)),
        K::Lte(a, b) => format!("{} <= {}", expr_desc(a), expr_desc(b)),
        K::And(a, b) => format!("{} && {}", expr_desc(a), expr_desc(b)),
        K::Or(a, b) => format!("{} || {}", expr_desc(a), expr_desc(b)),
        K::Default { target, .. } => format!("{}!", expr_desc(target)),
        K::Exists(t) => format!("{}??", expr_desc(t)),
        K::ListLit(items) => format!(
            "[{}]",
            items.iter().map(expr_desc).collect::<Vec<_>>().join(", ")
        ),
        K::BuiltIn { target, name, args } => match args {
            Some(args) => format!(
                "{}?{}({})",
                expr_desc(target),
                name,
                args.iter().map(expr_desc).collect::<Vec<_>>().join(", ")
            ),
            None => format!("{}?{}", expr_desc(target), name),
        },
        _ => "...".to_string(),
    }
}
