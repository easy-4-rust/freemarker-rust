//! 对应 Java `freemarker.template.TemplateModelWithAPISupport`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::template::TModel;

/// 对应 Java `TemplateModelWithAPISupport`（DefaultMapAdapter/MapModel 等实现）：
/// `?api` 返回该值的 API 视图（Java 侧为反射 API 表面；Rust 引擎自身不支持
/// 反射，由包装方提供视图模型）。
pub trait TemplateApiSupport {
    fn api_view(&self) -> Result<TModel>;
}
