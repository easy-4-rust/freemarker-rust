//! 简单数值 —— 对应 Java `freemarker.template.SimpleNumber`

use crate::error::Result;
use crate::template::TemplateNumberModel;
use crate::value::TNumber;

pub struct SimpleNumber(pub TNumber);
impl TemplateNumberModel for SimpleNumber {
    fn as_number(&self) -> Result<TNumber> {
        Ok(self.0.clone())
    }
}
