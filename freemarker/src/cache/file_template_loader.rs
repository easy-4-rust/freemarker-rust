//! 文件模板加载器 —— 对应 Java `freemarker.cache.FileTemplateLoader`
//! （构造校验 :117-152 / findTemplateSource :155-187 / getLastModified :190-197 /
//!   getReader :200-216；默认启用 canonical 路径逃逸防护）
//! v1 限制：`..` 逃逸防护用词法规范化路径校验，不解析 OS 符号链接
//! （Java 用 getCanonicalPath 会解析符号链接，见 Java:40-43 与 :168-175；P6 可补）

use crate::cache::{TemplateLoader, TemplateSource};
use crate::error::{Result, TemplateError};
use std::path::{Component, Path, PathBuf};

/// 文件加载器（对应 FileTemplateLoader；baseDir 字段 Java:72）
pub struct FileLoader {
    /// 规范化后的基目录（构造时 canonicalize；对应 Java baseDir :72/:133）
    base_dir: PathBuf,
}

impl FileLoader {
    /// 对应 `FileTemplateLoader(File)`（Java:97-99）：
    /// 基目录必须存在（FileNotFoundException 语义）且为目录（IOException 语义，Java:122-127）
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let raw = base_dir.into();
        if !raw.exists() {
            return Err(TemplateError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("{} does not exist.", raw.display()),
            )));
        }
        if !raw.is_dir() {
            return Err(TemplateError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} is not a directory.", raw.display()),
            )));
        }
        // 对应 baseDir.getCanonicalFile()（Java:133）：规范化后作为后续前缀校验基准
        let base_dir = std::fs::canonicalize(&raw)?;
        Ok(FileLoader { base_dir })
    }

    /// 基目录（对应 `getBaseDirectory`，Java:279-281）
    pub fn base_directory(&self) -> &Path {
        &self.base_dir
    }
}

impl TemplateLoader for FileLoader {
    /// 对应 `findTemplateSource`（Java:155-187）：
    /// 不存在/不是文件 → Ok(None)（Java:162-163）；越出基目录 → 错误（Java:168-175）
    fn find(&self, name: &str) -> Result<Option<Box<dyn TemplateSource>>> {
        // Java:160-161 —— new File(baseDir, name)
        let joined = self.base_dir.join(name);
        let normalized = lexical_normalize(&joined);
        // 逃逸防护（对应 Java canonical 前缀检查 :168-175）：
        // v1 只做词法规范化（"." / ".." 步骤解析），不解析 OS 符号链接
        if !normalized.starts_with(&self.base_dir) {
            return Err(TemplateError::misc(format!(
                "FileTemplateLoader: \"{}\" resolves to \"{}\" which doesn't start with \"{}\"",
                joined.display(),
                normalized.display(),
                self.base_dir.display()
            )));
        }
        if !normalized.is_file() {
            return Ok(None);
        }
        Ok(Some(Box::new(FileSource { path: normalized })))
    }

    /// 对应 `getReader`（Java:200-216）：v1 按 UTF-8 读取整个文件
    fn read(&self, src: &dyn TemplateSource) -> Result<String> {
        let file = downcast_file_src(src)?;
        std::fs::read_to_string(&file.path).map_err(Into::into)
    }

    /// 对应 `getLastModified`（Java:190-197）：文件修改时间，Unix 毫秒
    fn last_modified(&self, src: &dyn TemplateSource) -> Result<i64> {
        let file = downcast_file_src(src)?;
        let meta = std::fs::metadata(&file.path)?;
        let modified = meta.modified()?;
        Ok(modified
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0))
    }
}

/// 文件模板源（Java 侧直接使用 File 对象；name 返回规范化绝对路径）
pub struct FileSource {
    path: PathBuf,
}

impl TemplateSource for FileSource {
    fn name(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    /// 覆写默认 as_any（template_loader.rs:13-16 默认返回 None），否则
    /// downcast_file_src 恒失败（"Not a FileSource"）；MultiSource 同模式
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

impl FileSource {
    /// 从 trait 对象还原（对应 Java 内部按类型分发）
    fn downcast(src: &dyn TemplateSource) -> Option<&FileSource> {
        src.as_any().and_then(|a| a.downcast_ref::<FileSource>())
    }
}

fn downcast_file_src(src: &dyn TemplateSource) -> Result<&FileSource> {
    FileSource::downcast(src).ok_or_else(|| {
        TemplateError::misc(
            "Not a FileSource: template source was created by a different TemplateLoader",
        )
    })
}

/// 词法规范化（解析 "." / ".." 步骤，不触碰文件系统，不解析符号链接）
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            // 越出根时 pop 无效（根之上无目录），路径停留在根 → 前缀检查会拒绝
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 唯一临时目录（std::env::temp_dir() 下）
    fn unique_temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "freemarker-rs-{tag}-{}-{}",
            std::process::id(),
            nanos
        ))
    }

    #[test]
    fn load_file_content_and_missing() {
        let dir = unique_temp_dir("loader");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("hello.ftl"), "Hello, world!").unwrap();
        let loader = FileLoader::new(&dir).unwrap();

        // 创建文件、读取内容、last_modified（毫秒 > 0）
        let src = loader.find("hello.ftl").unwrap().expect("文件存在应命中");
        let name = src.name();
        assert!(name.starts_with(&loader.base_directory().to_string_lossy().to_string()));
        assert!(name.ends_with("hello.ftl"));
        assert_eq!(loader.read(&*src).unwrap(), "Hello, world!");
        assert!(loader.last_modified(&*src).unwrap() > 0);

        // 子目录命中
        std::fs::write(dir.join("sub/inner.ftl"), "inner").unwrap();
        let src = loader
            .find("sub/inner.ftl")
            .unwrap()
            .expect("子目录文件应命中");
        assert_eq!(loader.read(&*src).unwrap(), "inner");

        // 文件不存在 → Ok(None)（Java:162-163 isFile()==false → null）
        assert!(loader.find("nope.ftl").unwrap().is_none());
        // 目录名 → Ok(None)（非文件）
        assert!(loader.find("sub").unwrap().is_none());
    }

    #[test]
    fn escape_rejected() {
        let dir = unique_temp_dir("escape");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("ok.ftl"), "x").unwrap();
        let loader = FileLoader::new(&dir).unwrap();

        // ".." 越出基目录 → Err（对应 Java SecurityException :168-175）
        let e = loader.find("../ok.ftl").err().expect("应拒绝越界路径");
        assert!(e.to_user_message().contains("doesn't start with"));
        // 目录内 ".." 不越界 → 词法规范化后正常命中
        let src = loader
            .find("sub/../ok.ftl")
            .unwrap()
            .expect("目录内 .. 应命中");
        assert_eq!(loader.read(&*src).unwrap(), "x");
        // 绝对路径（绕过 base 前缀）→ 拒绝
        let e = loader.find("/etc/hosts").err().expect("应拒绝绝对路径");
        assert!(e.to_user_message().contains("doesn't start with"));
        // 越出根目录的 ".."（pop 到根后继续 pop 无效）→ 拒绝
        let e = loader.find("../../ok.ftl").err().expect("应拒绝多层越界");
        assert!(e.to_user_message().contains("doesn't start with"));
    }

    #[test]
    fn constructor_requires_existing_directory() {
        // 不存在 → 错误（Java:122-123 FileNotFoundException）
        let missing = unique_temp_dir("missing");
        let e = FileLoader::new(&missing).err().expect("不存在目录应报错");
        assert!(e.to_user_message().contains("does not exist."));

        // 存在但非目录 → 错误（Java:125-126 IOException）
        let dir = unique_temp_dir("file-not-dir");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("plain.txt");
        std::fs::write(&file, "not a dir").unwrap();
        let e = FileLoader::new(&file).err().expect("非目录应报错");
        assert!(e.to_user_message().contains("is not a directory."));
    }
}
