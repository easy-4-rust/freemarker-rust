//! 逻辑与匹配器 —— 对应 Java `freemarker.cache.AndMatcher`
//! （全部子匹配器命中才命中；至少 1 个）

use crate::cache::TemplateSourceMatcher;

/// 逻辑与匹配器（对应 AndMatcher.java）
pub struct AndMatcher {
    matchers: Vec<Box<dyn TemplateSourceMatcher>>,
}

impl AndMatcher {
    /// 构造（Java :32-35：至少 1 个子匹配器，否则 IllegalArgumentException → panic）
    pub fn new(matchers: Vec<Box<dyn TemplateSourceMatcher>>) -> Self {
        if matchers.is_empty() {
            panic!("Need at least 1 matcher, had 0.");
        }
        AndMatcher { matchers }
    }
}

impl TemplateSourceMatcher for AndMatcher {
    fn matches(&self, source_name: &str) -> bool {
        for m in &self.matchers {
            if !m.matches(source_name) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::FileExtensionMatcher;
    use crate::cache::PathGlobMatcher;

    #[test]
    fn and_matches() {
        let m = AndMatcher::new(vec![
            Box::new(PathGlobMatcher::new("foo/**")),
            Box::new(FileExtensionMatcher::new("ftl")),
        ]);
        assert!(m.matches("foo/a.ftl"));
        assert!(!m.matches("bar/a.ftl"));
        assert!(!m.matches("foo/a.txt"));
        assert!(std::panic::catch_unwind(|| AndMatcher::new(vec![])).is_err());
    }
}
