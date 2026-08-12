//! 扩展名匹配器 —— 对应 Java `freemarker.cache.FileExtensionMatcher`
//! （与其他 matcher 不同，默认大小写**不敏感**；名以"点+扩展名"结尾即匹配）

use crate::cache::TemplateSourceMatcher;

/// 扩展名匹配器（对应 FileExtensionMatcher.java；扩展名不含开头的点）
pub struct FileExtensionMatcher {
    extension: String,
    case_insensitive: bool,
}

impl FileExtensionMatcher {
    /// 构造（Java 构造器 :39-53 的校验：不能含 `/` `*` `?`、不能以 `.` 开头，
    /// 非法 → IllegalArgumentException —— Rust 用 panic! 对应，文档注明）
    pub fn new(extension: &str) -> Self {
        if extension.contains('/') {
            panic!("A file extension can't contain \"/\": {extension}");
        }
        if extension.contains('*') {
            panic!("A file extension can't contain \"*\": {extension}");
        }
        if extension.contains('?') {
            panic!("A file extension can't contain \"*\": {extension}");
        }
        if extension.starts_with('.') {
            panic!("A file extension can't start with \".\": {extension}");
        }
        FileExtensionMatcher {
            extension: extension.to_string(),
            case_insensitive: true,
        }
    }

    pub fn is_case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    /// 设置大小写不敏感（UNICODE 合规）；默认 true（Java :72-75）
    pub fn set_case_insensitive(&mut self, case_insensitive: bool) {
        self.case_insensitive = case_insensitive;
    }

    /// 流式变体（Java `caseInsensitive(boolean)` 流体 API :80-83）
    pub fn case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.set_case_insensitive(case_insensitive);
        self
    }
}

impl TemplateSourceMatcher for FileExtensionMatcher {
    /// Java :56-64：名长 ≥ 扩展长+1 且倒数第 extLn+1 位是 `.`，扩展名逐字符比较
    /// （regionMatches 的 caseInsensitive 语义 → Unicode 大小写折叠）
    fn matches(&self, source_name: &str) -> bool {
        let ln = source_name.len();
        let ext_ln = self.extension.len();
        if ln < ext_ln + 1 || source_name.as_bytes()[ln - ext_ln - 1] != b'.' {
            return false;
        }
        let ext_part = &source_name[ln - ext_ln..];
        if self.case_insensitive {
            ext_part.to_lowercase() == self.extension.to_lowercase()
        } else {
            ext_part == self.extension
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_matching() {
        let m = FileExtensionMatcher::new("ftlh");
        assert!(m.matches("foo.ftlh"));
        assert!(m.matches("foo/bar.ftlh"));
        assert!(m.matches("foo.FTLH"), "默认大小写不敏感");
        assert!(!m.matches("foo.ftl"));
        assert!(!m.matches("fooftlh"));
        assert!(!m.matches("foo.ftlhx"));
        // 大小写敏感
        let m = FileExtensionMatcher::new("ftlh").case_insensitive(false);
        assert!(m.matches("foo.ftlh"));
        assert!(!m.matches("foo.FTLH"));
    }

    #[test]
    #[should_panic]
    fn slash_rejected() {
        let _ = FileExtensionMatcher::new("a/b");
    }
}
