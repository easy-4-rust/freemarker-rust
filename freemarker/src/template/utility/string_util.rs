//! 字符串工具 —— 对应 Java `freemarker.template.utility.StringUtil`
//! （转义/裁剪/glob 转正则；Java trim 语义差异见 docs/05 §3）

/// HTML 转义（对应 `StringUtil.XHTMLEnc`：`& < > " '`）
pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 旧版 HTML 转义（对应 `StringUtil.HTMLEnc` = `XMLEncNA`，StringUtil.java:69-70：
/// 与 XHTMLEnc 的差异——**不转义 `'`**）。?html 内建在 ICI < 2.3.20 时使用
/// （BuiltInsForStringsEncoding.java:38-43 htmlBI.BIBeforeICI2d3d20）
pub fn html_enc_legacy(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// XML 转义（对应 `StringUtil.XMLEnc`）
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Java 风格 trim（对应 `String.trim`：仅 ≤ U+0020，非 Unicode 空白）
pub fn java_trim(s: &str) -> &str {
    let s = s.trim_start_matches(|c: char| c <= '\u{20}');
    s.trim_end_matches(|c: char| c <= '\u{20}')
}

/// Glob → 正则（对应 `StringUtil.globToRegularExpression`，StringUtil.java:2100-2185；
/// 模板源名以 `/` 分隔）：
/// - `?` → `[^/]`（恰一个字符，不含 `/`）
/// - `*` → `[^/]*`（零或多个字符，不含 `/`）
/// - `**` → 零或多个目录：须在开头或紧跟 `/` 之后，且须在结尾或紧跟 `/` 之后；
///   结尾的 `**`（如 `a/**`）匹配任意深层文件；单独 `**` 匹配一切
/// - `\` 转义下一字符为字面量（`\\` → 字面量反斜杠）
/// - `[` / `{` 保留（Java 报错；须用 `\` 转义）
///   非法 glob 模式 → Err（对应 Java IllegalArgumentException）
pub fn glob_to_regex(glob: &str, case_insensitive: bool) -> Result<regex::Regex, String> {
    let mut regex = String::new();
    let chars: Vec<char> = glob.chars().collect();
    let ln = chars.len();
    let mut next_start = 0;
    let mut escaped = false;
    let mut idx = 0;
    while idx < ln {
        let c = chars[idx];
        if !escaped {
            if c == '?' {
                push_literal_glob_section(&mut regex, &chars, next_start, idx);
                regex.push_str("[^/]");
                next_start = idx + 1;
            } else if c == '*' {
                push_literal_glob_section(&mut regex, &chars, next_start, idx);
                if idx + 1 < ln && chars[idx + 1] == '*' {
                    if !(idx == 0 || chars[idx - 1] == '/') {
                        return Err(format!(
                            "The \"**\" wildcard must be directly after a \"/\" or it must be at the beginning, in this glob: {glob}"
                        ));
                    }
                    if idx + 2 == ln {
                        // 尾部 "**"
                        regex.push_str(".*");
                        idx += 1;
                    } else {
                        // "**/"
                        if !(idx + 2 < ln && chars[idx + 2] == '/') {
                            return Err(format!(
                                "The \"**\" wildcard must be followed by \"/\", or must be at the end, in this glob: {glob}"
                            ));
                        }
                        regex.push_str("(.*?/)*");
                        idx += 2; // 跳过 "*/"
                    }
                } else {
                    regex.push_str("[^/]*");
                }
                next_start = idx + 1;
            } else if c == '\\' {
                escaped = true;
            } else if c == '[' || c == '{' {
                return Err(format!(
                    "The \"{c}\" glob operator is currently unsupported (precede it with \\ for literal matching), in this glob: {glob}"
                ));
            }
        } else {
            escaped = false;
        }
        idx += 1;
    }
    push_literal_glob_section(&mut regex, &chars, next_start, ln);

    let mut pattern = String::new();
    if case_insensitive {
        pattern.push_str("(?iu)");
    }
    // Java 调用方用 Pattern.matcher(...).matches()（全匹配）；Rust regex 的
    // is_match 为搜索语义 → 锚定等价
    pattern.push_str("^(?:");
    pattern.push_str(&regex);
    pattern.push_str(")$");
    regex::Regex::new(&pattern).map_err(|e| format!("invalid glob regex: {e}"))
}

/// 字面量段转义（对应 `appendLiteralGlobSection`：`\` 去转义后整体 quote）
fn push_literal_glob_section(out: &mut String, chars: &[char], start: usize, end: usize) {
    if start == end {
        return;
    }
    let part: String = unescape_literal_glob_section(&chars[start..end]);
    out.push_str(&regex::escape(&part));
}

/// 字面量段的 `\x` → x 去转义（对应 `unescapeLiteralGlobSection`）
fn unescape_literal_glob_section(s: &[char]) -> String {
    let mut out = String::with_capacity(s.len());
    let mut escaped = false;
    for &c in s {
        if !escaped && c == '\\' {
            escaped = true;
        } else {
            out.push(c);
            escaped = false;
        }
    }
    if escaped {
        out.push('\\'); // 尾部孤立反斜杠按字面量保留
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_basic() {
        assert_eq!(
            html_escape("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn xml_escape_apos() {
        assert_eq!(xml_escape("a'b"), "a&apos;b");
    }

    #[test]
    fn java_trim_only_ascii_space() {
        // Java trim 不裁剪 U+00A0（不换行空格）
        assert_eq!(java_trim(" \t x \n"), "x");
        assert_eq!(java_trim("\u{a0}x\u{a0}"), "\u{a0}x\u{a0}");
    }
}
