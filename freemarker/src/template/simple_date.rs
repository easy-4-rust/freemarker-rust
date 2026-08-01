//! 简单日期 —— 对应 Java `freemarker.template.SimpleDate`
//! （dateType：DATE/TIME/DATETIME，见 docs/06 §5）

use crate::error::Result;
use crate::template::TemplateDateModel;
use crate::value::DateValue;

pub struct SimpleDate(pub DateValue);
impl TemplateDateModel for SimpleDate {
    fn as_date(&self) -> Result<DateValue> {
        Ok(self.0.clone())
    }
}
