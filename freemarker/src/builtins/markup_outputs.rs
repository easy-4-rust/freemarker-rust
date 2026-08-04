//! 标记输出内建 —— 对应 Java `freemarker.core.builtins.BuiltInsForMarkupOutputs.java`
//! 与 `BuiltInsForOutputFormatRelated.java`（esc/no_esc：按 outputFormat 转义 /
//! 标记为 markup 但不转义；markup_string：markup 输出 → 底层字符串）

use crate::core::escape_markup;
use crate::core::eval_util::coerce_to_string;
use crate::core::{Environment, Expr, OutputFormatKind};
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// ?esc —— Java `BuiltInsForOutputFormatRelated.escBI`（:36-44）：目标按当前输出
/// 格式转义后标记为 markup（fromPlainTextByEscaping —— 保留源纯文本槽，跨格式
/// 转换时重转义）；目标已是 markup（同格式或当前格式允许混合）→ 原样绕过，
/// 否则按源纯文本重转义或报错（AbstractConverterBI.calculateResult :52-74 ——
/// "Keep this logic in sync. with ${...}'s logic!"）
pub fn esc(env: &mut Environment, target: &Expr, _args: Option<&[Expr]>) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if m.is_markup_output() {
        return convert_markup_or_bypass(env, &m, "esc");
    }
    let s = coerce_to_string(env, &m)?;
    let fmt = env.settings.output_format;
    Ok(Some(markup_model_with(
        escape_markup(fmt, &s),
        Some(s),
        fmt,
    )))
}

/// ?no_esc —— Java `BuiltInsForOutputFormatRelated.no_escBI`（:26-34）：
/// 标记为 markup 但不做转义（fromMarkup —— 无源纯文本槽）；目标已是 markup →
/// 同 esc 的绕过/重转义逻辑（AbstractConverterBI 共享）
pub fn no_esc(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let m = crate::core::eval::eval(env, target)?;
    if m.is_markup_output() {
        return convert_markup_or_bypass(env, &m, "no_esc");
    }
    let s = coerce_to_string(env, &m)?;
    Ok(Some(markup_model_with(s, None, env.settings.output_format)))
}

/// 目标已是 markup 时的 ?esc/?no_esc 结果 —— 对应 Java
/// AbstractConverterBI.calculateResult（BuiltInsForOutputFormatRelated.java:52-74）：
/// - 目标格式 == 当前格式（或当前格式允许混合）→ 原样绕过；
/// - 否则：目标有源纯文本 → 按当前格式重转义（fromPlainTextByEscaping）；
///   无源纯文本 → 报错（#attempt 可捕获）。
fn convert_markup_or_bypass(
    env: &mut Environment,
    m: &TModel,
    key: &str,
) -> Result<Option<TModel>> {
    let lho_fmt = m.markup_format.unwrap_or(env.settings.output_format);
    let cur_fmt = env.settings.output_format;
    if lho_fmt == cur_fmt || format_mixing_allowed(cur_fmt) {
        return Ok(Some(m.clone()));
    }
    match &m.markup_plain {
        Some(plain) => Ok(Some(markup_model_with(
            escape_markup(cur_fmt, plain),
            Some(plain.clone()),
            cur_fmt,
        ))),
        None => Err(TemplateError::misc(format!(
            "The left side operand of ?{key} is in {} format, which differs from the \
             current output format, {}. Conversion wasn't possible.",
            lho_fmt.name(),
            cur_fmt.name()
        ))),
    }
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

/// markup 模型（Java CommonTemplateMarkupOutputModel：输出格式 + 双内容槽
/// plainTextContent/markupContent，至多一者为 None —— fromPlainTextByEscaping
/// 保留纯文本、fromMarkup 仅存 markup；`?esc`/`?no_esc` 产物与字符串字面量/`+`
/// 拼接的 markup 提升共用）
pub(crate) fn markup_model_with(
    markup: String,
    plain: Option<String>,
    format: OutputFormatKind,
) -> TModel {
    TModel {
        scalar: Some(std::rc::Rc::new(crate::template::SimpleScalar(markup))),
        type_name: "markup_output",
        kind: crate::template::ModelKind::Markup,
        markup_format: Some(format),
        markup_plain: plain,
        ..TModel::nothing()
    }
}

/// 按输出格式转义纯文本 —— 对应 Java `MarkupOutputFormat.output(String, Writer)`
/// （CommonMarkupOutputFormat 的 escapePlainText）。markup 提升（字符串字面量/
/// `+` 拼接）时字符串部分按此转义（Java concatMarkupOutputs 的
/// getMarkupString/escapePlainText 语义，CommonMarkupOutputFormat.java:77-99）。
/// Java 用**目标 markup 的格式**转义字符串部分（AddConcatExpression.java:118-129
/// 的 fromPlainTextByEscaping(leftMO.getOutputFormat())），非当前输出格式。
pub(crate) fn escape_plain_text(format: OutputFormatKind, s: &str) -> String {
    escape_markup(format, s)
}

/// 当前格式是否允许输出外格式 markup 原样混入 —— 对应 Java
/// `OutputFormat.isOutputFormatMixingAllowed`：仅 UndefinedOutputFormat 为 true
/// （UndefinedOutputFormat.java:43-46；HTML/XML/RTF/CSS/JS/JSON/PlainText 均 false）。
/// v1 的 PlainText 承担 undefined 角色（.ftl 默认输出格式）。
pub(crate) fn format_mixing_allowed(kind: OutputFormatKind) -> bool {
    kind == OutputFormatKind::PlainText
}
