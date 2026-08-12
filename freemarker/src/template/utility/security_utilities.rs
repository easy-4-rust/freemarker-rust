//! 安全工具 —— 对应 Java `freemarker.template.utility.SecurityUtilities`
//! （getSystemProperty：经 SecurityManager 读系统属性；
//! v1 用 std::env 直读，文档化差异）

/// 安全工具（对应 SecurityUtilities.java）
pub struct SecurityUtilities;

impl SecurityUtilities {
    /// 系统属性（Java `getSystemProperty(String key)`；v1 = std::env::var）
    pub fn get_system_property(key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    /// 系统属性带默认值（Java :46-）
    pub fn get_system_property_or(key: &str, def_value: &str) -> String {
        std::env::var(key).unwrap_or_else(|_| def_value.to_string())
    }
}
