//! 字符串字面量 —— 对应 Java `freemarker.core.StringLiteral`
//! （`_eval` :88-106；含 `${}` 插值的 InterpStr 分支见 `eval_interp_str`）

use crate::core::environment::model_to_string;
use crate::core::eval::eval;
use crate::core::{OutputFormatKind, StrPart};
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
/// （StringLiteral.java:96-126 → EvalUtil.concatMarkupOutputs）：
/// 任一插值部分是 markup 输出 → 整串**提升为 markup**（"an interpolated expression
/// that returns markup promotes the result of the whole expression to markup"）——
/// markup 部分原样并入，字符串部分按 **markup 结果的格式**转义后并入
/// （markupResult.getOutputFormat().fromPlainTextByEscaping，Java :100-107）；
/// 源纯文本槽在全部段可逆时保留（跨格式转换依赖，见 ?esc 语义），否则 None。
/// 全部为纯文本 → 普通字符串拼接。
pub(crate) fn eval_interp_str(
    env: &mut crate::core::Environment,
    parts: &[StrPart],
) -> Result<TModel> {
    // Java plainTextResult / markupResult 双轨：markup 未出现时累积纯文本，
    // 首见 markup 后转为 markup 拼接（含纯文本槽合并，CommonMarkupOutputFormat.concat）
    let mut plain: Option<String> = Some(String::new());
    let mut markup: Option<String> = None;
    let mut fmt: Option<OutputFormatKind> = None;
    for part in parts {
        match part {
            StrPart::Text(t) => {
                if let Some(cur) = &mut markup {
                    let f = fmt.unwrap();
                    cur.push_str(&crate::core::built_ins_for_markup_outputs::escape_plain_text(f, t));
                    if let Some(p) = &mut plain {
                        p.push_str(t);
                    }
                } else if let Some(p) = &mut plain {
                    p.push_str(t);
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
                    let mfmt = m.markup_format.unwrap_or(env.settings.output_format);
                    if let Some(cur) = &mut markup {
                        // 后续 markup 段：与既有结果拼接（EvalUtil.concatMarkupOutputs
                        // :573-597 —— 跨格式任一侧有源纯文本则可转成对方格式，
                        // 均无 → 报错）
                        let cur_fmt = fmt.unwrap();
                        if cur_fmt == mfmt {
                            cur.push_str(&m.get_scalar()?);
                        } else if let Some(rp) = &m.markup_plain {
                            cur.push_str(&crate::core::built_ins_for_markup_outputs::escape_plain_text(
                                cur_fmt, rp,
                            ));
                        } else if let Some(lp) = &plain {
                            // 左侧纯文本可转义 → 整体按右侧格式重建
                            // （rightOF.concat(rightOF.fromPlainTextByEscaping(leftPT), rightMO)）
                            *cur = format!(
                                "{}{}",
                                crate::core::built_ins_for_markup_outputs::escape_plain_text(mfmt, lp),
                                m.get_scalar()?
                            );
                            fmt = Some(mfmt);
                        } else {
                            return Err(TemplateError::misc(format!(
                                "Concatenation left hand operand is in {} format, while the \
                                 right hand operand is in {}. Conversion to common format \
                                 wasn't possible.",
                                cur_fmt.name(),
                                mfmt.name()
                            )));
                        }
                        // 纯文本槽：任一段无源纯文本 → 整体不可逆（Java concat 的
                        // pc3 = pc1 != null && pc2 != null ? ... : null）
                        match &m.markup_plain {
                            Some(mp) => {
                                if let Some(p) = &mut plain {
                                    p.push_str(mp);
                                }
                            }
                            None => plain = None,
                        }
                    } else {
                        // 首见 markup（Java :112-116）：此前累积的纯文本按该格式
                        // 转义并入（fromPlainTextByEscaping(plainTextResult)）
                        let p = plain.take().unwrap_or_default();
                        fmt = Some(mfmt);
                        let mut s = String::new();
                        if !p.is_empty() {
                            s.push_str(&crate::core::built_ins_for_markup_outputs::escape_plain_text(
                                mfmt, &p,
                            ));
                        }
                        s.push_str(&m.get_scalar()?);
                        markup = Some(s);
                        plain = m.markup_plain.as_ref().map(|mp| format!("{p}{mp}"));
                    }
                } else if let Some(cur) = &mut markup {
                    // markup 结果后的纯文本段：按 markup 格式转义（Java :101-104）
                    let s = model_to_string(env, &m)?;
                    cur.push_str(&crate::core::built_ins_for_markup_outputs::escape_plain_text(
                        fmt.unwrap(),
                        &s,
                    ));
                    if let Some(p) = &mut plain {
                        p.push_str(&s);
                    }
                } else if let Some(p) = &mut plain {
                    p.push_str(&model_to_string(env, &m)?);
                }
            }
        }
    }
    match markup {
        Some(m) => Ok(crate::core::built_ins_for_markup_outputs::markup_model_with(
            m,
            plain,
            fmt.unwrap(),
        )),
        None => Ok(TModel::from_scalar(plain.unwrap_or_default())),
    }
}
