//! 错误上下文 —— 对应 Java `freemarker.core._ErrorDescriptionBuilder`
//! （模板名 + 行列 + 指令栈；消息结构见 docs/09 §2）
//!
//! FTL stack trace 段（`TemplateException.getMessage()` 结构，jar 实测）：
//! ```text
//! <description>
//!
//! ----
//! FTL stack trace ("~" means nesting-related):
//!     - Failed at: ${missing}  [in template "t.ftl" at line 1, column 1]
//!     ~ Reached through: #nested  [in template "t.ftl" in macro "m" at line 1, column 11]
//! ----
//! ```
//! 帧位置格式（Environment.appendInstructionStackItem / _MessageUtil.formatLocation）：
//! `[in template "{name}"{ in macro "{m}"} at line {L}, column {C}]`；
//! `~` 标记：帧本身是 BodyInstruction（`<#nested>`）或紧随其后的帧。

use crate::span::Span;

/// FTL 指令栈帧 —— 对应 Java `TemplateElement[]` 指令栈快照中的单帧
/// （`Environment.outputInstructionStack` :2620-2690 打印）
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// 帧描述（Java `_MessageUtil.shorten(getDescription(), 40)`；如 `${missing}`、`@m 1, 2`）
    pub description: String,
    /// 帧所在模板名（Java `stackEl.getTemplate().getName()` / 宏的模板）
    pub template_name: String,
    /// 帧起始行（Java `stackEl.beginLine`）
    pub line: u32,
    /// 帧起始列（Java `stackEl.beginColumn`）
    pub col: u32,
    /// 词法上包围该帧的宏名（Java `getEnclosingMacro(stackEl)`；
    /// 位置格式 `[in template "t" in macro "m" at ...]`）
    pub in_macro: Option<String>,
    /// 是否为嵌套相关帧（Java `BodyInstruction`；打印时标 `~`，
    /// 且紧随其后的帧也标 `~`）
    pub nesting: bool,
}

impl StackFrame {
    /// 位置段 —— 对应 Java `formatLocationForEvaluationError`
    /// （`[in template "t.ftl"{ in macro "m"} at line L, column C]`）
    pub fn location(&self) -> String {
        let mut s = format!("[in template \"{}\"", self.template_name);
        if let Some(m) = &self.in_macro {
            s.push_str(&format!(" in macro \"{m}\""));
        }
        s.push_str(&format!(" at line {}, column {}]", self.line, self.col));
        s
    }
}

/// 错误上下文（对应 `_ErrorDescriptionBuilder` 输出中的位置、blame 与指令栈）
#[derive(Debug, Clone, Default)]
pub struct ErrorCtx {
    /// 模板名（blame 表达式所在模板；`==> x  [in template ...]`）
    pub template_name: Option<String>,
    /// blame 表达式位置（Java `Expression.getStartLocation`）
    pub span: Span,
    /// FTL 指令栈快照（`TemplateException` 创建时；`getMessage()` 的
    /// `----\nFTL stack trace ...` 段来源）
    pub instruction_stack: Vec<StackFrame>,
    /// TypeMismatch：blamer 前缀（Java `_ErrorDescriptionBuilder.toString` 的
    /// `For "{nodeTypeSymbol}" {role}: ` 段，如 `For "-" right-hand operand: `）
    pub blamer: Option<String>,
    /// TypeMismatch：blame 表达式描述（`==> {expr}` 行；Java `blamed.toString()`）
    pub blamed_expr: Option<String>,
    /// TypeMismatch：赋值目标变量（Java `blamedAssignmentTargetVarName`；
    /// 与 blamed_expr 互斥——`Expected a number, but assignment target variable "x" ...`）
    pub assignment_target: Option<String>,
    /// TypeMismatch：期望类型描述覆盖（Java `expectedTypesDesc` 的 a/an 形式；
    /// 各调用点措辞不同，如 `a string or something automatically convertible to string ...`）
    pub expected_phrase: Option<String>,
    /// 附加 Tip（Java `_ErrorDescriptionBuilder.tip(...)`；如点链缺失的
    /// "It's the step after the last dot..."）
    pub extra_tip: Option<String>,
}

impl ErrorCtx {
    /// 渲染 blame 位置段（`  [in template "t.ftl" at line 1, column 3]`；
    /// 模板名或位置缺失 → 空串）
    pub fn blamed_location(&self) -> String {
        match &self.template_name {
            Some(t) if self.span.line > 0 => {
                format!(
                    "  [in template \"{t}\" at line {}, column {}]",
                    self.span.line, self.span.col
                )
            }
            _ => String::new(),
        }
    }
}

/// 渲染 FTL 指令栈段（含前导空行与 `----` 分隔线；空栈 → None）。
/// 对应 Java `TemplateException.renderMessages`（:180-190）：
/// `messageWithoutStackTop + "\n\n" + "----\nFTL stack trace ...\n{栈}\n----"`
/// 帧顺序：栈顶（最新）在前；`- Failed at:` 为首帧，其余 `- Reached through:`
/// （嵌套帧及其后继帧标 `~`）。
pub fn render_ftl_stack_section(stack: &[StackFrame]) -> Option<String> {
    if stack.is_empty() {
        return None;
    }
    let mut lines = Vec::with_capacity(stack.len());
    let mut prev_nesting = false;
    for (idx, frame) in stack.iter().enumerate() {
        let marker = if idx == 0 {
            "- Failed at: "
        } else {
            // Java：`(frameIdx > 0 && stackEl instanceof BodyInstruction) ||
            // (frameIdx > 1 && snapshot[frameIdx - 1] instanceof BodyInstruction)`
            let nesting = frame.nesting || prev_nesting;
            if nesting {
                "~ Reached through: "
            } else {
                "- Reached through: "
            }
        };
        prev_nesting = frame.nesting;
        lines.push(format!(
            "\t{marker}{}  {}",
            frame.description,
            frame.location()
        ));
    }
    Some(format!(
        "\n\n----\nFTL stack trace (\"~\" means nesting-related):\n{}\n----",
        lines.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_section_format() {
        let stack = vec![
            StackFrame {
                description: "${missing}".to_string(),
                template_name: "t.ftl".to_string(),
                line: 1,
                col: 11,
                in_macro: Some("m".to_string()),
                nesting: false,
            },
            StackFrame {
                description: "@m".to_string(),
                template_name: "t.ftl".to_string(),
                line: 1,
                col: 30,
                in_macro: None,
                nesting: false,
            },
        ];
        let s = render_ftl_stack_section(&stack).unwrap();
        assert_eq!(
            s,
            "\n\n----\nFTL stack trace (\"~\" means nesting-related):\n\
             \t- Failed at: ${missing}  [in template \"t.ftl\" in macro \"m\" at line 1, column 11]\n\
             \t- Reached through: @m  [in template \"t.ftl\" at line 1, column 30]\n----"
        );
    }

    #[test]
    fn nesting_markers() {
        let stack = vec![
            StackFrame {
                description: "${missing}".to_string(),
                template_name: "t.ftl".to_string(),
                line: 1,
                col: 33,
                in_macro: None,
                nesting: false,
            },
            StackFrame {
                description: "#nested".to_string(),
                template_name: "t.ftl".to_string(),
                line: 1,
                col: 11,
                in_macro: Some("m".to_string()),
                nesting: true,
            },
            StackFrame {
                description: "@m".to_string(),
                template_name: "t.ftl".to_string(),
                line: 1,
                col: 29,
                in_macro: None,
                nesting: false,
            },
        ];
        let s = render_ftl_stack_section(&stack).unwrap();
        assert_eq!(
            s,
            "\n\n----\nFTL stack trace (\"~\" means nesting-related):\n\
             \t- Failed at: ${missing}  [in template \"t.ftl\" at line 1, column 33]\n\
             \t~ Reached through: #nested  [in template \"t.ftl\" in macro \"m\" at line 1, column 11]\n\
             \t~ Reached through: @m  [in template \"t.ftl\" at line 1, column 29]\n----"
        );
    }
}
