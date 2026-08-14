//! 块赋值指令 —— 对应 Java `freemarker.core.BlockAssignment`
//! （块输出捕获：markup 输出格式下捕获为 markup 模型，否则字符串，赋值）

use crate::core::assignment::{exec_assign_value, AssignScope};
use crate::core::environment::RunSignal;
use crate::core::exec::ExecOutcome;
use crate::core::{Element, Expr};
use crate::error::Result;
use crate::template::TModel;

/// `<#assign name>body</#assign>` 块捕获（对应 BlockAssignment.java；
/// 捕获值在 markup 输出格式下为 markup 模型，见 exec）
pub struct BlockAssignment {
    pub target: String,
    pub body: Vec<Element>,
    pub namespace: Option<Expr>,
}

impl BlockAssignment {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: String, body: Vec<Element>, namespace: Option<Expr>) -> Self {
        BlockAssignment {
            target,
            body,
            namespace,
        }
    }

    /// 执行（Java accept：块输出捕获为字符串后赋值）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let (sig, text) = env.capture(|env| env.run(&self.body))?;
        match sig {
            RunSignal::Returned(v) => Ok(ExecOutcome::ReturnValue(v)),
            RunSignal::Completed => {
                // Java BlockAssignment.capturedStringToModel（BlockAssignment.java:86-88）：
                // 解析期输出格式为 markup 格式 → fromMarkup(captured)（markup 模型，
                // 插值时不再转义）；否则 SimpleScalar（普通字符串）。markup 格式集合
                // 对应 Java `instanceof MarkupOutputFormat`（HTML/XHTML/XML/RTF；
                // CSS/JS/JSON 非 markup，Undefined/PlainText 非 markup）。
                let fmt = env.settings.output_format;
                let value = if matches!(
                    fmt,
                    crate::core::OutputFormatKind::Html
                        | crate::core::OutputFormatKind::XHtml
                        | crate::core::OutputFormatKind::Xml
                        | crate::core::OutputFormatKind::Rtf
                ) {
                    // Java fromMarkup：仅存 markup 内容，无源纯文本槽（跨格式转换
                    // 不可逆 → 插值于其他 markup 格式时报错，DollarVariable.java:78-92）
                    crate::core::built_ins_for_markup_outputs::markup_model_with(text, None, fmt)
                } else {
                    TModel::from_scalar(text)
                };
                exec_assign_value(
                    env,
                    &self.target,
                    value,
                    self.namespace.as_ref(),
                    AssignScope::Namespace,
                )
            }
        }
    }
}
