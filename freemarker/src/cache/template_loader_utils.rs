//! 模板加载器工具 —— 对应 Java `freemarker.cache.TemplateLoaderUtils`
//! （加载器类名内省：Java 反射 `getClass().getName()`，
//! Rust 用 std::any::type_name）

use crate::cache::TemplateLoader;

/// 加载器的类名（对应 `getClassNameForToString(TemplateLoader)`，
/// TemplateLoaderUtils.java:30-：Java `templateLoader.getClass().getName()`；
/// Rust type_name 输出含模块路径，文档化差异）
pub fn get_class_name_for_to_string(template_loader: &dyn TemplateLoader) -> String {
    std::any::type_name_of_val(template_loader).to_string()
}
