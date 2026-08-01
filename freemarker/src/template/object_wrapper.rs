//! 对象包装 —— 对应 Java `freemarker.template.ObjectWrapper` / `ObjectWrapperAndUnwrapper`
//! （Java ObjectWrapper.java:96 `wrap`；ObjectWrapperAndUnwrapper.java:65 `unwrap`）

use crate::error::Result;
use crate::template::{DynValue, TModel};

/// 对应 Java: `freemarker.template.ObjectWrapper`（wrap）+ `ObjectWrapperAndUnwrapper`（unwrap）
/// - `wrap`：语言对象 → 模板模型（Java `wrap`；null → Ok(None)，对应 Java 返回 null）
/// - `unwrap`：模板模型 → 语言对象（Java `ObjectWrapperAndUnwrapper.unwrap`，
///   即 `utility/DeepUnwrap` 的递归展开：哈希/序列逐元素递归）
pub trait ObjectWrapper {
    /// 对应 Java: freemarker.template.ObjectWrapper.wrap
    /// 返回 None 表示 Java 的 null（不可包装为空模型）
    fn wrap(&self, obj: &DynValue) -> Result<Option<TModel>>;

    /// 对应 Java: freemarker.template.ObjectWrapperAndUnwrapper.unwrap
    /// 无法展开的模型类型 → Err（Java TemplateModelException，见 DeepUnwrap.java:175）
    fn unwrap(&self, model: &TModel) -> Result<DynValue>;
}
