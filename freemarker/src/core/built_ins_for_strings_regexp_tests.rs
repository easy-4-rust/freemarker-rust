//! 测试 —— 自 built_ins_for_strings_regexp.rs 拆出（#[cfg(test)] 模块；由主文件
//! #[cfg(test)] #[path] 声明，仅测试构建时编译）。

use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用匹配器数据（对应 ?matches 构造的 RegexMatchData）
    fn data(pattern: &str, input: &str) -> Rc<RegexMatchData> {
        Rc::new(RegexMatchData {
            pattern: pattern.to_string(),
            input: input.to_string(),
            flags: FlagSet::default(),
            parts: RefCell::new(None),
            whole: RefCell::new(None),
        })
    }

    #[test]
    fn flags_parse() {
        let f = parse_flags("ir").unwrap();
        assert!(f.case_insensitive && f.regexp && !f.first_only);
        assert_eq!(parse_flags("Ix").unwrap(), FlagSet::default());
    }

    #[test]
    fn replace_regex_with_groups() {
        let f = parse_flags("r").unwrap();
        let re = compile_pattern("(a)(b)", &f).unwrap();
        assert_eq!(replace_all(&re, "ab ab", "$2$1").unwrap(), "ba ba");
        assert_eq!(replace_first(&re, "ab ab", "$2$1").unwrap(), "ba ab");
        assert_eq!(replace_all(&re, "ab", "${2}${1}").unwrap(), "ba");
    }

    #[test]
    fn replace_literal_ci_basic() {
        assert_eq!(replace_literal_ci("FoobarfOO", "foo", "X", false), "XbarX");
        assert_eq!(replace_literal_ci("FoobarfOO", "foo", "X", true), "XbarfOO");
    }

    /// 整串匹配按 Java Matcher.matches() 锚定：懒量词也要吃掉整个输入
    /// （探针：Java "123"?matches(r"(\d+?)") → c=true、groups=123,123）
    #[test]
    fn whole_match_lazy_quantifier_anchored() {
        let (matched, groups) = data(r"(\d+?)", "123").whole().unwrap();
        assert!(matched);
        assert_eq!(
            groups,
            vec![Some("123".to_string()), Some("123".to_string())]
        );
    }

    /// matches() 失败时 ?groups?size 仍为模式组数+1（Java matcher.groupCount()）
    /// （探针：Java "x12"?matches(r"y(\d)(\d)") → gsize=3、size=0）
    #[test]
    fn whole_no_match_keeps_pattern_group_count() {
        let (matched, groups) = data(r"y(\d)(\d)", "x12").whole().unwrap();
        assert!(!matched);
        assert_eq!(groups.len(), 3);
        assert!(groups.iter().all(|g| g.is_none()));
        // ?matches 序列角色（find）在整串不匹配时为空
        assert_eq!(data(r"y(\d)(\d)", "x12").parts().unwrap().len(), 0);
    }

    /// 部分匹配不算整串匹配：?groups 仍以模式组数计且访问报错（Java 探针：
    /// "x12"?matches(r"x(\d)") → c=false、gsize=2、groups[0] 报
    /// "Failed to read regular expression match group"）
    #[test]
    fn whole_partial_match_is_not_whole() {
        let (matched, groups) = data(r"x(\d)", "x12").whole().unwrap();
        assert!(!matched);
        assert_eq!(groups.len(), 2);
        let seq = WholeGroupsSeq(data(r"x(\d)", "x12"));
        assert_eq!(seq.size().unwrap(), 2);
        let err = seq.get(0).unwrap_err().to_string();
        assert_eq!(err, "Failed to read regular expression match group");
    }

    /// 匹配成功但某组未参与 → None（Java matcher.group(i) == null）
    /// （探针：Java "x2"?matches(r"x(\d)?(\d)") → g0=x2、g1 空串、g2=2）
    #[test]
    fn whole_match_null_group() {
        let (matched, groups) = data(r"x(\d)?(\d)", "x2").whole().unwrap();
        assert!(matched);
        assert_eq!(
            groups,
            vec![Some("x2".to_string()), None, Some("2".to_string())]
        );
    }

    /// 整体 ?groups 的未参与组 → 空串标量（Java new SimpleScalar(null)）
    #[test]
    fn whole_groups_seq_null_group_renders_empty() {
        let seq = WholeGroupsSeq(data(r"x(\d)?(\d)", "x2"));
        assert_eq!(seq.size().unwrap(), 3);
        assert_eq!(seq.get(0).unwrap().get_scalar().unwrap(), "x2");
        assert_eq!(seq.get(1).unwrap().get_scalar().unwrap(), "");
        assert_eq!(seq.get(2).unwrap().get_scalar().unwrap(), "2");
    }

    /// 整体 ?groups 越界访问同报 "Failed to read regular expression match group"
    /// （Java RegexMatchModel.get 捕获 IndexOutOfBoundsException）
    #[test]
    fn whole_groups_seq_out_of_range_errors() {
        let seq = WholeGroupsSeq(data(r"x(\d)(\d)", "x12"));
        let err = seq.get(5).unwrap_err().to_string();
        assert_eq!(err, "Failed to read regular expression match group");
    }

    /// 多行 flag 下的整串匹配：^$ 行锚不影响整串锚定
    /// （探针：Java "12\n34"?matches(r"(?m)^\d+$") → false；"12" → true）
    #[test]
    fn whole_match_multiline_anchoring() {
        let d = data(r"(?m)^\d+$", "12\n34");
        assert!(!d.whole().unwrap().0);
        let d = data(r"(?m)^\d+$", "12");
        assert!(d.whole().unwrap().0);
    }

    /// find() 子匹配：非空匹配后紧跟的空匹配也要保留（Java 探针：
    /// "12x"?matches(r"\d*") → [12][][]、size=3；fancy-regex 的 captures_iter
    /// 会跳过紧跟匹配的空匹配，故手动循环复刻 Java 语义）
    #[test]
    fn parts_keep_empty_match_after_match() {
        let parts = data(r"\d*", "12x").parts().unwrap();
        let rendered: Vec<&str> = parts.iter().map(|p| p.matched.as_str()).collect();
        assert_eq!(rendered, vec!["12", "", ""]);
        // 每个子匹配的组 0 = 匹配段本身
        assert_eq!(parts[1].groups, vec![Some("".to_string())]);
    }

    /// 连续零宽匹配（Java 探针："ab"?matches(r"x*") → [][][]、size=3）
    #[test]
    fn parts_sequence_of_empty_matches() {
        let parts = data(r"x*", "ab").parts().unwrap();
        let rendered: Vec<&str> = parts.iter().map(|p| p.matched.as_str()).collect();
        assert_eq!(rendered, vec!["", "", ""]);
    }

    /// 反向引用（Fancy 后端）的零宽推进与 Java 一致
    /// （Java 探针："aax"?matches(r"(a*)\1") → [aa][][]、size=3）
    #[test]
    fn parts_zero_width_backref() {
        let parts = data(r"(a*)\1", "aax").parts().unwrap();
        let rendered: Vec<&str> = parts.iter().map(|p| p.matched.as_str()).collect();
        assert_eq!(rendered, vec!["aa", "", ""]);
        let parts = data(r"(a*)\1", "aa").parts().unwrap();
        let rendered: Vec<&str> = parts.iter().map(|p| p.matched.as_str()).collect();
        assert_eq!(rendered, vec!["aa", ""]);
    }

    /// 空输入整串匹配（Java 探针：""?matches(r"\d*") → c=true、size=1、gsize=1）
    #[test]
    fn whole_match_empty_input() {
        let d = data(r"\d*", "");
        let (matched, groups) = d.whole().unwrap();
        assert!(matched);
        assert_eq!(groups, vec![Some("".to_string())]);
        assert_eq!(d.parts().unwrap().len(), 1);
    }
}
