//! 词法器测试 —— 自 lexer.rs 拆出（#[cfg(test)] 模块；由主文件 #[cfg(test)]
//! #[path] 声明，仅测试构建时编译）。

use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(name: &str, text: &str, strict: bool) -> Lexer {
        Lexer::new(name, text, strict)
    }

    fn tokens(l: &mut Lexer, ctx: ExprCtx) -> Vec<Tok> {
        let mut out = Vec::new();
        loop {
            let (t, _, _, _, _) = l.next_expr_token(ctx).unwrap();
            let done = t == Tok::Eof;
            out.push(t);
            if done {
                break;
            }
        }
        out
    }

    #[test]
    fn expr_tokens_basic() {
        let mut l = lex("t", "a + b*2 != (x??) ?name", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Ident("a".into()),
                Tok::Plus,
                Tok::Ident("b".into()),
                Tok::Times,
                Tok::Number("2".into()),
                Tok::NotEq,
                Tok::OpenParen,
                Tok::Ident("x".into()),
                Tok::Exists,
                Tok::CloseParen,
                Tok::Builtin,
                Tok::Ident("name".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn word_operators_and_keywords() {
        let mut l = lex("t", "a lt b gt c and d or e", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        // `and`/`or` 不是 token（仅 `&&`/`\and`/`||`）；lt/gt 是运算符
        assert!(ts.contains(&Tok::Lt));
        assert!(ts.contains(&Tok::Gt));
        assert!(!ts.contains(&Tok::And));
        assert!(!ts.contains(&Tok::Or));
        let mut l = lex("t", "a \\and b || c && d &amp;&amp; e", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts.iter().filter(|t| **t == Tok::And).count(),
            3,
            "\\and、&&、&amp;&amp; 是 And（|| 是 Or）"
        );
        assert_eq!(ts.iter().filter(|t| **t == Tok::Or).count(), 1);
    }

    #[test]
    fn number_forms() {
        let mut l = lex("t", "1 1L 1F 1D 1.5 1e3 0x1A 1..5", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Number("1".into()),
                Tok::Number("1L".into()),
                Tok::Number("1F".into()),
                Tok::Number("1D".into()),
                Tok::Number("1.5".into()),
                Tok::Number("1e3".into()),
                Tok::Number("0x1A".into()),
                Tok::Number("1".into()),
                Tok::DotDot,
                Tok::Number("5".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn string_and_raw_string() {
        let mut l = lex("t", r#""a\n\t\"\\" 'x' r"raw\ny" "#, true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Str("a\\n\\t\\\"\\\\".into()),
                Tok::Str("x".into()),
                Tok::RawStr("raw\\ny".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn gt_ends_tag_outside_parens() {
        let mut l = lex("t", "a > b", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        // `a` 后 `>` 直接结束标签（TagEnd），`b` 不再是表达式 token
        assert_eq!(
            ts,
            vec![
                Tok::Ident("a".into()),
                Tok::TagEnd,
                Tok::Ident("b".into()),
                Tok::Eof
            ]
        );
        let mut l = lex("t", "(a > b)", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert!(ts.contains(&Tok::Gt));
        let mut l = lex("t", "a >= b", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        // 标签内 `>=` 不是 GTE（`>` 结束标签，`=` 留作文本）—— 与 Java 一致
        assert!(!ts.contains(&Tok::Gte));
        assert_eq!(ts[1], Tok::TagEnd);

        let mut l = lex("t", "a > b]", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: true });
        assert_eq!(
            ts,
            vec![
                Tok::Ident("a".into()),
                Tok::Gt,
                Tok::Ident("b".into()),
                Tok::TagEnd,
                Tok::Eof
            ]
        );

        let mut l = lex("t", "a >= b]", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: true });
        assert!(ts.contains(&Tok::Gte));
    }

    #[test]
    fn interp_ctx_closes_with_brace() {
        let mut l = lex("t", "a + }", true);
        let ts = tokens(&mut l, ExprCtx::Interp);
        assert_eq!(
            ts,
            vec![Tok::Ident("a".into()), Tok::Plus, Tok::InterpEnd, Tok::Eof]
        );
    }

    #[test]
    fn curly_bracket_hash_vs_interp_end() {
        let mut l = lex("t", r#"{"a": 1} x"#, true);
        let ts = tokens(&mut l, ExprCtx::Interp);
        assert_eq!(
            ts,
            vec![
                Tok::OpenCurly,
                Tok::Str("a".into()),
                Tok::Colon,
                Tok::Number("1".into()),
                Tok::CloseCurly,
                Tok::Ident("x".into()),
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn bracket_list_depth() {
        let mut l = lex("t", "[1, 2] x", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: true });
        assert_eq!(
            ts,
            vec![
                Tok::OpenBracket,
                Tok::Number("1".into()),
                Tok::Comma,
                Tok::Number("2".into()),
                Tok::CloseBracket,
                Tok::Ident("x".into()),
                Tok::Eof,
            ]
        );
        // 方括号语法：深度 0 的 `]` 结束标签
        let mut l = lex("t", "[1, 2]]", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: true });
        assert_eq!(
            ts,
            vec![
                Tok::OpenBracket,
                Tok::Number("1".into()),
                Tok::Comma,
                Tok::Number("2".into()),
                Tok::CloseBracket,
                Tok::TagEnd,
                Tok::Eof,
            ]
        );
        // 角度语法下深度 0 的 `]` 报错（`[1, 2]` 的 `]` 关闭列表字面量，
        // 第二个 `]` 在深度 0 → 报错）
        let mut l = lex("t", "[1, 2]]", true);
        for _ in 0..5 {
            l.next_expr_token(ExprCtx::Tag { square: false }).unwrap();
        }
        let r = l.next_expr_token(ExprCtx::Tag { square: false });
        assert!(r.is_err());
    }

    #[test]
    fn text_scanning_rules() {
        // `a < b` 是文本（严格语法）
        let mut l = lex("t", "a < b", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "a < b");
        assert_eq!(s, TextStop::Eof);
        // 非严格语法 `<if x>` 是标签
        let mut l = lex("t", "x <if y>", false);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x ");
        assert_eq!(s, TextStop::Tag);
        // `<b>`（非指令名）是文本
        let mut l = lex("t", "a <b> c", false);
        let (t, _s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "a <b> c");
        // `$${` → 文本 `$` + 插值
        let mut l = lex("t", "$${x}", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "$");
        assert_eq!(s, TextStop::Interp);
        // `${` 插值开始
        let mut l = lex("t", "ab${x}", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "ab");
        assert_eq!(s, TextStop::Interp);
        // `#{` 传统插值
        let mut l = lex("t", "ab#{x}", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "ab");
        assert_eq!(s, TextStop::Interp);
        // `<#--` 注释标签开头
        let mut l = lex("t", "ab<#-- c -->", true);
        let (t, s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "ab");
        assert_eq!(s, TextStop::Tag);
    }

    #[test]
    fn comment_scanning() {
        let mut l = lex("t", "hello --] world -->", true);
        let (s, _, _) = l.scan_comment(false).unwrap();
        assert_eq!(s, "hello --] world ");
        let mut l = lex("t", "hello --> world --]", true);
        let (s, _, _) = l.scan_comment(true).unwrap();
        assert_eq!(s, "hello --> world ");
        let mut l = lex("t", "unclosed", true);
        assert!(l.scan_comment(false).is_err());
    }

    #[test]
    fn unparsed_scanning() {
        let mut l = lex("t", "a</#noparse>", true);
        let (s, _, _) = l.scan_unparsed("noparse").unwrap();
        assert_eq!(s, "a");
        let mut l = lex("t", "a</noparse>", true);
        let (s, _, _) = l.scan_unparsed("noparse").unwrap();
        assert_eq!(s, "a");
        let mut l = lex("t", "a</#comment>", true);
        let (s, _, _) = l.scan_unparsed("comment").unwrap();
        assert_eq!(s, "a");
        // 不匹配的结束标签视为内容
        let mut l = lex("t", "a</#noparse x>rest</#noparse>", true);
        let (s, _, _) = l.scan_unparsed("noparse").unwrap();
        assert_eq!(s, "a</#noparse x>rest");
    }

    #[test]
    fn non_strict_tag_detection() {
        // 非严格：`<if x>` 标签、`<if>` 文本（IF 需 BLANK）、`<else>` 标签、`<foo>` 文本
        let mut l = lex("t", "x <if y>", false);
        let (_, stop) = l.scan_text_chunk().unwrap();
        assert_eq!(stop, TextStop::Tag);
        assert!(l.starts_tag());
        let mut l = lex("t", "x <if>", false);
        let (t, _) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x <if>");
        let mut l = lex("t", "x <else>", false);
        let (t, _s) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x ");
        let mut l = lex("t", "x <else y>", false);
        let (t, _) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x <else y>");
        // 严格：`<if x>` 是文本
        let mut l = lex("t", "x <if y>", true);
        let (t, _) = l.scan_text_chunk().unwrap();
        assert_eq!(t, "x <if y>");
    }

    #[test]
    fn escaped_identifiers() {
        let mut l = lex("t", r"a\-b\.c\:d\#e", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(ts[0], Tok::Ident("a-b.c:d#e".into()));
    }

    #[test]
    fn ident_special_chars() {
        // `$`、非 ASCII 都是标识符字符
        let mut l = lex("t", "$foo _bar français x2", true);
        let ts = tokens(&mut l, ExprCtx::Tag { square: false });
        assert_eq!(
            ts,
            vec![
                Tok::Ident("$foo".into()),
                Tok::Ident("_bar".into()),
                Tok::Ident("français".into()),
                Tok::Ident("x2".into()),
                Tok::Eof,
            ]
        );
    }
}
