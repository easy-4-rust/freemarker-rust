//! 字符串工具 —— 对应 Java `freemarker.template.utility.StringUtil`
//! （转义/裁剪；Java trim 语义差异见 docs/05 §3）

//! 工具函数 —— 对应 `freemarker.template.utility.*`
//! （StringUtil 转义/裁剪等；由各智能体按需补充）

/// HTML 转义（对应 `StringUtil.HTMLEnc`：`& < > " '`）
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
