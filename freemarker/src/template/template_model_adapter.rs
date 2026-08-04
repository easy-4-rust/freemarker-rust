//! 模板模型适配器 —— 对应 Java `freemarker.template.TemplateModelAdapter`
//! （Java :34 行：把非模板对象适配为模板模型——getTemplateModel）

use crate::template::TModel;

/// 模板模型适配器（对应 TemplateModelAdapter.java）
pub trait TemplateModelAdapter {
    /// 返回被适配对象的模板模型视图（Java `getTemplateModel()`）
    fn template_model(&self) -> TModel;
}
