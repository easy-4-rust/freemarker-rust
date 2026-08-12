//! 空白压缩变换 —— 对应 Java `freemarker.template.utility.StandardCompress`
//! （逐字符状态机 StandardCompressWriter :117-245；`<#compress>` 指令内部即
//! CompressedBlock → StandardCompress.INSTANCE，CompressedBlock.java:42）

use crate::core::environment::RunSignal;
use crate::core::{Element, Environment};
use crate::error::Result;
use crate::template::{TModel, TemplateTransformModel};
use std::collections::HashMap;

/// 空白压缩变换（对应 StandardCompress.java）
pub struct StandardCompressTransform;

impl TemplateTransformModel for StandardCompressTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        // Java getWriter 参数（StandardCompress.java:92-115）：buffer_size（数值，
        // 缓冲分块——v1 整段捕获等价）、single_line（布尔）
        let single_line = match params.get("single_line") {
            Some(m) => m
                .boolean
                .as_ref()
                .map(|b| b.as_boolean().unwrap_or(false))
                .unwrap_or(false),
            None => false,
        };
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&standard_compress_text(&captured, single_line))?;
        Ok(signal)
    }
}

/// StandardCompress 的逐字符状态机 —— 对应 Java `StandardCompressWriter`
/// （writeHelper :153-171 / updateLineBreakState :173-195 /
/// writeLineBreakOrSpace :197-232）。语义：
/// - 前导空白忽略（AT_BEGINNING）
/// - 换行序列 → 单个换行，**保留原换行类型**（CR / LF / CRLF）
/// - 行内空白序列 → 单个空格（INIT → ' '）
/// - 尾部空白丢弃（从未写入缓冲）
/// - single_line=true → 换行输出为空格（SINGLE_LINE 状态）
///
/// 差异：Rust `char::is_whitespace`（Unicode White_Space）vs Java
/// `Character.isWhitespace`（不含 U+00A0 等）——边界字符行为略宽（P6 可补）。
pub fn standard_compress_text(s: &str, single_line: bool) -> String {
    #[derive(PartialEq, Clone, Copy)]
    enum Lb {
        AtBeginning,
        SingleLine,
        Init,
        SawCr,
        LineBreakCr,
        LineBreakCrLf,
        LineBreakLf,
    }
    let mut out = String::new();
    let mut in_ws = true;
    let mut lb = Lb::AtBeginning;
    for c in s.chars() {
        if c.is_whitespace() {
            in_ws = true;
            // Java updateLineBreakState：仅 INIT / SAW_CR 状态推进
            lb = match lb {
                Lb::Init => {
                    if c == '\r' {
                        Lb::SawCr
                    } else if c == '\n' {
                        Lb::LineBreakLf
                    } else {
                        Lb::Init
                    }
                }
                Lb::SawCr => {
                    if c == '\n' {
                        Lb::LineBreakCrLf
                    } else {
                        Lb::LineBreakCr
                    }
                }
                other => other,
            };
        } else if in_ws {
            in_ws = false;
            // Java writeLineBreakOrSpace
            match lb {
                Lb::AtBeginning => {} // 前导空白忽略
                Lb::SawCr | Lb::LineBreakCr => out.push('\r'),
                Lb::LineBreakCrLf => {
                    out.push('\r');
                    out.push('\n');
                }
                Lb::LineBreakLf => out.push('\n'),
                Lb::Init | Lb::SingleLine => out.push(' '),
            }
            lb = if single_line {
                Lb::SingleLine
            } else {
                Lb::Init
            };
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out
}
