//! 对应 Java `freemarker.template.TemplateMethodModelEx（Java 旧接口 TemplateMethodModel 合并于此）`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::core::Environment;
use crate::error::Result;
use crate::template::TModel;

/// 对应 Java `TemplateMethodModelEx`：Java 经线程局部 `Environment.getCurrentEnvironment()`
/// 访问执行环境，Rust 侧改为显式传参（引擎内建方法需加载模板/访问配置时使用）。
pub trait TemplateMethodModelEx {
    fn exec(&self, env: &mut Environment, args: Vec<TModel>) -> Result<TModel>;
}
