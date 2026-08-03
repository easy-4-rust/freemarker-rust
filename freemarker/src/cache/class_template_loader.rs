//! 类路径模板加载器 —— 对应 Java `freemarker.cache.ClassTemplateLoader`
//! （从编译时嵌入的 &'static str 资源加载模板；适用于 include_str! 场景）
//!
//! Rust 等价：从 `HashMap<String, &'static str>` 注册表中加载。
//! 可选 `base_path` 前缀 —— 与 Java ClassTemplateLoader 的 basePath 字段语义一致：
//! 查找前先剥离前缀，再将剩余路径作为 key 检索。

use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::{Result, TemplateError};
use std::collections::HashMap;

/// 类路径模板加载器（对应 Java `ClassTemplateLoader`）
///
/// 存储 `&'static str` 引用（编译时嵌入，如 `include_str!`），不产生堆复制开销。
/// 通过 `Mutex` 提供线程安全的注册/查找。
pub struct ClassTemplateLoader {
    /// 模板注册表（key = 模板名，value = 编译时嵌入的源文本）
    templates: std::sync::Mutex<HashMap<String, &'static str>>,
    /// 前缀（查找时先剥离 —— 对应 Java `basePath` 字段）
    base_path: std::sync::Mutex<Option<String>>,
}

impl ClassTemplateLoader {
    /// 构造空加载器
    pub fn new() -> Self {
        ClassTemplateLoader {
            templates: std::sync::Mutex::new(HashMap::new()),
            base_path: std::sync::Mutex::new(None),
        }
    }

    /// 注册一个编译时嵌入的模板资源（类似 Java 的 classpath 注册语义）。
    ///
    /// `name` 为模板名（不含 base_path），`source` 为 `&'static str` 源文本。
    /// 同名模板会被后注册的覆盖（与 Java `StringTemplateLoader.putTemplate` 一致）。
    pub fn put(&self, name: &str, source: &'static str) {
        let mut t = self.templates.lock().unwrap();
        t.insert(name.to_string(), source);
    }

    /// 设置 base_path 前缀（对应 Java `setBasePath`）
    ///
    /// 设置后，`find` 会在查找前剥离该前缀。例如 base_path="/templates/"
    /// 时，"templates/foo.ftl" → 查找 "foo.ftl"。
    pub fn set_base_path(&self, path: &str) {
        let mut bp = self.base_path.lock().unwrap();
        *bp = Some(normalize_base_path(path));
    }

    /// 查找前归一化并剥离 base_path 前缀
    fn resolve_name(&self, raw_name: &str) -> String {
        let name = normalize_slash(raw_name);
        let bp = self.base_path.lock().unwrap();
        match &*bp {
            Some(prefix) if name.starts_with(prefix) => name[prefix.len()..].to_string(),
            _ => name,
        }
    }
}

impl Default for ClassTemplateLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateLoader for ClassTemplateLoader {
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        let resolved = self.resolve_name(name);
        let t = self.templates.lock().unwrap();
        Ok(t.get(&resolved)
            .map(|_| Box::new(ClassTemplateSource(resolved)) as Box<dyn TemplateSource>))
    }

    fn read(&self, src: &dyn TemplateSource) -> Result<String> {
        let t = self.templates.lock().unwrap();
        let source = t
            .get(&src.name())
            .ok_or_else(|| TemplateError::NotFound { name: src.name() })?;
        Ok((*source).to_string())
    }
}

/// 类路径模板源（记录模板名用于委托读）
pub struct ClassTemplateSource(String);

impl TemplateSource for ClassTemplateSource {
    fn name(&self) -> String {
        self.0.clone()
    }
}

/// 统一使用正斜杠（与 Java `ClassTemplateLoader` 内部 `canonicalizePrefix` 一致）
fn normalize_slash(path: &str) -> String {
    path.replace('\\', "/")
}

/// 归一化 base_path：去除首尾斜杠并统一分隔符
fn normalize_base_path(path: &str) -> String {
    let normalized = normalize_slash(path);
    let trimmed = normalized.trim_matches('/');
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{}/", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_and_read_without_base_path() {
        let loader = ClassTemplateLoader::new();
        loader.put("hello.ftl", "Hello, World!");

        let src = loader.find("hello.ftl").unwrap().expect("应命中");
        assert_eq!(src.name(), "hello.ftl");
        assert_eq!(loader.read(&*src).unwrap(), "Hello, World!");
    }

    #[test]
    fn find_with_base_path() {
        let loader = ClassTemplateLoader::new();
        loader.put("hello.ftl", "Hello, World!");
        loader.set_base_path("/templates/");

        // 带 base_path 前缀查找
        let src = loader.find("templates/hello.ftl").unwrap().expect("应命中");
        assert_eq!(src.name(), "hello.ftl");
        assert_eq!(loader.read(&*src).unwrap(), "Hello, World!");

        // 不带前缀查找也能命中（base_path 前缀不存在时不剥离）
        let src2 = loader.find("hello.ftl").unwrap().expect("应命中");
        assert_eq!(src2.name(), "hello.ftl");
    }

    #[test]
    fn find_missing_returns_none() {
        let loader = ClassTemplateLoader::new();
        assert!(loader.find("nope.ftl").unwrap().is_none());
    }

    #[test]
    fn base_path_with_backslash_normalized() {
        let loader = ClassTemplateLoader::new();
        loader.put("sub/foo.ftl", "foo content");
        loader.set_base_path("\\templates\\");

        // 反斜杠被归一化为正斜杠
        let src = loader
            .find("templates/sub/foo.ftl")
            .unwrap()
            .expect("应命中");
        assert_eq!(src.name(), "sub/foo.ftl");
        assert_eq!(loader.read(&*src).unwrap(), "foo content");
    }

    #[test]
    fn base_path_without_slashes() {
        let loader = ClassTemplateLoader::new();
        loader.put("foo.ftl", "foo");
        loader.set_base_path("prefix");

        // 查找 "prefix/foo.ftl" → 剥离 "prefix/" → "foo.ftl"
        let src = loader.find("prefix/foo.ftl").unwrap().expect("应命中");
        assert_eq!(src.name(), "foo.ftl");
    }

    #[test]
    fn same_name_overwrites() {
        let loader = ClassTemplateLoader::new();
        loader.put("a.ftl", "first");
        loader.put("a.ftl", "second");

        let src = loader.find("a.ftl").unwrap().expect("应命中");
        assert_eq!(loader.read(&*src).unwrap(), "second");
    }
}
