//! 对应 Java `freemarker.template.TemplateDateModel`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::value::DateValue;

pub trait TemplateDateModel {
    fn as_date(&self) -> Result<DateValue>;
}
