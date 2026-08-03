//! 字节数组模板加载器 —— 对应 Java `freemarker.cache.ByteArrayTemplateLoader`
//! （从内存中的字节数组加载模板，支持任意编码）
//!
//! 与 `StringTemplateLoader` 不同，此加载器存储原始字节而非字符串，
//! 使 `read_encoded` 可以按 include 指令的 encoding 属性重新解码。
//! 适用于非 UTF-8 编码模板或二进制模板源。

use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::{Result, TemplateError};
use encoding_rs::Encoding;
use std::collections::HashMap;

/// 字节数组模板加载器（对应 Java `ByteArrayTemplateLoader`）
///
/// 存储 `Vec<u8>` 字节数据和可选的时间戳。
/// 通过 `Mutex` 提供线程安全的注册/查找。
pub struct ByteArrayTemplateLoader {
    /// 模板字节数据（key = 模板名）
    templates: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    /// 最后修改时间戳（Unix 毫秒；key = 模板名；Java: `lastModified` 字段）
    last_modified_times: std::sync::Mutex<HashMap<String, i64>>,
}

impl ByteArrayTemplateLoader {
    /// 构造空加载器
    pub fn new() -> Self {
        ByteArrayTemplateLoader {
            templates: std::sync::Mutex::new(HashMap::new()),
            last_modified_times: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// 注册模板字节数据（对应 Java `putTemplate(name, bytes)`）
    ///
    /// 同名模板会被后注册的覆盖。时间戳设为当前系统时间（毫秒）。
    pub fn put(&self, name: &str, bytes: &[u8]) {
        let now = current_time_millis();
        self.put_with_time(name, bytes, now);
    }

    /// 注册模板字节数据并指定最后修改时间（对应 Java 完整构造参数）
    ///
    /// `last_modified` 为 Unix 毫秒时间戳。
    pub fn put_with_time(&self, name: &str, bytes: &[u8], last_modified: i64) {
        let mut t = self.templates.lock().unwrap();
        t.insert(name.to_string(), bytes.to_vec());
        let mut m = self.last_modified_times.lock().unwrap();
        m.insert(name.to_string(), last_modified);
    }

    /// 移除模板（对应 Java `removeTemplate`）
    pub fn remove(&self, name: &str) {
        let mut t = self.templates.lock().unwrap();
        t.remove(name);
        let mut m = self.last_modified_times.lock().unwrap();
        m.remove(name);
    }
}

impl Default for ByteArrayTemplateLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateLoader for ByteArrayTemplateLoader {
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        let t = self.templates.lock().unwrap();
        Ok(t.get(name).map(|_| {
            Box::new(ByteArrayTemplateSource(name.to_string())) as Box<dyn TemplateSource>
        }))
    }

    fn read(&self, src: &dyn TemplateSource) -> Result<String> {
        self.read_encoded(src, "UTF-8")
    }

    /// 对应 Java `getReader(source, encoding)`：按指定字符集解码原始字节。
    /// 未知字符集名按 Java `Charset.forName` 语义报错；编码名大小写不敏感。
    fn read_encoded(&self, src: &dyn TemplateSource, encoding: &str) -> Result<String> {
        let t = self.templates.lock().unwrap();
        let bytes = t
            .get(&src.name())
            .ok_or_else(|| TemplateError::NotFound { name: src.name() })?;
        let enc = Encoding::for_label(encoding.as_bytes()).ok_or_else(|| {
            TemplateError::misc(format!(
                "Unknown encoding: \"{encoding}\". Did you mean to use an IANA character set name?"
            ))
        })?;
        // Java CharsetDecoder 对非法字节默认替换为 U+FFFD；encoding_rs 默认行为一致
        let (text, _, _) = enc.decode(bytes);
        Ok(text.into_owned())
    }

    /// 返回注册时指定的时间戳，未注册则返回 0
    fn last_modified(&self, src: &dyn TemplateSource) -> Result<i64> {
        let m = self.last_modified_times.lock().unwrap();
        Ok(m.get(&src.name()).copied().unwrap_or(0))
    }
}

/// 字节数组模板源（记录模板名用于委托读）
pub struct ByteArrayTemplateSource(String);

impl TemplateSource for ByteArrayTemplateSource {
    fn name(&self) -> String {
        self.0.clone()
    }
}

/// 获取当前时间的 Unix 毫秒时间戳
fn current_time_millis() -> i64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_read_utf8() {
        let loader = ByteArrayTemplateLoader::new();
        loader.put("hello.ftl", "Hello, World!".as_bytes());

        let src = loader.find("hello.ftl").unwrap().expect("应命中");
        assert_eq!(src.name(), "hello.ftl");
        assert_eq!(loader.read(&*src).unwrap(), "Hello, World!");
    }

    #[test]
    fn put_with_time_and_last_modified() {
        let loader = ByteArrayTemplateLoader::new();
        let ts = 1_700_000_000_000i64;
        loader.put_with_time("a.ftl", b"a", ts);

        let src = loader.find("a.ftl").unwrap().expect("应命中");
        assert_eq!(loader.last_modified(&*src).unwrap(), ts);
    }

    #[test]
    fn put_without_time_gets_auto_timestamp() {
        let loader = ByteArrayTemplateLoader::new();
        loader.put("b.ftl", b"b");

        let src = loader.find("b.ftl").unwrap().expect("应命中");
        let ts = loader.last_modified(&*src).unwrap();
        assert!(ts > 0, "自动时间戳应 > 0，实际 {}", ts);
    }

    #[test]
    fn find_missing_returns_none() {
        let loader = ByteArrayTemplateLoader::new();
        assert!(loader.find("nope.ftl").unwrap().is_none());
    }

    #[test]
    fn remove_template() {
        let loader = ByteArrayTemplateLoader::new();
        loader.put("x.ftl", b"x");
        assert!(loader.find("x.ftl").unwrap().is_some());

        loader.remove("x.ftl");
        assert!(loader.find("x.ftl").unwrap().is_none());
    }

    #[test]
    fn read_encoded_non_utf8() {
        let loader = ByteArrayTemplateLoader::new();
        // ISO-8859-1 (Latin-1) encoded bytes: "café" with é = 0xE9
        let bytes: Vec<u8> = vec![0x63, 0x61, 0x66, 0xE9];
        loader.put("latin1.ftl", &bytes);

        let src = loader.find("latin1.ftl").unwrap().expect("应命中");
        let text = loader.read_encoded(&*src, "ISO-8859-1").unwrap();
        assert_eq!(text, "café");
    }

    #[test]
    fn read_encoded_unknown_encoding_errors() {
        let loader = ByteArrayTemplateLoader::new();
        loader.put("x.ftl", b"x");

        let src = loader.find("x.ftl").unwrap().expect("应命中");
        let err = loader.read_encoded(&*src, "BOGUS-CHARSET").unwrap_err();
        assert!(
            err.to_user_message().contains("Unknown encoding"),
            "{}",
            err.to_user_message()
        );
    }

    #[test]
    fn same_name_overwrites() {
        let loader = ByteArrayTemplateLoader::new();
        loader.put("a.ftl", b"first");
        loader.put("a.ftl", b"second");

        let src = loader.find("a.ftl").unwrap().expect("应命中");
        assert_eq!(loader.read(&*src).unwrap(), "second");
    }
}
