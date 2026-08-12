//! 换行规范化变换 —— 对应 Java `freemarker.template.utility.NormalizeNewlines`
//! （transform :89-112：BufferedReader.readLine 分行，首行非空才输出，其余行全部
//! println → 行尾统一为 \n）。readLine 语义：`\r\n`、`\r`、`\n` 均视为行尾；
//! EOF 前的最后一段（无行尾）也算一行。

use crate::core::environment::RunSignal;
use crate::core::{Element, Environment};
use crate::error::Result;
use crate::template::{TModel, TemplateTransformModel};
use std::collections::HashMap;

/// 换行规范化变换（对应 NormalizeNewlines.java）
pub struct NormalizeNewlinesTransform;

impl TemplateTransformModel for NormalizeNewlinesTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        _params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&normalize_newlines_text(&captured))?;
        Ok(signal)
    }
}

pub fn normalize_newlines_text(s: &str) -> String {
    let mut out = String::new();
    let mut first = true;
    let mut line_start = 0;
    let b = s.as_bytes();
    let mut i = 0;
    while i <= b.len() {
        let eol_len = if i == b.len() {
            0
        } else if b[i] == b'\n' {
            1
        } else if b[i] == b'\r' {
            if i + 1 < b.len() && b[i + 1] == b'\n' {
                2
            } else {
                1
            }
        } else {
            0
        };
        if eol_len == 0 {
            i += 1;
            continue;
        }
        let line = &s[line_start..i];
        if first {
            first = false;
            if line.is_empty() {
                // 首行空 → 跳过（Java :96-98），后续行照常输出
                i += eol_len;
                line_start = i;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
        i += eol_len;
        line_start = i;
    }
    // EOF 前的最后一段（readLine 返回无行尾的行）
    if line_start < b.len() {
        let line = &s[line_start..];
        if first {
            if !line.is_empty() {
                out.push_str(line);
                out.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}
