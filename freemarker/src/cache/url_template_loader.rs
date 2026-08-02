//! URL 模板加载器 —— 对应 Java `freemarker.cache.URLTemplateLoader`
//! （从 URL 加载模板；Abstract 基类，子类实现 getURL）
//!
//! v1 占位实现：URL 模板加载暂不支持。
//! 完整实现需要 HTTP 客户端依赖（如 `reqwest`），当前返回明确错误提示。
//! 结构体保留以兼容 API 设计，后续版本可对接实际 HTTP/文件 URL 加载。

use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::{Result, TemplateError};

/// URL 模板加载器（对应 Java `URLTemplateLoader`）
///
/// v1 占位：`find` 始终返回错误 "URL-based template loading is not yet supported."。
/// 保留此类型使上层代码可预先引入 `URLTemplateLoader` 而不破坏 API 合约。
pub struct URLTemplateLoader;

impl URLTemplateLoader {
    /// 构造空加载器（v1 无操作）
    pub fn new() -> Self {
        URLTemplateLoader
    }
}

impl Default for URLTemplateLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateLoader for URLTemplateLoader {
    fn find(&self, _name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        Err(TemplateError::misc(
            "URL-based template loading is not yet supported. \
             Use FileLoader or StringLoader instead.",
        ))
    }

    fn read(&self, _src: &dyn TemplateSource) -> Result<String> {
        Err(TemplateError::misc(
            "URL-based template loading is not yet supported.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_returns_error() {
        let loader = URLTemplateLoader::new();
        match loader.find("http://example.com/t.ftl") {
            Ok(_) => panic!("expected error, got Ok"),
            Err(err) => {
                let msg = err.to_user_message();
                assert!(
                    msg.contains("URL-based template loading is not yet supported"),
                    "{msg}"
                );
            }
        }
    }
}
