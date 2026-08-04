//! 标记输出内建 —— 对应 Java `freemarker.core.builtins.BuiltInsForMarkupOutputs.java`
//! （esc/no_esc：按 outputFormat 转义 / 标记为 markup 但不转义）

use crate::core::eval_util::target_string;
use crate::core::{Environment, Expr};
use crate::error::Result;
use crate::template::utility::html_escape;
use crate::template::TModel;

pub fn esc(env: &mut Environment, target: &Expr, _args: Option<&[Expr]>) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    let escaped = match env.settings.output_format {
        crate::core::OutputFormatKind::Html | crate::core::OutputFormatKind::XHtml => {
            html_escape(&s)
        }
        crate::core::OutputFormatKind::Xml => crate::template::utility::xml_escape(&s),
        _ => s,
    };
    Ok(Some(markup_model(escaped)))
}

/// ?no_esc —— Java `BuiltInsForOutputFormatRelated.no_escBI`（v1 基础版）：
/// 标记为 markup 但不做转义
pub fn no_esc(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    Ok(Some(markup_model(s)))
}

/// markup 模型（Java TemplateMarkupOutputModel；v1 以字符串承载 + is_markup_output 判定）
fn markup_model(s: String) -> TModel {
    TModel {
        scalar: Some(std::rc::Rc::new(crate::template::SimpleScalar(s))),
        type_name: "markup_output",
        kind: crate::template::ModelKind::Markup,
        ..TModel::nothing()
    }
}
