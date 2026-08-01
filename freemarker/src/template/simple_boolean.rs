//! 简单布尔 —— 对应 Java `freemarker.template.TrueTemplateBooleanModel` / `FalseTemplateBooleanModel`

use crate::error::Result;
use crate::template::TemplateBooleanModel;

pub struct SimpleBoolean(pub bool);
impl TemplateBooleanModel for SimpleBoolean {
    fn as_boolean(&self) -> Result<bool> {
        Ok(self.0)
    }
}
