//! `?api` 不可用异常 —— 对应 Java `freemarker.core.APINotSupportedTemplateException`
//!（Rust 侧由 `TemplateError` 的 ?api 错误路径承载；`?api` 恒错误的默认语义见
//! `freemarker-test/tests/security_smoke.rs::api_builtin_always_errors`）

/// Java 类锚点：`APINotSupportedTemplateException`
#[allow(dead_code)]
pub(crate) struct ApiNotSupportedTemplateException;

impl ApiNotSupportedTemplateException {
    /// Java 构造：`APINotSupportedTemplateException(env, blamedExpr, model)`
    /// → `buildDescription(...)` 拼装说明后委托 TemplateException。
    /// Rust 对应 `TemplateError` 的 ?api 报错路径。
    /// Java 构造委托（返回统一错误类型而非 Self）
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub(crate) fn new() -> crate::error::TemplateError {
        crate::error::TemplateError::misc(
            "?api is not supported by this object wrapper (see security model decision 1)",
        )
    }
}
