//! 类工具 —— 对应 Java `freemarker.template.utility.ClassUtil`
//! （JVM 反射工具：forName/短类名/类型描述等）
//! v1 差异：无类加载机制（安全决策）——`for_name` 报错；
//! 短类名用 std::any::type_name 近似（文档化）

use crate::error::{Result, TemplateError};

/// 类工具（对应 ClassUtil.java）
pub struct ClassUtil;

impl ClassUtil {
    /// 类加载（Java `forName`；v1 无类加载 → 明确报错，与 ?new 的
    /// ClassNotFoundException 语义一致）
    pub fn for_name(class_name: &str) -> Result<()> {
        Err(TemplateError::misc(format!(
            "No error description was specified for this error; low-level message: java.lang.ClassNotFoundException: {class_name}"
        )))
    }

    /// 类型名（Java `getShortClassName`；Rust 用 type_name 近似）
    pub fn get_short_class_name<T: ?Sized>() -> String {
        let full = std::any::type_name::<T>();
        full.rsplit("::").next().unwrap_or(full).to_string()
    }
}
