//! 测试 —— 自 built_ins_for_strings_basic.rs 拆出（#[cfg(test)] 模块；由主文件
//! #[cfg(test)] #[path] 声明，仅测试构建时编译）。

#[cfg(test)]
mod tests {

    #[test]
    fn capitalize_and_chomp() {
        assert_eq!(
            capitalize_simple("dieBugsDie! * vazzZE"),
            "Diebugsdie! * Vazzze"
        );
        assert_eq!(chop_linebreak_simple("qwe\r\n\r\n"), "qwe\r\n");
        assert_eq!(chop_linebreak_simple("qwe\n"), "qwe");
        assert_eq!(chop_linebreak_simple("qwe"), "qwe");
    }

    fn capitalize_simple(s: &str) -> String {
        let mut out = String::new();
        let mut token = String::new();
        for c in s.chars() {
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                if !token.is_empty() {
                    let mut chars = token.chars();
                    out.push_str(&chars.next().unwrap().to_uppercase().collect::<String>());
                    out.push_str(&chars.as_str().to_lowercase());
                    token.clear();
                }
                out.push(c);
            } else {
                token.push(c);
            }
        }
        if !token.is_empty() {
            let mut chars = token.chars();
            out.push_str(&chars.next().unwrap().to_uppercase().collect::<String>());
            out.push_str(&chars.as_str().to_lowercase());
        }
        out
    }

    fn chop_linebreak_simple(s: &str) -> String {
        if let Some(rest) = s.strip_suffix("\r\n") {
            rest.to_string()
        } else if let Some(rest) = s.strip_suffix('\r') {
            rest.to_string()
        } else if let Some(rest) = s.strip_suffix('\n') {
            rest.to_string()
        } else {
            s.to_string()
        }
    }

    #[test]
    fn pad_utf16_semantics() {
        // 非 BMP 字符占 2 个 UTF-16 码元（Java String.length）
        let s = "\u{10000}".to_string();
        assert_eq!(s.encode_utf16().count(), 2);
        // left_pad("\u{10000}", 4, " ") → 2 个空格 + 字符
        let pad = pad_impl_pure(&s, 4, " ", true);
        assert_eq!(pad.encode_utf16().count(), 4);
        assert_eq!(pad, "  \u{10000}");
    }

    fn pad_impl_pure(s: &str, width: usize, filling: &str, left: bool) -> String {
        let s_units: Vec<u16> = s.encode_utf16().collect();
        if width <= s_units.len() {
            return s.to_string();
        }
        let filling: Vec<u16> = filling.encode_utf16().collect();
        let dif = width - s_units.len();
        let mut pad: Vec<u16> = Vec::with_capacity(dif);
        let mut i = 0;
        while pad.len() < dif {
            pad.push(filling[i % filling.len()]);
            i += 1;
        }
        let pad = String::from_utf16_lossy(&pad);
        if left {
            format!("{pad}{s}")
        } else {
            format!("{s}{pad}")
        }
    }

    /// 渲染辅助（keep/ensure/substring 家族矩阵，断言源自 Java templatesuite
    /// string-builtins3.ftl + jar 探针）
    fn render_out(src: &str) -> crate::error::Result<String> {
        use crate::cache::StringLoader;
        use crate::template::{Configuration, ObjectWrapper, SimpleObjectWrapper, TModel};
        let mut c = Configuration::new();
        let loader = std::sync::Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put("t.ftl", src);
        let t = c.get_template("t.ftl")?;
        let root = SimpleObjectWrapper
            .wrap(&crate::template::DynValue::Map(vec![]))?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn keep_regexp_overlapping_scan() {
        // Java matcher.find(start+1) 重叠扫描：非重叠 find_iter 漏匹配
        // （string-builtins3.ftl:37-40）
        assert_eq!(
            render_out("${'aaabb'?keep_before_last('[ab]{3}', 'r')}").unwrap(),
            "aa",
            "正则 [ab]{{3}} 在 aaabb 上的最后匹配 start=2（aaa@0/aab@1/abb@2）"
        );
        assert_eq!(
            render_out("${'aaabbxbabe'?keep_before_last('[ab]{3}', 'r')}").unwrap(),
            "aaabbx",
            "babe 上的 abb@… 后移"
        );
        assert_eq!(
            render_out("${'aaabb12345'?keep_after_last('[ab]{3}', 'r')}").unwrap(),
            "12345"
        );
        // 无重叠歧义时与 find_iter 一致
        assert_eq!(
            render_out("${'xxxaaayyy'?keep_before_last('a+', 'r')}").unwrap(),
            "xxxaa"
        );
        assert_eq!(
            render_out("${'xxxaaayyy'?keep_after_last('a+', 'r')}").unwrap(),
            "yyy"
        );
    }

    #[test]
    fn ensure_starts_with_default_regexp() {
        // Java BuiltInsForStringsBasic.java:163-166：2 参数无显式 flags → 默认 REGEXP
        assert_eq!(
            render_out("${'bacdef'?ensure_starts_with('[ab]{2}', 'ab')}").unwrap(),
            "bacdef",
            "默认正则前缀 [ab]{{2}} 匹配 ba → 不加前缀"
        );
        assert_eq!(
            render_out("${'cacdef'?ensure_starts_with('[ab]{2}', 'ab')}").unwrap(),
            "abcacdef",
            "ca 不匹配 [ab]{{2}} → 加前缀 ab"
        );
        assert_eq!(
            render_out("${'HTTP://example.com'?ensure_starts_with('[a-z]+://', 'http://', 'ir')}")
                .unwrap(),
            "HTTP://example.com",
            "显式 ir flags"
        );
        // Java ensure_ends_with 仅 1 参数（checkMethodArgCount(args, 1)）
        let err = render_out("${'x'?ensure_ends_with('x', 'x')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?ensure_ends_with(...) expects 1 argument but has received 2."),
            "{err}"
        );
    }

    #[test]
    fn arg_count_messages_match_java() {
        // _MessageUtil.newArgCntError：max-min==1 → "expects 1 or 2 arguments"；
        // argCnt==0 → "but has received none."
        let err = render_out("${'x'?keep_before()}").unwrap_err().to_string();
        assert!(
            err.contains("?keep_before(...) expects 1 or 2 arguments but has received none."),
            "{err}"
        );
        let err = render_out("${'x'?keep_before('x', 'i', 'x')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?keep_before(...) expects 1 or 2 arguments but has received 3."),
            "{err}"
        );
        let err = render_out("${'x'?ensure_starts_with('x', 'x', 'x', 'x')}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?ensure_starts_with(...) expects 1 to 3 arguments but has received 4."),
            "{err}"
        );
        // "m" flag 无正则 → 报错（RegexpHelper.checkOnlyHasNonRegexpFlags）
        let err = render_out("${'x'?keep_before('x', 'm')}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("\"m\" flag"), "{err}");
    }

    // -----------------------------------------------------------------------
    // truncate 家族单元测试
    // -----------------------------------------------------------------------

    #[test]
    fn truncate_no_truncation_needed() {
        // 串长 ≤ max_len → 原样返回
        assert_eq!(render_out("${'hello'?truncate(10)}").unwrap(), "hello");
        assert_eq!(render_out("${'hello'?truncate(5)}").unwrap(), "hello");
    }

    #[test]
    fn truncate_basic() {
        // "hello world", truncate(8) → "hello..."
        // UTF-16: 'hello world' = 11 码元；8 码元预算；terminator "..." = 3 码元；
        // 保留 8-3=5 码元 → "hello" + "..." → "hello..."
        assert_eq!(
            render_out("${'hello world'?truncate(8)}").unwrap(),
            "hello..."
        );
    }

    #[test]
    fn truncate_zero_length() {
        // max_len <= 0 → 空串
        assert_eq!(render_out("${'hello'?truncate(0)}").unwrap(), "");
        assert_eq!(render_out("${'hello'?truncate(-1)}").unwrap(), "");
    }

    #[test]
    fn truncate_custom_terminator() {
        assert_eq!(
            render_out("${'hello world'?truncate(7, '!')}").unwrap(),
            "hello !"
        );
    }

    #[test]
    fn truncate_terminator_wont_fit() {
        // max_len=2, terminator="..." (3 UTF-16) → max_len < terminator len → 空串
        assert_eq!(render_out("${'hello'?truncate(2)}").unwrap(), "");
    }

    #[test]
    fn truncate_non_bmp() {
        // 😀 (U+1F600) 占 2 个 UTF-16 码元；"a😀b😀c" → UTF-16 长度=7
        // truncate(5) → 保留 5-3=2 码元 → "a" + "..." → "a..."
        assert_eq!(
            render_out("${'a\u{1F600}b\u{1F600}c'?truncate(5)}").unwrap(),
            "a..."
        );
    }

    #[test]
    fn truncate_w_basic() {
        assert_eq!(
            render_out("${'one two three four'?truncate_w(2)}").unwrap(),
            "one two..."
        );
    }

    #[test]
    fn truncate_w_no_truncation() {
        assert_eq!(render_out("${'one two'?truncate_w(5)}").unwrap(), "one two");
    }

    #[test]
    fn truncate_w_custom_terminator() {
        assert_eq!(
            render_out("${'a b c d'?truncate_w(2, ' [more]')}").unwrap(),
            "a b [more]"
        );
    }

    #[test]
    fn truncate_c_basic() {
        // "hello world" = 11 char; truncate_c(8 char) → 保留 8-3=5 char → "hello..."
        assert_eq!(
            render_out("${'hello world'?truncate_c(8)}").unwrap(),
            "hello..."
        );
    }

    #[test]
    fn truncate_c_non_bmp() {
        // 😀 (U+1F600) 是一个 Unicode char（code point）
        // "a😀b😀c" = 5 Unicode chars
        // truncate_c(4) → 保留 4-3=1 char → "a..."
        assert_eq!(
            render_out("${'a\u{1F600}b\u{1F600}c'?truncate_c(4)}").unwrap(),
            "a..."
        );
    }

    #[test]
    fn truncate_c_custom_terminator() {
        assert_eq!(
            render_out("${'hello world'?truncate_c(6, '!')}").unwrap(),
            "hello!"
        );
    }

    #[test]
    fn truncate_c_no_truncation() {
        assert_eq!(render_out("${'hi'?truncate_c(5)}").unwrap(), "hi");
    }

    #[test]
    fn truncate_m_not_supported() {
        let err = render_out("${'hello'?truncate_m(5)}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("truncate_m"), "{err}");
        assert!(err.contains("isn't supported yet"), "{err}");
    }

    #[test]
    fn truncate_w_m_not_supported() {
        let err = render_out("${'hello'?truncate_w_m(5)}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("truncate_w_m"), "{err}");
    }

    #[test]
    fn truncate_c_m_not_supported() {
        let err = render_out("${'hello'?truncate_c_m(5)}")
            .unwrap_err()
            .to_string();
        assert!(err.contains("truncate_c_m"), "{err}");
    }

    #[test]
    fn truncate_arg_count_validation() {
        let err = render_out("${'x'?truncate()}").unwrap_err().to_string();
        assert!(
            err.contains("?truncate(...) expects 1 or 2 arguments but has received none."),
            "{err}"
        );
        let err = render_out("${'x'?truncate(1, '...', 2)}")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("?truncate(...) expects 1 or 2 arguments but has received 3."),
            "{err}"
        );
    }
}
