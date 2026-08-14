//! 模板后处理器异常 —— 对应 Java `freemarker.core.TemplatePostProcessorException`
//! （checked exception；postProcess 失败时抛出；Java 为包级可见，Rust 为 pub(crate)）

use std::fmt;

/// 模板后处理器异常（对应 Java `TemplatePostProcessorException`）
///
/// Java 签名：
/// - `TemplatePostProcessorException(String message, Throwable cause)`
/// - `TemplatePostProcessorException(String message)`
#[derive(Debug)]
pub struct TemplatePostProcessorException {
    message: String,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl TemplatePostProcessorException {
    /// 创建异常（Java `TemplatePostProcessorException(String message)`）
    pub fn new(message: impl Into<String>) -> Self {
        TemplatePostProcessorException {
            message: message.into(),
            source: None,
        }
    }

    /// 创建带原因的异常（Java `TemplatePostProcessorException(String, Throwable)`）
    pub fn with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        TemplatePostProcessorException {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// 获取异常消息
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for TemplatePostProcessorException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(source) = &self.source {
            write!(f, ": {source}")?;
        }
        Ok(())
    }
}

impl std::error::Error for TemplatePostProcessorException {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}
