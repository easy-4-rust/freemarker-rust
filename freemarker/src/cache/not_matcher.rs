//! 逻辑非匹配器 —— 对应 Java `freemarker.cache.NotMatcher`
//! （子匹配器不命中才命中）

use crate::cache::TemplateSourceMatcher;

/// 逻辑非匹配器（对应 NotMatcher.java）
pub struct NotMatcher {
    matcher: Box<dyn TemplateSourceMatcher>,
}

impl NotMatcher {
    pub fn new(matcher: Box<dyn TemplateSourceMatcher>) -> Self {
        NotMatcher { matcher }
    }
}

impl TemplateSourceMatcher for NotMatcher {
    fn matches(&self, source_name: &str) -> bool {
        !self.matcher.matches(source_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::FileExtensionMatcher;

    #[test]
    fn not_matches() {
        let m = NotMatcher::new(Box::new(FileExtensionMatcher::new("ftl")));
        assert!(m.matches("a.txt"));
        assert!(!m.matches("a.ftl"));
    }
}
