//! 文件名 glob 匹配器 —— 对应 Java `freemarker.cache.FileNameGlobMatcher`
//! （与 PathGlobMatcher 不同：只比较最后一个 `/` 之后的文件名部分；
//! 等价于 `**/<glob>` 的路径 glob；默认大小写**敏感**）

use crate::cache::TemplateSourceMatcher;
use crate::template::utility::glob_to_regex;
use regex::Regex;

/// 文件名 glob 匹配器（对应 FileNameGlobMatcher.java）
pub struct FileNameGlobMatcher {
    glob: String,
    pattern: Regex,
    case_insensitive: bool,
}

impl FileNameGlobMatcher {
    /// 构造（Java :46-52：glob 不能含 `/`，非法 → panic 对应 IllegalArgumentException）
    pub fn new(glob: &str) -> Self {
        if glob.contains('/') {
            panic!("A file name glob can't contain \"/\": {glob}");
        }
        let mut m = FileNameGlobMatcher {
            glob: glob.to_string(),
            pattern: Regex::new(".*").unwrap(),
            case_insensitive: false,
        };
        m.build_pattern();
        m
    }

    fn build_pattern(&mut self) {
        // Java :55：globToRegularExpression("**/" + glob, caseInsensitive)
        self.pattern = glob_to_regex(&format!("**/{}", self.glob), self.case_insensitive)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    pub fn is_case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    /// 设置大小写不敏感（UNICODE 合规）；默认 false（Java :70-76）
    pub fn set_case_insensitive(&mut self, case_insensitive: bool) {
        let last = self.case_insensitive;
        self.case_insensitive = case_insensitive;
        if last != case_insensitive {
            self.build_pattern();
        }
    }

    /// 流式变体（Java `caseInsensitive(boolean)` :81-84）
    pub fn case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.set_case_insensitive(case_insensitive);
        self
    }
}

impl TemplateSourceMatcher for FileNameGlobMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.pattern.is_match(source_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_glob_matches() {
        let m = FileNameGlobMatcher::new("*.ftlh");
        assert!(m.matches("foo.ftlh"));
        assert!(m.matches("foo/bar.ftlh"), "文件名部分匹配任意目录深度");
        assert!(!m.matches("foo.ftl"));
        assert!(!m.matches("foo.ftlhx"));
        // 大小写敏感（默认）
        let m = FileNameGlobMatcher::new("*.FTLH");
        assert!(!m.matches("foo.ftlh"));
        let m = FileNameGlobMatcher::new("*.ftlh").case_insensitive(true);
        assert!(m.matches("foo.FTLH"));
    }
}
