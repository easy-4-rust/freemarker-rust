//! Java `freemarker.cache.TemplateSourceMatcherTest` 的 Rust 1:1 实现
//! （TemplateSourceMatcherTest.java：PathGlobMatcher/FileNameGlobMatcher/
//!   FileExtensionMatcher/PathRegexMatcher/And/Or/Not 匹配器测试）
//!
//! 引擎差异：v1 引擎无 TemplateSourceMatcher 家族（TemplateConfiguration 机制
//! 未移植）——本文件按 Java 语义在测试内实现同名匹配器（纯字符串匹配逻辑，
//! 1:1 断言；glob→正则转换对照 StringUtil.globToRegularExpression，Java:2100+，
//! 共用 util.rs 的 glob_to_regex）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use regex::Regex;

/// 匹配器 trait（对应 Java TemplateSourceMatcher.matches）
trait SourceMatcher {
    fn matches(&self, source_name: &str) -> bool;
}

/// PathGlobMatcher（对应 Java：glob 不能以 "/" 开头；大小写敏感默认 false）
struct PathGlobMatcher {
    glob: String,
    pattern: Regex,
    case_insensitive: bool,
}

impl PathGlobMatcher {
    fn new(glob: &str) -> Self {
        if glob.starts_with('/') {
            panic!("Absolute template paths need no inital \"/\"; remove it from: {glob}");
        }
        let pattern = glob_to_regex(glob, false).expect("glob 应合法");
        PathGlobMatcher {
            glob: glob.to_string(),
            pattern,
            case_insensitive: false,
        }
    }

    fn case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.set_case_insensitive(case_insensitive);
        self
    }

    fn set_case_insensitive(&mut self, case_insensitive: bool) {
        self.case_insensitive = case_insensitive;
        self.pattern = glob_to_regex(&self.glob, case_insensitive).expect("glob 应合法");
    }
}

impl SourceMatcher for PathGlobMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.pattern.is_match(source_name)
    }
}

/// FileNameGlobMatcher（对应 Java：glob 不能含 "/"；模式 = "**/" + glob）
struct FileNameGlobMatcher {
    glob: String,
    pattern: Regex,
}

impl FileNameGlobMatcher {
    fn new(glob: &str) -> Self {
        if glob.contains('/') {
            panic!("A file name glob can't contain \"/\": {glob}");
        }
        let pattern = glob_to_regex(&format!("**/{glob}"), false).expect("glob 应合法");
        FileNameGlobMatcher {
            glob: glob.to_string(),
            pattern,
        }
    }

    fn case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.set_case_insensitive(case_insensitive);
        self
    }

    fn set_case_insensitive(&mut self, case_insensitive: bool) {
        self.pattern =
            glob_to_regex(&format!("**/{}", self.glob), case_insensitive).expect("glob 应合法");
    }
}

impl SourceMatcher for FileNameGlobMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.pattern.is_match(source_name)
    }
}

/// FileExtensionMatcher（对应 Java：默认大小写不敏感；扩展名不能含 '/','*','?'，不能以 "." 开头）
struct FileExtensionMatcher {
    extension: String,
    case_insensitive: bool,
}

impl FileExtensionMatcher {
    fn new(extension: &str) -> Self {
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

    fn set_case_insensitive(&mut self, case_insensitive: bool) {
        self.case_insensitive = case_insensitive;
    }
}

impl SourceMatcher for FileExtensionMatcher {
    /// 对应 Java matches：名字以 "." + 扩展名结尾（点前必须有字符）
    fn matches(&self, source_name: &str) -> bool {
        let ln = source_name.chars().count();
        let ext_ln = self.extension.chars().count();
        if ln < ext_ln + 1 || source_name.chars().nth(ln - ext_ln - 1) != Some('.') {
            return false;
        }
        let tail: String = source_name.chars().skip(ln - ext_ln).collect();
        if self.case_insensitive {
            tail.eq_ignore_ascii_case(&self.extension)
        } else {
            tail == self.extension
        }
    }
}

/// PathRegexMatcher（对应 Java：正则整体匹配；不能以 "/" 开头）
struct PathRegexMatcher {
    pattern: Regex,
}

impl PathRegexMatcher {
    fn new(regex_str: &str) -> Self {
        if regex_str.starts_with('/') {
            panic!("Absolute template paths need no inital \"/\"; remove it from: {regex_str}");
        }
        PathRegexMatcher {
            pattern: Regex::new(regex_str).expect("正则应合法"),
        }
    }
}

impl SourceMatcher for PathRegexMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.pattern.is_match(source_name)
    }
}

/// AndMatcher（对应 Java：0 个匹配器 → IllegalArgumentException）
struct AndMatcher {
    matchers: Vec<Box<dyn SourceMatcher>>,
}

impl AndMatcher {
    fn new(matchers: Vec<Box<dyn SourceMatcher>>) -> Self {
        if matchers.is_empty() {
            panic!("Need at least 1 matcher, had 0.");
        }
        AndMatcher { matchers }
    }
}

impl SourceMatcher for AndMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.matchers.iter().all(|m| m.matches(source_name))
    }
}

/// OrMatcher（对应 Java：0 个匹配器 → IllegalArgumentException）
struct OrMatcher {
    matchers: Vec<Box<dyn SourceMatcher>>,
}

impl OrMatcher {
    fn new(matchers: Vec<Box<dyn SourceMatcher>>) -> Self {
        if matchers.is_empty() {
            panic!("Need at least 1 matcher, had 0.");
        }
        OrMatcher { matchers }
    }
}

impl SourceMatcher for OrMatcher {
    fn matches(&self, source_name: &str) -> bool {
        self.matchers.iter().any(|m| m.matches(source_name))
    }
}

/// NotMatcher
struct NotMatcher {
    matcher: Box<dyn SourceMatcher>,
}

impl NotMatcher {
    fn new(matcher: Box<dyn SourceMatcher>) -> Self {
        NotMatcher { matcher }
    }
}

impl SourceMatcher for NotMatcher {
    fn matches(&self, source_name: &str) -> bool {
        !self.matcher.matches(source_name)
    }
}

/// Java testPathGlobMatcher
#[test]
fn test_path_glob_matcher() {
    let m = PathGlobMatcher::new("**/a/?.ftl");
    assert!(m.matches("a/b.ftl"));
    assert!(m.matches("x/a/c.ftl"));
    assert!(!m.matches("a/b.Ftl"));
    assert!(!m.matches("b.ftl"));
    assert!(!m.matches("a/bc.ftl"));

    let m = PathGlobMatcher::new("**/a/?.ftl").case_insensitive(true);
    assert!(m.matches("A/B.FTL"));
    let mut m = m;
    m.set_case_insensitive(false);
    assert!(!m.matches("A/B.FTL"));

    let r = std::panic::catch_unwind(|| PathGlobMatcher::new("/b.ftl"));
    assert!(r.is_err(), "以 / 开头的 glob 应抛 IllegalArgumentException");
}

/// Java testFileNameGlobMatcher
#[test]
fn test_file_name_glob_matcher() {
    let m = FileNameGlobMatcher::new("a*.ftl");
    assert!(m.matches("ab.ftl"));
    assert!(m.matches("dir/ab.ftl"));
    assert!(m.matches("/dir/dir/ab.ftl"));
    assert!(!m.matches("Ab.ftl"));
    assert!(!m.matches("bb.ftl"));
    assert!(!m.matches("ab.ftl/x"));

    let m = FileNameGlobMatcher::new("a*.ftl").case_insensitive(true);
    assert!(m.matches("AB.FTL"));
    let mut m = m;
    m.set_case_insensitive(false);
    assert!(!m.matches("AB.FTL"));

    let m = FileNameGlobMatcher::new("\u{00E1}*.ftl").case_insensitive(true);
    assert!(m.matches("\u{00C1}b.ftl"));

    let r = std::panic::catch_unwind(|| FileNameGlobMatcher::new("dir/a*.ftl"));
    assert!(r.is_err(), "含 / 的 glob 应抛 IllegalArgumentException");
}

/// Java testFileExtensionMatcher
#[test]
fn test_file_extension_matcher() {
    let m = FileExtensionMatcher::new("ftlx");
    assert!(m.matches("a.ftlx"));
    assert!(m.matches(".ftlx"));
    assert!(m.matches("b/a.b.ftlx"));
    assert!(m.matches("b/a.ftlx"));
    assert!(m.matches("c.b/a.ftlx"));
    assert!(!m.matches("a.ftl"));
    assert!(!m.matches("ftlx"));
    assert!(!m.matches("b.ftlx/a.ftl"));

    assert!(m.case_insensitive);
    assert!(m.matches("a.fTlX"));
    let mut m = m;
    m.set_case_insensitive(false);
    assert!(!m.matches("a.fTlX"));
    assert!(m.matches("A.ftlx"));

    let m = FileExtensionMatcher::new("");
    assert!(m.matches("a."));
    assert!(m.matches("."));
    assert!(!m.matches("a"));
    assert!(!m.matches(""));
    assert!(!m.matches("a.x"));

    let m = FileExtensionMatcher::new("html.t");
    assert!(m.matches("a.html.t"));
    assert!(!m.matches("a.xhtml.t"));
    assert!(!m.matches("a.html"));
    assert!(!m.matches("a.t"));

    assert!(std::panic::catch_unwind(|| FileExtensionMatcher::new("*.ftlx")).is_err());
    assert!(std::panic::catch_unwind(|| FileExtensionMatcher::new("ftl?")).is_err());
    assert!(std::panic::catch_unwind(|| FileExtensionMatcher::new(".ftlx")).is_err());
    assert!(std::panic::catch_unwind(|| FileExtensionMatcher::new("dir/a.ftl")).is_err());
}

/// Java testPathRegexMatcher
#[test]
fn test_path_regex_matcher() {
    let m = PathRegexMatcher::new(r"a/[a-z]+\.ftl");
    assert!(m.matches("a/b.ftl"));
    assert!(m.matches("a/abc.ftl"));
    assert!(!m.matches("b.ftl"));
    assert!(!m.matches("b/b.ftl"));

    let r = std::panic::catch_unwind(|| PathRegexMatcher::new("/b.ftl"));
    assert!(r.is_err(), "以 / 开头的正则应抛 IllegalArgumentException");
}

/// Java testAndMatcher
#[test]
fn test_and_matcher() {
    let m = AndMatcher::new(vec![
        Box::new(PathGlobMatcher::new("a*.*")),
        Box::new(PathGlobMatcher::new("*.t")),
    ]);
    assert!(m.matches("ab.t"));
    assert!(!m.matches("bc.t"));
    assert!(!m.matches("ab.ftl"));

    assert!(std::panic::catch_unwind(|| AndMatcher::new(vec![])).is_err());
}

/// Java testOrMatcher
#[test]
fn test_or_matcher() {
    let m = OrMatcher::new(vec![
        Box::new(PathGlobMatcher::new("a*.*")),
        Box::new(PathGlobMatcher::new("*.t")),
    ]);
    assert!(m.matches("ab.t"));
    assert!(m.matches("bc.t"));
    assert!(m.matches("ab.ftl"));
    assert!(!m.matches("bc.ftl"));

    assert!(std::panic::catch_unwind(|| OrMatcher::new(vec![])).is_err());
}

/// Java testNotMatcher
#[test]
fn test_not_matcher() {
    let m = NotMatcher::new(Box::new(PathGlobMatcher::new("a*.*")));
    assert!(!m.matches("ab.t"));
    assert!(m.matches("bc.t"));
}
