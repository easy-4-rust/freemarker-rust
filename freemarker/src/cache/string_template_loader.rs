//! 内存模板加载器 —— 对应 Java `freemarker.cache.StringTemplateLoader`
//! （putTemplate/removeTemplate；测试主力）

use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::{Result, TemplateError};
use encoding_rs::Encoding;

#[derive(Default)]
pub struct StringLoader {
    /// 原始 UTF-8 字节（Java StringTemplateLoader 存 char[]；本实现存 UTF-8 字节，
    /// 使 `read_encoded` 能按 include 的 encoding 属性重新解码，还原 Java
    /// getReader(source, encoding) 语义）
    templates: std::sync::Mutex<Vec<(String, Vec<u8>)>>,
}

impl StringLoader {
    pub fn put(&self, name: &str, text: &str) {
        self.put_bytes(name, text.as_bytes());
    }

    /// 按原始字节注册（Java StringTemplateLoader.putTemplate 存 char[]；本实现存
    /// UTF-8 字节，`read_encoded` 可按 include 的 encoding 属性重新解码——
    /// charset-in-header 等非 UTF-8 模板须经此路径注册）
    pub fn put_bytes(&self, name: &str, bytes: &[u8]) {
        let mut t = self.templates.lock().unwrap();
        t.retain(|(n, _)| n != name);
        t.push((name.to_string(), bytes.to_vec()));
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
        self.read_encoded(src, "UTF-8")
    }

    /// 对应 Java `getReader(source, encoding)`：按指定字符集解码原始字节。
    /// 未知字符集名按 Java Charset.forName 语义报错；编码名大小写不敏感。
    fn read_encoded(&self, src: &dyn TemplateSource, encoding: &str) -> Result<String> {
        let t = self.templates.lock().unwrap();
        let bytes = t
            .iter()
            .find(|(n, _)| n == &src.name())
            .map(|(_, b)| b.clone())
            .ok_or_else(|| TemplateError::NotFound { name: src.name() })?;
        let enc = Encoding::for_label(encoding.as_bytes()).ok_or_else(|| {
            TemplateError::misc(format!(
                "Unknown encoding: \"{encoding}\". Did you mean to use an IANA character set name?"
            ))
        })?;
        // Java CharsetDecoder 对非法字节默认替换为 U+FFFD；encoding_rs 默认行为一致
        let (text, _, _) = enc.decode(&bytes);
        Ok(text.into_owned())
    }
}

pub struct StringSource(String);
impl TemplateSource for StringSource {
    fn name(&self) -> String {
        self.0.clone()
    }
}
