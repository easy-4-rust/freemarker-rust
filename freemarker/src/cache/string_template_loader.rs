//! 内存模板加载器 —— 对应 Java `freemarker.cache.StringTemplateLoader`
//! （putTemplate/removeTemplate；测试主力）

use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::{Result, TemplateError};

#[derive(Default)]
pub struct StringLoader {
    templates: std::sync::Mutex<Vec<(String, String)>>,
}

impl StringLoader {
    pub fn put(&self, name: &str, text: &str) {
        let mut t = self.templates.lock().unwrap();
        t.retain(|(n, _)| n != name);
        t.push((name.to_string(), text.to_string()));
    }
}

impl TemplateLoader for StringLoader {
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        let t = self.templates.lock().unwrap();
        Ok(t.iter()
            .find(|(n, _)| n == name)
            .map(|(n, _)| Box::new(StringSource(n.clone())) as Box<dyn TemplateSource>))
    }

    fn read(&self, src: &dyn TemplateSource) -> Result<String> {
        let t = self.templates.lock().unwrap();
        t.iter()
            .find(|(n, _)| n == &src.name())
            .map(|(_, text)| text.clone())
            .ok_or_else(|| TemplateError::NotFound { name: src.name() })
    }
}

pub struct StringSource(String);
impl TemplateSource for StringSource {
    fn name(&self) -> String {
        self.0.clone()
    }
}
