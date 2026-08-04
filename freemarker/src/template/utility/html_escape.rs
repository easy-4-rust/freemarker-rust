//! HTML 转义变换 —— 对应 Java `freemarker.template.utility.HtmlEscape`
//! （HTMLEnc 变换；转义逻辑见 string_util.rs 的 html_escape——Java 的
//! StringUtil.htmlEscape 静态方法与此变换共享实现）

use crate::core::environment::RunSignal;
use crate::core::{Element, Environment};
use crate::error::Result;
use crate::template::{TModel, TemplateTransformModel};
use std::collections::HashMap;

/// HTML 转义变换（对应 HtmlEscape.java）
pub struct HtmlEscapeTransform;

impl TemplateTransformModel for HtmlEscapeTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        _params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&html_escape_entity(&captured))?;
        Ok(signal)
    }
}

/// Java HtmlEscape 的实体集：`& < > "`（HtmlEscape.java:63-96 的 getWriter；
/// 与 StringUtil.HTMLEnc 相同，不含 `'`——不同于 XHTMLEnc）
pub fn html_escape_entity(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}
