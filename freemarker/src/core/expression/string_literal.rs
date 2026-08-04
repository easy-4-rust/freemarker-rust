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
/// （StringLiteral.java:96-106 → EvalUtil.coerceModelToTextualCommon）
pub(crate) fn eval_interp_str(
    env: &mut crate::core::Environment,
    parts: &[StrPart],
) -> Result<TModel> {
    let mut out = String::new();
    for part in parts {
        match part {
            StrPart::Text(t) => out.push_str(t),
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
                out.push_str(&model_to_string(env, &m)?);
            }
        }
    }
    Ok(TModel::from_scalar(out))
}
