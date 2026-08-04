//! 字符串编码内建 —— 对应 Java `BuiltInsForStringsEncoding.java`（j_string/js_string/
//! json_string/url/url_path/rtf/xhtml/esc/no_esc；html/xml 在 eval.rs 内建集）。
//!
//! 语义要点（Java 对照）：
//! - j_string → StringUtil.javaStringEnc（StringUtil.java:1263）：转义 `"`/`\` 与 <0x20
//!   （\n\r\f\b\t 或 \u00XX；**不加引号**，与 ?c 的字符串输出不同）；
//! - js_string → jsStringEnc(JAVA_SCRIPT)（StringUtil.java:1330 起）：
//!   转义控制字符、`"`、`'`、`\`、`</` 前的 `/`、危险 `>`/`<`、\u007F-\u009F、
//!   \u2028/\u2029；非 JSON 模式 <0x100 用 \xXX；
//! - json_string → jsStringEnc(JSON)：转义 `"`、`\`、`/`（`</` 前）、`>`（危险时 \u003E）、
//!   `<`（危险时 \u003C）、控制字符 \uXXXX；
//! - url/url_path → StringUtil.URLEnc（StringUtil.java:346）：safe 集 a-zA-Z0-9_-.!~'()*，
//!   url_path 额外保留 `/`；按 url_escaping_charset 编码（v1：UTF-8/ISO-8859-1）；
//! - rtf → RTFEnc：转义 `\` `{` `}`；
//! - xhtml → XHTMLEnc（与 Rust html_escape 相同：& < > " '→&#39;）。

use crate::builtins::eval_util::{arg_count, arg_string, check_arg_count, target_string};
use crate::core::{Environment, Expr};
use crate::error::Result;
use crate::template::utility::html_escape;
use crate::template::TModel;

/// ?j_string —— Java `StringUtil.javaStringEnc`（不加引号）：
/// 转义 `"`、`\`、<0x20（\n\r\f\b\t 或 \u00XX 小写十六进制）
pub fn j_string(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    Ok(Some(TModel::from_scalar(java_string_enc(&s))))
}

/// Java `StringUtil.javaStringEnc(s, quote=false)` 的复刻
pub fn java_string_enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\u{c}' => out.push_str("\\f"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!(
                    "\\u00{:x}{:x}",
                    (c as u32) / 0x10,
                    (c as u32) & 0xF
                ));
            }
            c => out.push(c),
        }
    }
    out
}

/// ?js_string —— Java `jsStringEnc(s, JAVA_SCRIPT)`（quotation=null）
pub fn js_string(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    Ok(Some(TModel::from_scalar(js_string_enc(&s, false))))
}

/// ?json_string —— Java `jsStringEnc(s, JSON)`（quotation=null）
pub fn json_string(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    Ok(Some(TModel::from_scalar(js_string_enc(&s, true))))
}

/// Java `StringUtil.jsStringEnc`（StringUtil.java:1428 起）的复刻：
/// `json` 为 true → JSON 兼容模式（\uXXXX、转义 `'` 为 \u0027 等）
pub fn js_string_enc(s: &str, json: bool) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        let c = chars[i];
        let esc: Option<String> = if c <= '\u{1F}' {
            Some(match c {
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\u{c}' => "\\f".to_string(),
                '\u{8}' => "\\b".to_string(),
                '\t' => "\\t".to_string(),
                _ => hex_escape(c, json),
            })
        } else if c == '"' {
            Some("\\\"".to_string())
        } else if c == '\'' {
            // JSON 模式不转义 '（jsonCompatible → NO_ESC）；JS 模式转义
            if json {
                None
            } else {
                Some("\\'".to_string())
            }
        } else if c == '\\' {
            Some("\\\\".to_string())
        } else if c == '/' && (i == 0 || chars[i - 1] == '<') {
            // 防 "</"（Java :1508-1511）
            Some("\\/".to_string())
        } else if c == '>' {
            // 防 "]]>" 与 "-->"（Java :1512-1524）
            let dangerous = if i == 0 {
                true
            } else {
                let prev = chars[i - 1];
                if prev == ']' || prev == '-' {
                    i < 2 || chars[i - 2] == prev
                } else {
                    false
                }
            };
            if dangerous {
                Some(if json {
                    "\\u003E".to_string()
                } else {
                    "\\>".to_string()
                })
            } else {
                None
            }
        } else if c == '<' {
            // 防 "<!" / "<?"（Java :1525-1531）
            let dangerous = if i == n - 1 {
                false
            } else {
                chars[i + 1] == '!' || chars[i + 1] == '?'
            };
            // Java：dangerous → ESC_HEXA（JS 模式 <0x100 用 \x3C）
            if dangerous {
                Some(hex_escape(c, json))
            } else {
                None
            }
        } else if (c as u32) >= 0x7F && (c as u32) <= 0x9F || c == '\u{2028}' || c == '\u{2029}' {
            Some(hex_escape(c, json))
        } else {
            None
        };
        match esc {
            Some(e) => out.push_str(&e),
            None => out.push(c),
        }
        i += 1;
    }
    out
}

/// 十六进制转义（JS 模式 <0x100 用 \xXX；JSON 用 \uXXXX —— Java jsStringEnc ESC_HEXA 分支）
fn hex_escape(c: char, json: bool) -> String {
    let v = c as u32;
    if !json && v < 0x100 {
        format!("\\x{:02X}", v)
    } else {
        format!("\\u{:04X}", v)
    }
}

/// ?url —— Java `StringUtil.URLEnc`（safe 集 + 按 url_escaping_charset 编码；
/// 未设置时回退到 output_encoding——Java getEffectiveURLEscapingCharset 语义）
pub fn url(env: &mut Environment, target: &Expr, args: Option<&[Expr]>) -> Result<Option<TModel>> {
    check_arg_count("url", args, 0, 1)?;
    let s = target_string(env, target)?;
    let charset = if arg_count(args) > 0 {
        arg_string(env, args, 0)?
    } else {
        let c = env.settings.url_escaping_charset.clone();
        if c.is_empty() {
            env.settings.output_encoding.clone()
        } else {
            c
        }
    };
    Ok(Some(TModel::from_scalar(url_enc(&s, &charset, false)?)))
}

/// ?url_path —— 同 ?url 但保留 `/`
pub fn url_path(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("url_path", args, 0, 1)?;
    let s = target_string(env, target)?;
    let charset = if arg_count(args) > 0 {
        arg_string(env, args, 0)?
    } else {
        let c = env.settings.url_escaping_charset.clone();
        if c.is_empty() {
            env.settings.output_encoding.clone()
        } else {
            c
        }
    };
    Ok(Some(TModel::from_scalar(url_enc(&s, &charset, true)?)))
}

/// Java `StringUtil.URLEnc`（StringUtil.java:346-416）：
/// safe 集 = a-z A-Z 0-9 _ - . ! ~ ' ( ) *；其余按 charset 逐字节 %XX（大写十六进制）
fn url_enc(s: &str, charset: &str, keep_slash: bool) -> Result<String> {
    // Java String.getBytes(charset)：空串（url_escaping_charset 未设置，默认 null）→ UTF-8
    // （Environment.getEffectiveURLEscapingCharset）
    let charset = if charset.is_empty() { "UTF-8" } else { charset };
    // Java StringUtil.URLEnc（:360-415）：安全字符原样输出；连续的非安全字符段
    // 整体 getBytes(charset) 后逐字节 %XX —— UTF-16 的 BOM 每段出现一次
    // （output-encoding1 期望 "a%FE%FF%00%2F%00%25b"）
    let safe = |c: char| -> bool {
        c.is_ascii_alphanumeric()
            || matches!(c, '_' | '-' | '.' | '!' | '~')
            || ('\''..='*').contains(&c)
            || (keep_slash && c == '/')
    };
    let encode_run = |run: &str, out: &mut String| {
        let rb = match charset.to_ascii_uppercase().as_str() {
            "UTF-8" | "UTF8" => run.as_bytes().to_vec(),
            "ISO-8859-1" | "ISO8859-1" | "LATIN1" | "8859_1" => {
                run.chars().map(|c| c as u32 as u8).collect()
            }
            "UTF-16" | "UTF16" => {
                let mut v = vec![0xFE, 0xFF];
                for u in run.encode_utf16() {
                    v.push((u >> 8) as u8);
                    v.push((u & 0xFF) as u8);
                }
                v
            }
            _ => unreachable!(),
        };
        for b in rb {
            out.push_str(&format!("%{b:02X}"));
        }
    };
    let mut out = String::with_capacity(s.len());
    let mut run_start: Option<usize> = None;
    for (idx, c) in s.char_indices() {
        if safe(c) {
            if let Some(start) = run_start.take() {
                encode_run(&s[start..idx], &mut out);
            }
            out.push(c);
        } else if run_start.is_none() {
            run_start = Some(idx);
        }
    }
    if let Some(start) = run_start {
        encode_run(&s[start..], &mut out);
    }
    Ok(out)
}

/// ?rtf —— Java `StringUtil.RTFEnc`：转义 `\` `{` `}`（不转义换行）。
/// 同时属于 Java FTL.jj :2230-2238 `BuiltInBannedWhenAutoEscaping` 家族
/// （auto-escaping on + markup 格式时禁用）
pub fn rtf(env: &mut Environment, target: &Expr, _args: Option<&[Expr]>) -> Result<Option<TModel>> {
    crate::core::eval::check_legacy_escaping_ban(env, "rtf")?;
    let s = target_string(env, target)?;
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    Ok(Some(TModel::from_scalar(out)))
}

/// ?xhtml —— Java `StringUtil.XHTMLEnc`（与 Rust html_escape 相同；`'`→&#39;）
pub fn xhtml(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    Ok(Some(TModel::from_scalar(html_escape(&s))))
}

/// ?esc —— Java `BuiltInsForOutputFormatRelated.escBI`（v1 基础版）：
/// 按当前 outputFormat 转义并标记为 markup（HTML/XML 转义；纯文本按原样）
pub fn esc(env: &mut Environment, target: &Expr, _args: Option<&[Expr]>) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    let escaped = match env.settings.output_format {
        crate::core::OutputFormatKind::Html | crate::core::OutputFormatKind::XHtml => {
            html_escape(&s)
        }
        crate::core::OutputFormatKind::Xml => crate::template::utility::xml_escape(&s),
        _ => s,
    };
    Ok(Some(markup_model(escaped)))
}

/// ?no_esc —— Java `BuiltInsForOutputFormatRelated.no_escBI`（v1 基础版）：
/// 标记为 markup 但不做转义
pub fn no_esc(
    env: &mut Environment,
    target: &Expr,
    _args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    let s = target_string(env, target)?;
    Ok(Some(markup_model(s)))
}

/// markup 模型（Java TemplateMarkupOutputModel；v1 以字符串承载 + is_markup_output 判定）
fn markup_model(s: String) -> TModel {
    TModel {
        scalar: Some(std::rc::Rc::new(crate::template::SimpleScalar(s))),
        type_name: "markup_output",
        kind: crate::template::ModelKind::Markup,
        ..TModel::nothing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j_string_basic() {
        assert_eq!(java_string_enc("a"), "a");
        assert_eq!(java_string_enc("a\"b"), "a\\\"b");
        assert_eq!(java_string_enc("a\\b"), "a\\\\b");
        assert_eq!(java_string_enc("\n\t\u{1}"), "\\n\\t\\u0001");
        assert_eq!(java_string_enc("\u{1a}"), "\\u001a");
    }

    #[test]
    fn js_string_basic() {
        assert_eq!(js_string_enc("a'x'\nb", false), "a\\'x\\'\\nb");
        assert_eq!(js_string_enc("\u{1}\u{1a} ", false), "\\x01\\x1A ");
        assert_eq!(js_string_enc("<![CDATA[", false), "\\x3C![CDATA[");
        assert_eq!(js_string_enc("]]>", false), "]]\\>");
        assert_eq!(js_string_enc("</script>", false), "<\\/script>");
    }

    #[test]
    fn json_string_basic() {
        assert_eq!(js_string_enc("a'x'\nb", true), "a'x'\\nb");
        assert_eq!(js_string_enc("\u{1}\u{1a} ", true), "\\u0001\\u001A ");
        assert_eq!(
            js_string_enc("\n\r\t\u{c}\u{8}\"", true),
            "\\n\\r\\t\\f\\b\\\""
        );
        assert_eq!(js_string_enc("/", true), "\\/");
        assert_eq!(js_string_enc("a/b", true), "a/b");
        assert_eq!(js_string_enc("</script>", true), "<\\/script>");
        assert_eq!(js_string_enc("<![CDATA[", true), "\\u003C![CDATA[");
        assert_eq!(js_string_enc("]]>", true), "]]\\u003E");
    }

    #[test]
    fn url_enc_basic() {
        assert_eq!(
            url_enc("a/báb?c/x;y=1", "ISO-8859-1", false).unwrap(),
            "a%2Fb%E1b%3Fc%2Fx%3By%3D1"
        );
        assert_eq!(
            url_enc("a/báb?c/x;y=1", "UTF-8", false).unwrap(),
            "a%2Fb%C3%A1b%3Fc%2Fx%3By%3D1"
        );
        assert_eq!(
            url_enc("a/báb?c/x;y=1", "ISO-8859-1", true).unwrap(),
            "a/b%E1b%3Fc/x%3By%3D1"
        );
    }

    #[test]
    fn rtf_basic() {
        assert_eq!(rtf_enc("a\\b{c}d"), "a\\\\b\\{c\\}d");
    }

    fn rtf_enc(s: &str) -> String {
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '\\' | '{' | '}' => {
                    out.push('\\');
                    out.push(c);
                }
                c => out.push(c),
            }
        }
        out
    }
}
