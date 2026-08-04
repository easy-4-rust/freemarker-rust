//! 字符串字面量 —— 对应 Java `freemarker.core.StringLiteral`
//! （`_eval` :88-106；含 `${}` 插值的 InterpStr 分支见 `eval_interp_str`）

use crate::core::environment::model_to_string;
use crate::core::eval::eval;
use crate::core::StrPart;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 字符串字面量（对应 StringLiteral.java；解析器经 `ExprKind::Str` 承载）
pub struct StringLiteral {
    pub value: String,
}

impl StringLiteral {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(value: String) -> Self {
        StringLiteral { value }
    }

    /// 求值（Java `_eval` 纯文本分支）
    pub(crate) fn eval(&self, _env: &mut crate::core::Environment) -> Result<TModel> {
        Ok(TModel::from_scalar(self.value.clone()))
    }
}

/// 插值字符串求值 —— 对应 Java `StringLiteral._eval` 插值分支
/// （StringLiteral.java:96-106 → EvalUtil.coerceModelToTextualCommon）：
/// 任一插值部分是 markup 输出 → 整串**提升为 markup**（Java StringLiteral._eval
/// :88-106 注释 "an interpolated expression that returns markup promotes the result
/// of the whole expression to markup"）——markup 部分原样并入，字符串部分按输出
/// 格式转义后并入（concatMarkupOutputs 的 getMarkupString 语义）；否则纯字符串拼接。
pub(crate) fn eval_interp_str(
    env: &mut crate::core::Environment,
    parts: &[StrPart],
) -> Result<TModel> {
    let mut out = String::new();
    let mut has_markup = false;
    for part in parts {
        match part {
            StrPart::Text(t) => {
                if has_markup {
                    out.push_str(&crate::builtins::markup_outputs::escape_plain_text(env, t));
                } else {
                    out.push_str(t);
                }
            }
            StrPart::Interp(e) => {
                let m = eval(env, e)?;
                if m.is_nothing() {
                    // Java EvalUtil.coerceModelToTextualCommon：tm == null 时 classic 兼容
                    // 模式回退空串（EvalUtil.java:486-489），否则 InvalidReferenceException。
                    if env.settings.classic_compatible {
                        continue;
                    }
                    return Err(TemplateError::invalid_reference(
                        crate::core::environment::expr_desc(e),
                    ));
                }
                if m.is_markup_output() {
                    // Java :101-106：首见 markup → 此前累积的纯文本转义后并入
                    // （fromPlainTextByEscaping(plainTextResult)），随后 markup 原样
                    let content = m.get_scalar()?;
                    if has_markup {
                        out.push_str(&content);
                    } else {
                        has_markup = true;
                        let plain = std::mem::take(&mut out);
                        out.push_str(&crate::builtins::markup_outputs::escape_plain_text(
                            env, &plain,
                        ));
                        out.push_str(&content);
                    }
                } else if has_markup {
                    let s = model_to_string(env, &m)?;
                    out.push_str(&crate::builtins::markup_outputs::escape_plain_text(env, &s));
                } else {
                    out.push_str(&model_to_string(env, &m)?);
                }
            }
        }
    }
    if has_markup {
        Ok(crate::builtins::markup_outputs::markup_model(out))
    } else {
        Ok(TModel::from_scalar(out))
    }
}
