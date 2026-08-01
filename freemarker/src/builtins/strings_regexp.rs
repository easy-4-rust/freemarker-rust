//! 正则相关内建 —— 对应 Java `BuiltInsForStringsRegexp.java`（matches/groups/replace/split；
//! Java 中 `?replace`/`?replace_re`/`?split` 的 flags 语义同源 RegexpHelper）。
//!
//! 语义要点（Java 对照）：
//! - `?matches(pattern[, flags])` → RegexMatchModel（RegexMatchModel.java 对应结构）：
//!   boolean 角色 = 整串匹配（Matcher.matches）；序列角色 = find() 子匹配序列
//!   （元素为 MatchWithGroups：标量 = 匹配段，`?groups` = 该次匹配的捕获组）；
//!   `?groups` 在整体模型上 = 整串匹配的捕获组序列（组未参与 → null）；
//! - flags：i/m/c/s/f/r（RegexpHelper.parseFlagString）；未知 flag 仅告警忽略；
//!   'r' = 正则模式（replace/split）；'f' = 只替换首个；
//! - `?replace` 正则模式替换串支持 `$1` 组引用（Java Matcher.replaceAll）；
//! - 正则引擎：fancy-regex（支持反向引用/环视；Java 反向引用差异见 docs/05 §3；
//!   占有量词 `*+`/`++` Java 支持而 fancy-regex 不支持 → 编译报错，属已知差异）。

use crate::builtins::eval_util::{arg_count, arg_string, check_arg_count, coerce_to_string};
use crate::core::environment::model_to_string;
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::{
    TModel, TemplateBooleanModel, TemplateCollectionModel, TemplateMethodModelEx,
    TemplateSequenceModel,
};
use std::cell::RefCell;
use std::rc::Rc;

/// flags 集合（对应 Java RegexpHelper 的 long flags 位）
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlagSet {
    pub case_insensitive: bool,
    pub multiline: bool,
    pub comments: bool,
    pub dotall: bool,
    pub regexp: bool,
    pub first_only: bool,
}

/// 解析 flags 字符串（Java RegexpHelper.parseFlagString：i/m/c/s/r/f；未知字符仅告警忽略）
pub fn parse_flags(s: &str) -> Result<FlagSet> {
    let mut f = FlagSet::default();
    for c in s.chars() {
        match c {
            'i' => f.case_insensitive = true,
            'm' => f.multiline = true,
            'c' => f.comments = true,
            's' => f.dotall = true,
            'r' => f.regexp = true,
            'f' => f.first_only = true,
            _ => {} // Java：未知 flag 仅 logFlagWarning（本实现不记日志）
        }
    }
    Ok(f)
}

impl FlagSet {
    /// 非正则模式下 m/s/c flag → 报错（Java checkOnlyHasNonRegexpFlags strict=true；
    /// keep_* 家族用；replace/split 非正则模式仅告警不报错）
    pub fn check_non_regexp_strict(&self, bi: &str) -> Result<()> {
        let flag = if self.multiline {
            Some("m")
        } else if self.dotall {
            Some("s")
        } else if self.comments {
            Some("c")
        } else {
            None
        };
        match flag {
            Some(f) => Err(TemplateError::misc(format!(
                "?{bi} doesn't support the \"{f}\" flag without the \"r\" flag."
            ))),
            None => Ok(()),
        }
    }

    /// 非正则模式下 m/s/c flag → 仅告警（Java checkNonRegexpFlags；replace/split 用）
    pub fn check_non_regexp_warn(&self, _bi: &str) {}

    /// 编译为内联 flags 前缀（fancy-regex 支持 (?i)(?m)(?s)(?x)）
    fn inline_prefix(&self) -> String {
        let mut out = String::new();
        if self.case_insensitive {
            out.push('i');
        }
        if self.multiline {
            out.push('m');
        }
        if self.dotall {
            out.push('s');
        }
        if self.comments {
            out.push('x');
        }
        if out.is_empty() {
            String::new()
        } else {
            format!("(?{out})")
        }
    }
}

/// 编译正则（Java RegexpHelper.getPattern；错误消息对齐 "Failed to compile regular expression"）
pub fn compile_pattern(pattern: &str, flags: &FlagSet) -> Result<fancy_regex::Regex> {
    let pat = format!("{}{}", flags.inline_prefix(), pattern);
    fancy_regex::Regex::new(&pat)
        .map_err(|e| TemplateError::misc(format!("Failed to compile regular expression: {e}")))
}

/// 一次 find 匹配 + 捕获组（Java `RegexMatchModel.MatchWithGroups`）
#[derive(Debug, Clone)]
pub struct MatchWithGroups {
    pub matched: String,
    pub groups: Vec<Option<String>>,
}

/// 匹配器模型数据（Java `RegexMatchModel`：boolean=整串匹配、序列=子匹配列表、
/// 方法=再调 matches(pattern[, flags])；内部槽位供 ?groups 读取）
pub struct RegexMatchData {
    pub pattern: String,
    pub input: String,
    pub flags: FlagSet,
    /// find() 子匹配列表缓存（Java matchingInputParts）
    pub parts: RefCell<Option<Vec<MatchWithGroups>>>,
    /// 整串匹配结果缓存（Java entireInputMatched/firedEntireInputMatcher）
    pub whole: RefCell<Option<(bool, Vec<Option<String>>)>>,
}

impl RegexMatchData {
    fn compute_parts(&self) -> Result<Vec<MatchWithGroups>> {
        let re = compile_pattern(&self.pattern, &self.flags)?;
        let mut parts = Vec::new();
        let mut last_end = 0usize;
        for caps_r in re.captures_iter(&self.input) {
            let caps = caps_r.map_err(|e| {
                TemplateError::misc(format!("Failed to match regular expression: {e}"))
            })?;
            let m0 = caps.get(0).expect("group 0 always exists");
            // Java matcher.find() 在零宽匹配后自动前进（无死循环）
            if m0.end() == last_end && m0.start() == m0.end() {
                continue;
            }
            let mut groups = Vec::new();
            for i in 0..caps.len() {
                groups.push(caps.get(i).map(|gm| gm.as_str().to_string()));
            }
            let matched = self.input[m0.start()..m0.end()].to_string();
            parts.push(MatchWithGroups { matched, groups });
            last_end = m0.end();
        }
        Ok(parts)
    }

    /// 整串匹配（Java Matcher.matches）→ (是否匹配, 捕获组)
    fn compute_whole(&self) -> Result<(bool, Vec<Option<String>>)> {
        let re = compile_pattern(&self.pattern, &self.flags)?;
        let caps = re
            .captures(&self.input)
            .map_err(|e| TemplateError::misc(format!("Failed to match regular expression: {e}")))?;
        // Java Matcher.matches：整串必须整体匹配（隐含锚定）
        let matched = caps.as_ref().is_some_and(|c| {
            c.get(0)
                .is_some_and(|m0| m0.start() == 0 && m0.end() == self.input.len())
        });
        let mut groups = Vec::new();
        if let Some(caps) = caps {
            for i in 0..caps.len() {
                groups.push(caps.get(i).map(|gm| gm.as_str().to_string()));
            }
        }
        Ok((matched, groups))
    }

    pub fn parts(&self) -> Result<Vec<MatchWithGroups>> {
        if let Some(p) = &*self.parts.borrow() {
            return Ok(p.clone());
        }
        let p = self.compute_parts()?;
        *self.parts.borrow_mut() = Some(p.clone());
        Ok(p)
    }

    pub fn whole(&self) -> Result<(bool, Vec<Option<String>>)> {
        if let Some(w) = &*self.whole.borrow() {
            return Ok(w.clone());
        }
        let w = self.compute_whole()?;
        *self.whole.borrow_mut() = Some(w.clone());
        Ok(w)
    }
}

/// 构造匹配器模型（boolean + sequence + collection + method + internal 槽位）
pub fn matcher_model(data: Rc<RegexMatchData>) -> TModel {
    let bool_m: Rc<dyn TemplateBooleanModel> = Rc::new(MatcherBool(data.clone()));
    let seq: Rc<dyn TemplateSequenceModel> = Rc::new(MatcherSeq(data.clone()));
    let coll: Rc<dyn TemplateCollectionModel> = Rc::new(MatcherColl(data.clone()));
    let method: Rc<dyn TemplateMethodModelEx> = Rc::new(MatcherMethod(data.clone()));
    TModel {
        boolean: Some(bool_m),
        sequence: Some(seq),
        collection: Some(coll),
        method: Some(method),
        internal: Some(data),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

struct MatcherBool(Rc<RegexMatchData>);
impl TemplateBooleanModel for MatcherBool {
    fn as_boolean(&self) -> Result<bool> {
        Ok(self.0.whole()?.0)
    }
}

struct MatcherSeq(Rc<RegexMatchData>);
impl TemplateSequenceModel for MatcherSeq {
    fn get(&self, index: usize) -> Result<TModel> {
        let parts = self.0.parts()?;
        parts
            .get(index)
            .cloned()
            .map(match_element_model)
            .ok_or_else(|| TemplateError::misc("Sequence index out of bounds"))
    }
    fn size(&self) -> Result<usize> {
        Ok(self.0.parts()?.len())
    }
}

struct MatcherColl(Rc<RegexMatchData>);
impl TemplateCollectionModel for MatcherColl {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        let parts = self.0.parts()?;
        Ok(Box::new(
            parts.into_iter().map(|m| Ok(match_element_model(m))),
        ))
    }
}

/// `x?matches`（无参）→ 方法模型：exec([pattern[, flags]]) 返回新匹配器
struct MatcherMethod(Rc<RegexMatchData>);
impl TemplateMethodModelEx for MatcherMethod {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel> {
        let bi = "matches";
        if args.is_empty() || args.len() > 2 {
            return Err(TemplateError::misc(format!(
                "The {bi} built-in expects 1 to 2 arguments, but got {}.",
                args.len()
            )));
        }
        let pattern = simple_coerce_string(&args[0])?;
        let flags = if args.len() > 1 {
            parse_flags(&simple_coerce_string(&args[1])?)?
        } else {
            FlagSet::default()
        };
        Ok(matcher_model(Rc::new(RegexMatchData {
            pattern,
            input: self.0.input.clone(),
            flags,
            parts: RefCell::new(None),
            whole: RefCell::new(None),
        })))
    }
}

/// 无 Environment 时的简化字符串强制转换（方法模型 exec 路径；Java
/// EvalUtil.coerceModelToStringOrMarkup 的标量/数字/布尔子集）
fn simple_coerce_string(m: &TModel) -> Result<String> {
    if let Some(s) = &m.scalar {
        return s.as_string();
    }
    if let Some(n) = &m.number {
        return Ok(n.as_number()?.to_plain_string());
    }
    if let Some(b) = &m.boolean {
        return Ok(if b.as_boolean()? { "true" } else { "false" }.to_string());
    }
    Err(TemplateError::type_mismatch(
        "string-like value",
        m.type_name,
    ))
}

/// 匹配元素模型（Java MatchWithGroups：标量 + ?groups 槽位）
pub fn match_element_model(m: MatchWithGroups) -> TModel {
    TModel {
        scalar: Some(Rc::new(crate::template::SimpleScalar(m.matched.clone()))),
        internal: Some(Rc::new(m)),
        type_name: "string",
        kind: crate::template::ModelKind::Scalar,
        ..TModel::nothing()
    }
}

/// 求值目标（缺失 → 报错；Java matchesBI 等要求非 null）
fn eval_target(env: &mut Environment, target: &Expr) -> Result<TModel> {
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    Ok(m)
}

/// ?matches(pattern[, flags]) —— Java BuiltInsForStringsRegexp.matchesBI:
/// 返回 RegexMatchModel（boolean=整串匹配；列表=子匹配；方法=MatcherBuilder）。
/// 未知 flag 忽略；'f' 仅告警。
pub fn matches(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("matches", args, 1, 2)?;
    let tm = eval_target(env, target)?;
    let input = model_to_string(env, &tm)?;
    let pattern = arg_string(env, args, 0)?;
    let flags = if arg_count(args) > 1 {
        parse_flags(&arg_string(env, args, 1)?)?
    } else {
        FlagSet::default()
    };
    Ok(Some(matcher_model(Rc::new(RegexMatchData {
        pattern,
        input,
        flags,
        parts: RefCell::new(None),
        whole: RefCell::new(None),
    }))))
}

/// ?groups —— Java BuiltInsForStringsRegexp.groupsBI:
/// 目标为 RegexMatchModel → 整串匹配的捕获组序列；为 MatchWithGroups → 该次匹配的捕获组序列
pub fn groups(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("groups", args, 0, 0)?;
    let m = eval_target(env, target)?;
    if let Some(data) = m.internal::<RegexMatchData>() {
        let (_, g) = data.whole()?;
        return Ok(Some(groups_sequence(g)));
    }
    if let Some(mwg) = m.internal::<MatchWithGroups>() {
        return Ok(Some(groups_sequence(mwg.groups.clone())));
    }
    Err(TemplateError::misc(format!(
        "?groups is not applicable to a {} value",
        m.type_name
    )))
}

/// 捕获组序列（未参与匹配的组 → null/nothing）
fn groups_sequence(groups: Vec<Option<String>>) -> TModel {
    TModel::from_sequence(
        groups
            .into_iter()
            .map(|g| match g {
                Some(s) => TModel::from_scalar(s),
                None => TModel::nothing(),
            })
            .collect(),
    )
}

/// ?replace(sub[, replacement[, flags]]) / ?replace_re —— Java replace_reBI:
/// 无 'r' → StringUtil.replace（字面量全量/首个替换，'i' 大小写不敏感）；
/// 有 'r' → Matcher.replaceAll/replaceFirst（替换串支持 $N 组引用）。
pub fn replace(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("replace", args, 2, 3)?;
    let tm = eval_target(env, target)?;
    let s = coerce_to_string(env, &tm)?;
    let from = arg_string(env, args, 0)?;
    let to = arg_string(env, args, 1)?;
    let flags = if arg_count(args) > 2 {
        parse_flags(&arg_string(env, args, 2)?)?
    } else {
        FlagSet::default()
    };
    let result = if !flags.regexp {
        flags.check_non_regexp_warn("replace");
        // Java StringUtil.replace（StringUtil.java:791）：空 oldSub 在每字符前后插入；
        // caseInsensitive 时两侧 toLowerCase 后取下标（Java 同款索引假设）
        if from.is_empty() {
            if to.is_empty() {
                s
            } else if flags.first_only {
                format!("{to}{s}")
            } else {
                let mut out = String::new();
                out.push_str(&to);
                for c in s.chars() {
                    out.push(c);
                    out.push_str(&to);
                }
                out
            }
        } else if flags.case_insensitive {
            replace_literal_ci(&s, &from, &to, flags.first_only)
        } else {
            if flags.first_only {
                s.replacen(&from, &to, 1)
            } else {
                s.replace(&from, &to)
            }
        }
    } else {
        let re = compile_pattern(&from, &flags)?;
        if flags.first_only {
            replace_first(&re, &s, &to)?
        } else {
            replace_all(&re, &s, &to)?
        }
    };
    Ok(Some(TModel::from_scalar(result)))
}

/// 大小写不敏感的字符串替换（Java StringUtil.replace caseInsensitive：两侧 toLowerCase
/// 后取下标、回原串切片——与 Java 相同的索引假设）
fn replace_literal_ci(s: &str, from: &str, to: &str, first_only: bool) -> String {
    let input = s.to_lowercase();
    let oldsub = from.to_lowercase();
    let mut out = String::new();
    let mut b = 0usize;
    loop {
        let found = input[b..].find(&oldsub).map(|i| i + b);
        match found {
            Some(e) => {
                out.push_str(&s[b..e]);
                out.push_str(to);
                b = e + from.len();
                if first_only || b > s.len() {
                    break;
                }
            }
            None => break,
        }
    }
    out.push_str(&s[b.min(s.len())..]);
    out
}

/// 正则替换（Java Matcher.replaceAll；$N 组引用，$$ 转义 $）
pub fn replace_all(re: &fancy_regex::Regex, s: &str, to: &str) -> Result<String> {
    let mut out = String::new();
    let mut last = 0usize;
    let mut matches = Vec::new();
    for m in re.find_iter(s) {
        matches.push(m.map_err(|e| TemplateError::misc(format!("Regexp error: {e}")))?);
    }
    for m in &matches {
        out.push_str(&s[last..m.start()]);
        out.push_str(&expand_replacement(re, s, m, to)?);
        last = m.end();
    }
    out.push_str(&s[last..]);
    Ok(out)
}

/// 正则替换首个（Java Matcher.replaceFirst）
pub fn replace_first(re: &fancy_regex::Regex, s: &str, to: &str) -> Result<String> {
    let mut it = re.find_iter(s);
    let m = it.next();
    match m {
        None => Ok(s.to_string()),
        Some(m) => {
            let m = m.map_err(|e| TemplateError::misc(format!("Regexp error: {e}")))?;
            let mut out = String::new();
            out.push_str(&s[..m.start()]);
            out.push_str(&expand_replacement(re, s, &m, to)?);
            out.push_str(&s[m.end()..]);
            Ok(out)
        }
    }
}

/// 替换串展开（Java Matcher.appendReplacement 的 $N/$$ 语义；${N} 同 $N）
fn expand_replacement(
    re: &fancy_regex::Regex,
    input: &str,
    _m: &fancy_regex::Match,
    to: &str,
) -> Result<String> {
    let caps = re.captures(input).map_err(|e| {
        TemplateError::misc(format!("Failed to get regular expression captures: {e}"))
    })?;
    let mut out = String::new();
    let chars: Vec<char> = to.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '$' {
            if i + 1 < chars.len() && chars[i + 1] == '$' {
                out.push('$');
                i += 2;
                continue;
            }
            if i + 1 < chars.len() && chars[i + 1] == '{' {
                // ${N} 形式
                let mut j = i + 2;
                let mut num = String::new();
                while j < chars.len() && chars[j].is_ascii_digit() {
                    num.push(chars[j]);
                    j += 1;
                }
                if j < chars.len() && chars[j] == '}' {
                    if let Some(g) = group_of(&caps, &num) {
                        out.push_str(g);
                    }
                    i = j + 1;
                    continue;
                }
                out.push('$');
                i += 1;
                continue;
            }
            if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                let mut j = i + 1;
                let mut num = String::new();
                while j < chars.len() && chars[j].is_ascii_digit() && num.len() < 2 {
                    num.push(chars[j]);
                    j += 1;
                }
                if let Some(g) = group_of(&caps, &num) {
                    out.push_str(g);
                }
                i = j;
                continue;
            }
            out.push('$');
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    Ok(out)
}

fn group_of<'a>(caps: &'a Option<fancy_regex::Captures>, num: &str) -> Option<&'a str> {
    if let Some(c) = caps {
        if let Ok(idx) = num.parse::<usize>() {
            return c.get(idx).map(|gm| gm.as_str());
        }
    }
    None
}

/// 大小写不敏感字面量分割（Java StringUtil.split caseInsensitive）
fn split_literal_ci(s: &str, sep: &str) -> Vec<TModel> {
    if sep.is_empty() {
        return s
            .chars()
            .map(|c| TModel::from_scalar(c.to_string()))
            .collect();
    }
    let converted = s.to_lowercase();
    let split = sep.to_lowercase();
    let mut out = Vec::new();
    let mut next = 0usize;
    loop {
        let end = converted[next..].find(&split).map(|i| i + next);
        match end {
            Some(e) => {
                out.push(TModel::from_scalar(s[next..e].to_string()));
                next = e + sep.len();
            }
            None => {
                out.push(TModel::from_scalar(s[next..].to_string()));
                break;
            }
        }
    }
    out
}

/// ?split(sep[, flags]) —— Java BuiltInsForStringsBasic.split_BI:
/// 无 'r' → 字面量分割（含空串段）；有 'r' → 正则分割；'i'/'m'/'s'/'c' 非正则模式仅告警
pub fn split(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("split", args, 1, 2)?;
    let tm = eval_target(env, target)?;
    let s = coerce_to_string(env, &tm)?;
    let sep = arg_string(env, args, 0)?;
    let flags = if arg_count(args) > 1 {
        parse_flags(&arg_string(env, args, 1)?)?
    } else {
        FlagSet::default()
    };
    let parts: Vec<TModel> = if !flags.regexp {
        flags.check_non_regexp_warn("split");
        if flags.case_insensitive {
            // Java StringUtil.split(s, sep, caseInsensitive)：两侧 toLowerCase 后按下标切原串
            split_literal_ci(&s, &sep)
        } else {
            s.split(&sep)
                .map(|p| TModel::from_scalar(p.to_string()))
                .collect()
        }
    } else if sep.is_empty() {
        // 空正则分隔符：Java 按空串分割（每个字符一段；v1 单字符近似）
        s.chars()
            .map(|c| TModel::from_scalar(c.to_string()))
            .collect()
    } else {
        let re = compile_pattern(&sep, &flags)?;
        let mut out = Vec::new();
        let mut last = 0usize;
        for m in re.find_iter(&s) {
            let m = m.map_err(|e| TemplateError::misc(format!("Regexp error: {e}")))?;
            out.push(TModel::from_scalar(s[last..m.start()].to_string()));
            last = m.end();
        }
        out.push(TModel::from_scalar(s[last..].to_string()));
        // Java Pattern.split：丢弃尾部空段
        while out.len() > 1
            && out
                .last()
                .is_some_and(|m| m.get_scalar().is_ok_and(|s| s.is_empty()))
        {
            out.pop();
        }
        out
    };
    Ok(Some(TModel::from_sequence(parts)))
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
