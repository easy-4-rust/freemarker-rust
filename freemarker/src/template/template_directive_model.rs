//! 对应 Java `freemarker.template.TemplateDirectiveModel`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::core::Environment;
use crate::error::Result;
use crate::template::TModel;
use crate::template::TemplateDirectiveBody;
use std::collections::HashMap;

pub trait TemplateDirectiveModel {
    fn execute(
        &self,
        env: &mut Environment,
        params: &HashMap<String, TModel>,
        loop_vars: &mut [TModel],
        body: Option<&dyn TemplateDirectiveBody>,
    ) -> Result<()>;
}
