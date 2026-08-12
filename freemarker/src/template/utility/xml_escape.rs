//! XML 转义变换 —— 对应 Java `freemarker.template.utility.XmlEscape`
//! （XMLEnc 变换；转义逻辑见 string_util.rs 的 xml_escape）

use crate::core::environment::RunSignal;
use crate::core::{Element, Environment};
use crate::error::Result;
use crate::template::{TModel, TemplateTransformModel};
use std::collections::HashMap;

/// XML 转义变换（对应 XmlEscape.java）
pub struct XmlEscapeTransform;

impl TemplateTransformModel for XmlEscapeTransform {
    fn transform_with_body(
        &self,
        env: &mut Environment,
        _params: &HashMap<String, TModel>,
        body: &[Element],
    ) -> Result<RunSignal> {
        let (signal, captured) = env.capture(|e| e.run(body))?;
        env.emit(&crate::template::utility::string_util::xml_escape(
            &captured,
        ))?;
        Ok(signal)
    }
}
