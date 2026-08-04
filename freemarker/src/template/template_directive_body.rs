//! 对应 Java `freemarker.template.TemplateDirectiveBody`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::core::Environment;
use crate::error::Result;

/// 自定义指令 body 回插（对应 TemplateDirectiveBody）
pub trait TemplateDirectiveBody {
    fn render(&self, env: &mut Environment) -> Result<()>;
}
