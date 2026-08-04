//! 路径正则匹配器 —— 对应 Java `freemarker.cache.PathRegexMatcher`
//! （匹配整个模板源名（相对模板存储根的路径）与给定正则；glob 不能以 `/` 开头）

use crate::cache::TemplateSourceMatcher;
use regex::Regex;

/// 路径正则匹配器（对应 PathRegexMatcher.java）
pub struct PathRegexMatcher {
    pattern: Regex,
}

impl PathRegexMatcher {
    /// 构造（Java :42-47：正则不能以 `/` 开头——模板路径绝不以 `/` 开头，
    /// 非法 → panic 对应 IllegalArgumentException；编译失败 → panic 对应
    /// PatternSyntaxException）
    pub fn new(regex: &str) -> Self {
        if regex.starts_with('/') {
            panic!("Absolute template paths need no inital \"/\"; remove it from: {regex}");
        }
        // Java Pattern.matcher(...).matches() = 全匹配；Rust is_match 为搜索
        // 语义 → 锚定等价
        let anchored = format!("^(?:{regex})$");
        PathRegexMatcher {
            pattern: Regex::new(&anchored)
                .unwrap_or_else(|e| panic!("invalid path regex \"{regex}\": {e}")),
        }
    }
}

impl TemplateSourceMatcher for PathRegexMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.pattern.is_match(source_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_regex_matches() {
        let m = PathRegexMatcher::new(r"^foo/.*\.ftl$");
        assert!(m.matches("foo/a.ftl"));
        assert!(!m.matches("bar/a.ftl"));
        assert!(std::panic::catch_unwind(|| PathRegexMatcher::new("/abs.ftl")).is_err());
        assert!(std::panic::catch_unwind(|| PathRegexMatcher::new("[")).is_err());
    }
}
