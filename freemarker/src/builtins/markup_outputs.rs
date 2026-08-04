//! 标记输出内建 —— 对应 Java `freemarker.core.builtins.BuiltInsForMarkupOutputs.java`
//! 与 `BuiltInsForOutputFormatRelated.java`（esc/no_esc：按 outputFormat 转义 /
//! 标记为 markup 但不转义；markup_string：markup 输出 → 底层字符串）

use crate::core::eval_util::coerce_to_string;
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::utility::html_escape;
use crate::template::TModel;

/// ?esc —— Java `BuiltInsForOutputFormatRelated.escBI`（:36-44）：目标按当前输出
/// 格式转义后标记为 markup；目标已是 markup（同格式）→ 原样绕过
/// （AbstractConverterBI.calculateResult :52-74 —— "Keep this logic in sync.
/// with ${...}'s logic!"，跨格式重转义 v1 简化：markup 无格式槽位）
pub fn esc(env: &mut Environment, target: &Expr, _args: Option<&[Expr]>) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if m.is_markup_output() {
        return Ok(Some(m));
    }
    let s = coerce_to_string(env, &m)?;
    let escaped = match env.settings.output_format {
        crate::core::OutputFormatKind::Html | crate::core::OutputFormatKind::XHtml => {
            html_escape(&s)
        }
        crate::core::OutputFormatKind::Xml => crate::template::utility::xml_escape(&s),
        _ => s,
    };
    Ok(Some(markup_model(escaped)))
}

/// ?no_esc —— Java `BuiltInsForOutputFormatRelated.no_escBI`（:26-34）：
/// 标记为 markup 但不做转义；目标已是 markup → 原样绕过（同上）
pub fn no_esc(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if m.is_markup_output() {
        return Ok(Some(m));
    }
    let s = coerce_to_string(env, &m)?;
    Ok(Some(markup_model(s)))
}

/// ?markup_string —— Java `BuiltInsForMarkupOutputs.markup_stringBI`
/// （BuiltInsForMarkupOutputs.java:31-38）：目标必须是 markup 输出
/// （BuiltInForMarkupOutput._eval :29-36 否则 NonMarkupOutputException）；
/// 结果 = 底层 markup 字符串（CommonMarkupOutputFormat.getMarkupString :77-86）
pub fn markup_string(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if !m.is_markup_output() {
        // Java NonMarkupOutputException（UnexpectedTypeException）：blame 由
        // built_in.rs 的 blame_builtin_operand 附加（"For \"?markup_string\"
        // left-hand operand: Expected a markup output value..."）
        return Err(TemplateError::type_mismatch("markup output", m.type_name));
    }
    Ok(Some(TModel::from_scalar(m.get_scalar()?)))
}

/// markup 模型（Java TemplateMarkupOutputModel；v1 以字符串承载 + is_markup_output
/// 判定；`?esc`/`?no_esc` 产物与字符串字面量/`+` 拼接的 markup 提升共用）
pub(crate) fn markup_model(s: String) -> TModel {
    TModel {
        scalar: Some(std::rc::Rc::new(crate::template::SimpleScalar(s))),
        type_name: "markup_output",
        kind: crate::template::ModelKind::Markup,
        ..TModel::nothing()
    }
}

/// 按当前输出格式转义纯文本 —— 对应 Java `MarkupOutputFormat.output(String, Writer)`
/// （CommonMarkupOutputFormat 的 escapePlainText；HTML/XML 变体，其余格式原样——
/// v1 简化，与 apply_escape 的 autoEsc 分支一致）。markup 提升（字符串字面量/
/// `+` 拼接）时字符串部分按此转义（Java concatMarkupOutputs 的
/// getMarkupString/escapePlainText 语义，CommonMarkupOutputFormat.java:77-99）
pub(crate) fn escape_plain_text(env: &Environment, s: &str) -> String {
    match env.settings.output_format {
        crate::core::OutputFormatKind::Html | crate::core::OutputFormatKind::XHtml => {
            html_escape(s)
        }
        crate::core::OutputFormatKind::Xml => crate::template::utility::xml_escape(s),
        _ => s.to_string(),
    }
}
