//! 路径 glob 匹配器 —— 对应 Java `freemarker.cache.PathGlobMatcher`
//! （匹配整个模板源名（相对模板存储根的路径）；`**` 为 Ant 风格目录通配；
//! 默认大小写**敏感**；glob 不能以 `/` 开头）

use crate::cache::TemplateSourceMatcher;
use crate::utility::glob_to_regex;
use regex::Regex;

/// 路径 glob 匹配器（对应 PathGlobMatcher.java）
pub struct PathGlobMatcher {
    glob: String,
    pattern: Regex,
    case_insensitive: bool,
}

impl PathGlobMatcher {
    /// 构造（Java :60-66：glob 不能以 `/` 开头——模板路径绝不以 `/` 开头，
    /// 非法 → panic 对应 IllegalArgumentException）
    pub fn new(glob: &str) -> Self {
        if glob.starts_with('/') {
            panic!("Absolute template paths need no inital \"/\"; remove it from: {glob}");
        }
        let mut m = PathGlobMatcher {
            glob: glob.to_string(),
            pattern: Regex::new(".*").unwrap(),
            case_insensitive: false,
        };
        m.build_pattern();
        m
    }

    fn build_pattern(&mut self) {
        self.pattern =
            glob_to_regex(&self.glob, self.case_insensitive).unwrap_or_else(|e| panic!("{e}"));
    }

    pub fn is_case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    /// 设置大小写不敏感（UNICODE 合规）；默认 false（Java :84-90）
    pub fn set_case_insensitive(&mut self, case_insensitive: bool) {
        let last = self.case_insensitive;
        self.case_insensitive = case_insensitive;
        if last != case_insensitive {
            self.build_pattern();
        }
    }

    /// 流式变体（Java `caseInsensitive(boolean)` :95-98）
    pub fn case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.set_case_insensitive(case_insensitive);
        self
    }
}

impl TemplateSourceMatcher for PathGlobMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.pattern.is_match(source_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_glob_matches() {
        let m = PathGlobMatcher::new("a/**");
        assert!(m.matches("a/b/c.ftl"), "尾部 ** 匹配任意深度");
        assert!(m.matches("a/b.ftl"));
        assert!(!m.matches("b/c.ftl"));
        let m = PathGlobMatcher::new("**/head.ftl");
        assert!(m.matches("head.ftl"));
        assert!(m.matches("foo/head.ftl"));
        assert!(m.matches("foo/bar/head.ftl"));
        let m = PathGlobMatcher::new("foo/*.ftl");
        assert!(m.matches("foo/a.ftl"));
        assert!(!m.matches("foo/a/b.ftl"), "* 不跨目录");
        // ** 规则：必须跟在 / 后或开头
        assert!(std::panic::catch_unwind(|| PathGlobMatcher::new("a**/b.ftl")).is_err());
    }
}
