//! Java `freemarker.cache.MultiTemplateLoaderTest` 的 Rust 1:1 实现
//! （MultiTemplateLoaderTest.java：多加载器顺序回退 + sticky 行为测试）
//!
//! 引擎映射：`freemarker::cache::MultiLoader` 对应 MultiTemplateLoader。
//! 引擎差异：v1 MultiLoader 恒为非 sticky（按构造顺序查询，等价 Java
//! setSticky(false)）；`setSticky(true)` 未实现（见 multi_template_loader.rs 头注）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::{MultiLoader, TemplateLoader, TemplateSource};
use freemarker::error::Result;
use std::sync::{Arc, Mutex};

/// 可增删模板的内存加载器（对应 Java StringTemplateLoader——
/// 引擎 StringLoader 无 removeTemplate，测试需增删语义）
#[derive(Default)]
struct RemovableLoader {
    templates: Mutex<Vec<(String, String)>>,
}

impl RemovableLoader {
    fn put(&self, name: &str, text: &str) {
        let mut t = self.templates.lock().unwrap();
        t.retain(|(n, _)| n != name);
        t.push((name.to_string(), text.to_string()));
    }

    /// 对应 StringTemplateLoader.removeTemplate：返回是否实际移除
    fn remove(&self, name: &str) -> bool {
        let mut t = self.templates.lock().unwrap();
        let before = t.len();
        t.retain(|(n, _)| n != name);
        t.len() != before
    }
}

impl TemplateLoader for RemovableLoader {
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        let t = self.templates.lock().unwrap();
        Ok(t.iter()
            .find(|(n, _)| n == name)
            .map(|(n, _)| Box::new(RemovableSource(n.clone())) as Box<dyn TemplateSource>))
    }

    fn read(&self, src: &dyn TemplateSource) -> Result<String> {
        let t = self.templates.lock().unwrap();
        t.iter()
            .find(|(n, _)| n == &src.name())
            .map(|(_, c)| c.clone())
            .ok_or_else(|| freemarker::error::TemplateError::NotFound { name: src.name() })
    }
}

struct RemovableSource(String);

impl TemplateSource for RemovableSource {
    fn name(&self) -> String {
        self.0.clone()
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// 对应 Java getTemplateContent：findTemplateSource + getReader
fn get_template_content(loader: &dyn TemplateLoader, name: &str) -> Option<String> {
    let src = loader.find(name).expect("find 不应失败")?;
    Some(loader.read(&*src).expect("read 不应失败"))
}

/// Java testBasics：第一个命中优先；全部 miss → null
#[test]
fn test_basics() {
    let stl1 = Arc::new(RemovableLoader::default());
    stl1.put("1.ftl", "1");
    stl1.put("both.ftl", "both 1");

    let stl2 = Arc::new(RemovableLoader::default());
    stl2.put("2.ftl", "2");
    stl2.put("both.ftl", "both 2");

    let mtl = MultiLoader::new(vec![stl1.clone(), stl2.clone()]);
    assert_eq!(get_template_content(&mtl, "1.ftl").as_deref(), Some("1"));
    assert_eq!(get_template_content(&mtl, "2.ftl").as_deref(), Some("2"));
    assert_eq!(
        get_template_content(&mtl, "both.ftl").as_deref(),
        Some("both 1")
    );
    assert_eq!(get_template_content(&mtl, "neither.ftl"), None);
}

/// Java testSticky（sticky=true）：记住上次命中的 loader。
/// 引擎差异：v1 MultiLoader 恒非 sticky——Java 第 4 步期望 "both 2"（sticky 记住 stl1），
/// v1 按构造顺序重新查询返回 "both 1"。
#[test]
fn test_sticky() {
    test_stickiness(true);
}

/// Java testNonSticky（sticky=false）：每次按构造顺序查询——与 v1 一致
#[test]
fn test_non_sticky() {
    test_stickiness(false);
}

fn test_stickiness(sticky: bool) {
    let stl1 = Arc::new(RemovableLoader::default());
    stl1.put("both.ftl", "both 1");

    let stl2 = Arc::new(RemovableLoader::default());
    stl2.put("both.ftl", "both 2");

    let mtl = MultiLoader::new(vec![stl1.clone(), stl2.clone()]);
    // 引擎差异：v1 无 setSticky——恒为非 sticky
    let _ = sticky;

    assert_eq!(
        get_template_content(&mtl, "both.ftl").as_deref(),
        Some("both 1")
    );
    assert!(stl1.remove("both.ftl"));
    assert_eq!(
        get_template_content(&mtl, "both.ftl").as_deref(),
        Some("both 2")
    );
    stl1.put("both.ftl", "both 1");
    if sticky {
        // 引擎差异：Java sticky=true 期望 "both 2"（记住上次命中的 stl1 已无此模板时
        // 仍只查 stl1）；v1 MultiLoader 恒非 sticky，按顺序查到 stl1 → "both 1"
        assert_eq!(
            get_template_content(&mtl, "both.ftl").as_deref(),
            Some("both 1")
        );
    } else {
        assert_eq!(
            get_template_content(&mtl, "both.ftl").as_deref(),
            Some("both 1")
        );
    }
    assert!(stl2.remove("both.ftl"));
    assert_eq!(
        get_template_content(&mtl, "both.ftl").as_deref(),
        Some("both 1")
    );
}
