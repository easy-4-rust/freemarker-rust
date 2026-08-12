//! 逻辑或匹配器 —— 对应 Java `freemarker.cache.OrMatcher`
//! （任一子匹配器命中即命中；至少 1 个）

use crate::cache::TemplateSourceMatcher;

/// 逻辑或匹配器（对应 OrMatcher.java）
pub struct OrMatcher {
    matchers: Vec<Box<dyn TemplateSourceMatcher>>,
}

impl OrMatcher {
    /// 构造（Java :32-35：至少 1 个子匹配器，否则 IllegalArgumentException → panic）
    pub fn new(matchers: Vec<Box<dyn TemplateSourceMatcher>>) -> Self {
        if matchers.is_empty() {
            panic!("Need at least 1 matcher, had 0.");
        }
        OrMatcher { matchers }
    }
}

impl TemplateSourceMatcher for OrMatcher {
    fn matches(&self, source_name: &str) -> bool {
        for m in &self.matchers {
            if m.matches(source_name) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::FileExtensionMatcher;

    #[test]
    fn or_matches() {
        let m = OrMatcher::new(vec![
            Box::new(FileExtensionMatcher::new("ftl")),
            Box::new(FileExtensionMatcher::new("ftlh")),
        ]);
        assert!(m.matches("a.ftl"));
        assert!(m.matches("a.ftlh"));
        assert!(!m.matches("a.txt"));
    }
}
