//! 简单标量 —— 对应 Java `freemarker.template.SimpleScalar`

use crate::error::Result;
use crate::template::TemplateScalarModel;

pub struct SimpleScalar(pub String);
impl TemplateScalarModel for SimpleScalar {
    fn as_string(&self) -> Result<String> {
        Ok(self.0.clone())
    }
}
