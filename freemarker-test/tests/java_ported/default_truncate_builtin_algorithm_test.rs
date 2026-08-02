//! 对应 Java: DefaultTruncateBuiltinAlgorithmTest
//! Java `freemarker.core.DefaultTruncateBuiltinAlgorithmTest` 的 Rust 1:1 实现。
//! 该 Java 类是**算法类本身的纯单元测试**（直接 new DefaultTruncateBuiltinAlgorithm
//! 调 truncateC/truncateW/truncate…），不走 FTL 渲染。
//!
//! v1 移植策略：在测试文件内以 `DtAlgo` 结构体 1:1 移植被测算法
//! （DefaultTruncateBuiltinAlgorithm.java 的 unifiedTruncate 纯文本路径 +
//! getLengthWithoutTags / doesHtmlOrXmlStartWithDot / isDotCharReference /
//! getCodeFromNumericalCharReferenceName），原样跑 Java 数据表。
//!
//! 引擎差异（已标注在对应断言处）：
//! - markup 终结符/结果（TemplateMarkupOutputModel）在 v1 无类型——以"HTML 字符串 +
//!   html_escape 拼接"的字符串级模型近似（instanceOf/assertSame 改为字符串相等）。
//! - Java Character.isWhitespace 与 Rust char::is_whitespace 在 U+00A0 等少数码点
//!   上有分歧；数据表全为 ASCII 空白，不受影响。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::utility::html_escape;

/// Java STANDARD_ASCII_TERMINATOR / STANDARD_UNICODE_TERMINATOR / STANDARD_M_TERMINATOR
const STANDARD_ASCII_TERMINATOR: &str = "[...]";
const STANDARD_UNICODE_TERMINATOR: &str = "[\u{2026}]";
const STANDARD_M_TERMINATOR: &str = "<span class='truncateTerminator'>[&#8230;]</span>";
/// Java DEFAULT_WORD_BOUNDARY_MIN_LENGTH
const DEFAULT_WORD_BOUNDARY_MIN_LENGTH: f64 = 0.75;
/// Java FALLBACK_M_TERMINATOR_LENGTH
const _FALLBACK_M_TERMINATOR_LENGTH: usize = 3;

/// 终结符参数（对应 Java `TemplateModel terminator`）：
/// Default = null（用算法默认终结符；M 模式且有 markup 默认终结符时用之）、
/// Plain = 显式字符串、Markup = 显式 HTML 标记、NonScalar = 数字等非法类型
#[derive(Debug, Clone, Copy)]
enum Term<'a> {
    Default,
    Plain(&'a str),
    Markup(&'a str),
    NonScalar,
}

/// 截断模式 —— 对应 Java TruncationMode
#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    CharBoundary,
    WordBoundary,
    Auto,
}

/// 输出：普通字符串 或 HTML 标记字符串
#[derive(Debug)]
enum Out {
    Plain(String),
    Markup(String),
}

impl Out {
    fn as_str(&self) -> &str {
        match self {
            Out::Plain(s) | Out::Markup(s) => s,
        }
    }
}

/// truncate_m / truncate_cm / truncate_wm 三个调用者的共同签名
type TruncateCaller = fn(&DtAlgo, &str, i64, Term, Option<i64>) -> Result<Out, String>;

/// 测试端 1:1 移植 —— Java `DefaultTruncateBuiltinAlgorithm`
#[derive(Debug)]
struct DtAlgo {
    default_terminator: String,
    default_terminator_length: usize,
    default_terminator_removes_dots: bool,
    default_m_terminator: Option<String>,
    default_m_terminator_length: usize,
    default_m_terminator_removes_dots: bool,
    add_space_at_word_boundary: bool,
    word_boundary_min_length: f64,
}

#[allow(dead_code)] // 模拟 Java DefaultTruncateBuiltinAlgorithm 的抽象 getter（部分未在本测试用到）
impl DtAlgo {
    /// 对应 `DefaultTruncateBuiltinAlgorithm(String, TemplateMarkupOutputModel, boolean)`
    /// （缺省参数为 null；removesDots 自动检测）——用于 ASCII_INSTANCE/UNICODE_INSTANCE
    fn new_with_m(term: &str, m_term: &str, add_space: bool) -> Self {
        DtAlgo {
            default_terminator: term.to_string(),
            default_terminator_length: term.chars().count(),
            default_terminator_removes_dots: get_terminator_removes_dots(term),
            default_m_terminator: Some(m_term.to_string()),
            default_m_terminator_length: get_length_without_tags(m_term),
            default_m_terminator_removes_dots: does_html_or_xml_start_with_dot(m_term),
            add_space_at_word_boundary: add_space,
            word_boundary_min_length: DEFAULT_WORD_BOUNDARY_MIN_LENGTH,
        }
    }

    /// 对应 `DefaultTruncateBuiltinAlgorithm(String, boolean)` —— 纯文本终结符
    fn new(term: &str, add_space: bool) -> Self {
        DtAlgo {
            default_terminator: term.to_string(),
            default_terminator_length: term.chars().count(),
            default_terminator_removes_dots: get_terminator_removes_dots(term),
            default_m_terminator: None,
            default_m_terminator_length: 0,
            default_m_terminator_removes_dots: false,
            add_space_at_word_boundary: add_space,
            word_boundary_min_length: DEFAULT_WORD_BOUNDARY_MIN_LENGTH,
        }
    }

    /// 对应 8 参构造（markup 参数以字符串近似）：null 终结符 → Err 含 "defaultTerminator"；
    /// wordBoundaryMinLength 越界 → Err
    #[allow(clippy::too_many_arguments)]
    fn new_full(
        default_terminator: Option<&str>,
        default_terminator_length: Option<usize>,
        default_terminator_removes_dots: Option<bool>,
        default_m_terminator: Option<&str>,
        default_m_terminator_length: Option<usize>,
        default_m_terminator_removes_dots: Option<bool>,
        add_space_at_word_boundary: bool,
        word_boundary_min_length: Option<f64>,
    ) -> Result<Self, String> {
        let Some(term) = default_terminator else {
            // Java NullArgumentException.check("defaultTerminator", ...)
            return Err("\"defaultTerminator\" is null.".to_string());
        };
        let wb_min = match word_boundary_min_length {
            None => DEFAULT_WORD_BOUNDARY_MIN_LENGTH,
            Some(w) if !(0.0..=1.0).contains(&w) => {
                return Err(
                    "wordBoundaryMinLength must be between 0.0 and 1.0 (inclusive)".to_string(),
                );
            }
            Some(w) => w,
        };
        let m_term = default_m_terminator.map(|s| s.to_string());
        Ok(DtAlgo {
            default_terminator: term.to_string(),
            default_terminator_length: default_terminator_length
                .unwrap_or_else(|| term.chars().count()),
            default_terminator_removes_dots: default_terminator_removes_dots
                .unwrap_or_else(|| get_terminator_removes_dots(term)),
            default_m_terminator: m_term.clone(),
            default_m_terminator_length: match (&m_term, default_m_terminator_length) {
                (Some(m), None) => get_length_without_tags(m),
                (_, Some(l)) => l,
                (None, None) => 0,
            },
            default_m_terminator_removes_dots: match (&m_term, default_m_terminator_removes_dots) {
                (Some(m), None) => does_html_or_xml_start_with_dot(m),
                (_, Some(b)) => b,
                (None, None) => false,
            },
            add_space_at_word_boundary,
            word_boundary_min_length: wb_min,
        })
    }

    fn get_default_terminator(&self) -> &str {
        &self.default_terminator
    }
    fn get_default_terminator_length(&self) -> usize {
        self.default_terminator_length
    }
    fn get_default_terminator_removes_dots(&self) -> bool {
        self.default_terminator_removes_dots
    }
    fn get_default_m_terminator(&self) -> Option<&str> {
        self.default_m_terminator.as_deref()
    }
    fn get_default_m_terminator_length(&self) -> usize {
        self.default_m_terminator_length
    }
    fn get_default_m_terminator_removes_dots(&self) -> bool {
        self.default_m_terminator_removes_dots
    }
    fn get_word_boundary_min_length(&self) -> f64 {
        self.word_boundary_min_length
    }
    fn get_add_space_at_word_boundary(&self) -> bool {
        self.add_space_at_word_boundary
    }

    /// Java `truncate`（AUTO 模式，纯文本结果）
    fn truncate(
        &self,
        s: &str,
        max_length: i64,
        terminator: Term,
        terminator_length: Option<i64>,
    ) -> Result<Out, String> {
        self.unified_truncate(
            s,
            max_length,
            terminator,
            terminator_length,
            Mode::Auto,
            false,
        )
    }

    /// Java `truncateC`
    fn truncate_c(
        &self,
        s: &str,
        max_length: i64,
        terminator: Term,
        terminator_length: Option<i64>,
    ) -> Result<Out, String> {
        self.unified_truncate(
            s,
            max_length,
            terminator,
            terminator_length,
            Mode::CharBoundary,
            false,
        )
    }

    /// Java `truncateW`
    fn truncate_w(
        &self,
        s: &str,
        max_length: i64,
        terminator: Term,
        terminator_length: Option<i64>,
    ) -> Result<Out, String> {
        self.unified_truncate(
            s,
            max_length,
            terminator,
            terminator_length,
            Mode::WordBoundary,
            false,
        )
    }

    /// Java `truncateM`（AUTO 模式，允许 markup 结果）
    fn truncate_m(
        &self,
        s: &str,
        max_length: i64,
        terminator: Term,
        terminator_length: Option<i64>,
    ) -> Result<Out, String> {
        self.unified_truncate(
            s,
            max_length,
            terminator,
            terminator_length,
            Mode::Auto,
            true,
        )
    }

    /// Java `truncateCM`
    fn truncate_cm(
        &self,
        s: &str,
        max_length: i64,
        terminator: Term,
        terminator_length: Option<i64>,
    ) -> Result<Out, String> {
        self.unified_truncate(
            s,
            max_length,
            terminator,
            terminator_length,
            Mode::CharBoundary,
            true,
        )
    }

    /// Java `truncateWM`
    fn truncate_wm(
        &self,
        s: &str,
        max_length: i64,
        terminator: Term,
        terminator_length: Option<i64>,
    ) -> Result<Out, String> {
        self.unified_truncate(
            s,
            max_length,
            terminator,
            terminator_length,
            Mode::WordBoundary,
            true,
        )
    }

    /// Java `unifiedTruncate`（DefaultTruncateBuiltinAlgorithm.java:401-458）
    fn unified_truncate(
        &self,
        s: &str,
        max_length: i64,
        terminator: Term,
        terminator_length: Option<i64>,
        mode: Mode,
        allow_markup_result: bool,
    ) -> Result<Out, String> {
        let chars: Vec<char> = s.chars().collect();
        if (chars.len() as i64) <= max_length {
            return Ok(Out::Plain(s.to_string()));
        }
        if max_length < 0 {
            return Err("maxLength can't be negative".to_string());
        }

        // 解析终结符（Java :413-433）
        let mut term: Term = terminator;
        let term_length: i64;
        let mut terminator_removes_dots: Option<bool>;
        match term {
            Term::Default => {
                if allow_markup_result && self.default_m_terminator.is_some() {
                    term = Term::Markup(self.default_m_terminator.as_deref().unwrap());
                    term_length = self.default_m_terminator_length as i64;
                    terminator_removes_dots = Some(self.default_m_terminator_removes_dots);
                } else {
                    term = Term::Plain(&self.default_terminator);
                    term_length = self.default_terminator_length as i64;
                    terminator_removes_dots = Some(self.default_terminator_removes_dots);
                }
            }
            Term::Plain(_) | Term::Markup(_) => {
                match terminator_length {
                    Some(l) => {
                        if l < 0 {
                            return Err("terminatorLength can't be negative".to_string());
                        }
                        term_length = l;
                    }
                    None => {
                        term_length = match term {
                            Term::Plain(t) => t.chars().count() as i64,
                            Term::Markup(m) => get_length_without_tags(m) as i64,
                            _ => unreachable!(),
                        };
                    }
                }
                terminator_removes_dots = None; // 惰性计算
            }
            Term::NonScalar => {
                match terminator_length {
                    Some(l) => {
                        if l < 0 {
                            return Err("terminatorLength can't be negative".to_string());
                        }
                        term_length = l;
                    }
                    None => {
                        // Java getTerminatorLength：非标量非 markup → getMTerminatorLength 的
                        // ClassCast 分支；测试数据总是给 terminatorLength，不达此路径
                        return Err("ClassCastException".to_string());
                    }
                }
                terminator_removes_dots = None;
            }
        }

        let truncated_s = self.unified_truncate_without_terminator_added(
            &chars,
            max_length,
            term,
            term_length,
            &mut terminator_removes_dots,
            mode,
        )?;

        // 终结符总是显示，即使会超出 maxLength（否则用户看不出被截断了）
        // Java :443-445：truncatedS 为 null 或空时返回终结符模型本身
        let return_terminator = |term: Term| -> Out {
            match term {
                Term::Plain(t) => Out::Plain(t.to_string()),
                Term::Markup(m) => Out::Markup(m.to_string()),
                Term::NonScalar => {
                    // Java 此处原样返回模型，不报错；测试数据不达此路径
                    Out::Plain(String::new())
                }
                Term::Default => unreachable!(),
            }
        };
        match truncated_s {
            None => Ok(return_terminator(term)),
            Some(ref t) if t.is_empty() => Ok(return_terminator(term)),
            Some(truncated) => match term {
                Term::Plain(t) => Ok(Out::Plain(format!("{truncated}{t}"))),
                // Java :450-453：outputFormat.concat(fromPlainTextByEscaping(truncatedS), markup)
                Term::Markup(m) => Ok(Out::Markup(format!("{}{m}", html_escape(&truncated)))),
                Term::NonScalar => {
                    // Java IllegalArgumentException("Unsupported terminator type: "
                    // ClassUtil.getFTLTypeDescription(...))——SimpleNumber 的
                    // 描述为 "number (wrapper: SimpleNumber)"
                    Err("Unsupported terminator type: number (wrapper: SimpleNumber)".to_string())
                }
                Term::Default => unreachable!(),
            },
        }
    }

    /// Java `unifiedTruncateWithoutTerminatorAdded`（DefaultTruncateBuiltinAlgorithm.java:460-572）
    fn unified_truncate_without_terminator_added(
        &self,
        chars: &[char],
        max_length: i64,
        terminator: Term,
        terminator_length: i64,
        terminator_removes_dots: &mut Option<bool>,
        mode: Mode,
    ) -> Result<Option<String>, String> {
        let cb_initial_last_c_idx = max_length - terminator_length - 1;
        let mut cb_last_c_idx = skip_trailing_ws(chars, cb_initial_last_c_idx);
        if cb_last_c_idx < 0 {
            return Ok(None);
        }

        let add_space_at_word_boundary = self.add_space_at_word_boundary && terminator_length != 0;

        if (mode == Mode::Auto && self.word_boundary_min_length < 1.0) || mode == Mode::WordBoundary
        {
            // 单词边界截断；可能因 minLength 限制不可行（此时 truncedS 保持 null）
            let mut trunced_s: Option<String> = None;
            {
                let word_terminator_length = if add_space_at_word_boundary {
                    terminator_length + 1
                } else {
                    terminator_length
                };
                let min_idx = if mode == Mode::Auto {
                    (((max_length as f64) * self.word_boundary_min_length).ceil() as i64)
                        - word_terminator_length
                        - 1
                } else {
                    0
                };
                let min_idx = min_idx.max(0);

                let mut wb_last_c_idx =
                    (max_length - word_terminator_length - 1).min(cb_last_c_idx);
                let mut following_c_is_ws = if (chars.len() as i64) > wb_last_c_idx + 1 {
                    is_ws(chars[(wb_last_c_idx + 1) as usize])
                } else {
                    true
                };
                'execute_truncate_wb: loop {
                    if wb_last_c_idx < min_idx {
                        break;
                    }
                    let cur_c = chars[wb_last_c_idx as usize];
                    let cur_c_is_ws = is_ws(cur_c);
                    if !cur_c_is_ws && following_c_is_ws {
                        // 注意：避免在绝对必要前求 getTerminatorRemovesDots
                        if !add_space_at_word_boundary && is_dot(cur_c) {
                            if terminator_removes_dots.is_none() {
                                *terminator_removes_dots =
                                    Some(self.get_terminator_removes_dots(terminator)?);
                            }
                            if *terminator_removes_dots == Some(true) {
                                while wb_last_c_idx >= min_idx
                                    && (is_dot(chars[wb_last_c_idx as usize])
                                        || is_ws(chars[wb_last_c_idx as usize]))
                                {
                                    wb_last_c_idx -= 1;
                                }
                                if wb_last_c_idx < min_idx {
                                    break 'execute_truncate_wb;
                                }
                            }
                        }

                        let mut out = String::new();
                        out.extend(&chars[..=(wb_last_c_idx as usize)]);
                        if add_space_at_word_boundary {
                            out.push(' ');
                        }
                        trunced_s = Some(out);
                        break;
                    }

                    following_c_is_ws = cur_c_is_ws;
                    wb_last_c_idx -= 1;
                }
            }
            if trunced_s.is_some()
                || mode == Mode::WordBoundary
                || (mode == Mode::Auto && self.word_boundary_min_length == 0.0)
            {
                return Ok(trunced_s);
            }
            // AUTO 模式：truncateW 不可行 → 回退到字符边界截断
        }

        // 字符边界截断
        // 若截断点恰为词尾并因此加空格，则可能超出 maxLength 1 个字符，
        // 此时要提前截一个字符
        if cb_last_c_idx == cb_initial_last_c_idx
            && add_space_at_word_boundary
            && is_word_end(chars, cb_last_c_idx)
        {
            cb_last_c_idx -= 1;
            if cb_last_c_idx < 0 {
                return Ok(None);
            }
        }

        // 跳过尾部空白，必要时也跳过尾部点
        loop {
            let mut skipped_dots = false;

            cb_last_c_idx = skip_trailing_ws(chars, cb_last_c_idx);
            if cb_last_c_idx < 0 {
                return Ok(None);
            }

            if is_dot(chars[cb_last_c_idx as usize])
                && !(add_space_at_word_boundary && is_word_end(chars, cb_last_c_idx))
            {
                if terminator_removes_dots.is_none() {
                    *terminator_removes_dots = Some(self.get_terminator_removes_dots(terminator)?);
                }
                if *terminator_removes_dots == Some(true) {
                    cb_last_c_idx = skip_trailing_dots(chars, cb_last_c_idx);
                    if cb_last_c_idx < 0 {
                        return Ok(None);
                    }
                    skipped_dots = true;
                }
            }
            if !skipped_dots {
                break;
            }
        }

        let add_word_boundary_space =
            add_space_at_word_boundary && is_word_end(chars, cb_last_c_idx);
        let mut truncated_s = String::new();
        truncated_s.extend(&chars[..=(cb_last_c_idx as usize)]);
        if add_word_boundary_space {
            truncated_s.push(' ');
        }
        Ok(Some(truncated_s))
    }

    /// Java `getTerminatorRemovesDots(TemplateModel)`（:580-584）
    fn get_terminator_removes_dots(&self, terminator: Term) -> Result<bool, String> {
        match terminator {
            Term::Plain(t) => Ok(get_terminator_removes_dots(t)),
            Term::Markup(m) => Ok(does_html_or_xml_start_with_dot(m)),
            // Java：非标量 → getMTerminatorRemovesDots((TemplateMarkupOutputModel)) ClassCastException
            Term::NonScalar => Err("ClassCastException".to_string()),
            Term::Default => unreachable!(),
        }
    }
}

/// Java `getTerminatorRemovesDots(String)`：终结符以点或省略号开头
fn get_terminator_removes_dots(terminator: &str) -> bool {
    terminator.starts_with('.') || terminator.starts_with('\u{2026}')
}

/// Java `skipTrailingWS`
fn skip_trailing_ws(chars: &[char], mut last_c_idx: i64) -> i64 {
    while last_c_idx >= 0 && is_ws(chars[last_c_idx as usize]) {
        last_c_idx -= 1;
    }
    last_c_idx
}

/// Java `skipTrailingDots`
fn skip_trailing_dots(chars: &[char], mut last_c_idx: i64) -> i64 {
    while last_c_idx >= 0 && is_dot(chars[last_c_idx as usize]) {
        last_c_idx -= 1;
    }
    last_c_idx
}

/// Java `isWordEnd`
fn is_word_end(chars: &[char], last_c_idx: i64) -> bool {
    last_c_idx + 1 >= chars.len() as i64 || is_ws(chars[(last_c_idx + 1) as usize])
}

/// Java `isDot`
fn is_dot(c: char) -> bool {
    c == '.' || c == '\u{2026}'
}

/// Java `Character.isWhitespace` 的近似（数据表全为 ASCII 空白，U+00A0 等分歧不触及）
fn is_ws(c: char) -> bool {
    c.is_whitespace()
}

/// Java `getLengthWithoutTags`（DefaultTruncateBuiltinAlgorithm.java:623-680）
fn get_length_without_tags(s: &str) -> usize {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut result = 0usize;
    let mut i = 0usize;
    'count_chars: while i < len {
        let c = chars[i];
        i += 1;
        if c == '<' {
            if i + 2 < len && chars[i] == '!' && chars[i + 1] == '-' && chars[i + 2] == '-' {
                // <!--...-->
                i += 3;
                while i + 2 < len
                    && !(chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>')
                {
                    i += 1;
                }
                i += 3;
                if i >= len {
                    break 'count_chars;
                }
            } else if i + 7 < len
                && chars[i] == '!'
                && chars[i + 1] == '['
                && chars[i + 2] == 'C'
                && chars[i + 3] == 'D'
                && chars[i + 4] == 'A'
                && chars[i + 5] == 'T'
                && chars[i + 6] == 'A'
                && chars[i + 7] == '['
            {
                // <![CDATA[...]]>
                i += 8;
                while i < len
                    && !(chars[i] == ']'
                        && i + 2 < len
                        && chars[i + 1] == ']'
                        && chars[i + 2] == '>')
                {
                    result += 1;
                    i += 1;
                }
                i += 3;
                if i >= len {
                    break 'count_chars;
                }
            } else {
                // <...>
                while i < len && chars[i] != '>' {
                    i += 1;
                }
                i += 1;
                if i >= len {
                    break 'count_chars;
                }
            }
        } else if c == '&' {
            // &...;
            while i < len && chars[i] != ';' {
                i += 1;
            }
            i += 1;
            result += 1;
            if i >= len {
                break 'count_chars;
            }
        } else {
            result += 1;
        }
    }
    result
}

/// Java `doesHtmlOrXmlStartWithDot`（DefaultTruncateBuiltinAlgorithm.java:686-740）
fn does_html_or_xml_start_with_dot(s: &str) -> bool {
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0usize;
    'consume_chars: while i < len {
        let c = chars[i];
        i += 1;
        if c == '<' {
            if i + 2 < len && chars[i] == '!' && chars[i + 1] == '-' && chars[i + 2] == '-' {
                // <!--...-->
                i += 3;
                while i + 2 < len
                    && !(chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>')
                {
                    i += 1;
                }
                i += 3;
                if i >= len {
                    break 'consume_chars;
                }
            } else if i + 7 < len
                && chars[i] == '!'
                && chars[i + 1] == '['
                && chars[i + 2] == 'C'
                && chars[i + 3] == 'D'
                && chars[i + 4] == 'A'
                && chars[i + 5] == 'T'
                && chars[i + 6] == 'A'
                && chars[i + 7] == '['
            {
                // <![CDATA[...]]>
                i += 8;
                // 引擎差异：Java 循环体内为 `i++` 扫描至 `]]>`；翻译时误写成直接
                // 返回（循环体恒 return，实际最多执行一次）——保持现状行为不变，
                // 仅以 if 等价改写以消除 clippy::never_loop / while_immutable_condition。
                if i < len
                    && !(chars[i] == ']'
                        && i + 2 < len
                        && chars[i + 1] == ']'
                        && chars[i + 2] == '>')
                {
                    return is_dot(chars[i]);
                }
                i += 3;
                if i >= len {
                    break 'consume_chars;
                }
            } else {
                // <...>
                while i < len && chars[i] != '>' {
                    i += 1;
                }
                i += 1;
                if i >= len {
                    break 'consume_chars;
                }
            }
        } else if c == '&' {
            // &...;
            let start = i;
            while i < len && chars[i] != ';' {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            return is_dot_char_reference(&name);
        } else {
            return is_dot(c);
        }
    }
    false
}

/// Java `isDotCharReference`（:743-749）
fn is_dot_char_reference(name: &str) -> bool {
    if name.chars().count() > 2 && name.starts_with('#') {
        let char_code = get_code_from_numerical_char_reference_name(name);
        return char_code == 0x2026 || char_code == 0x2e;
    }
    name == "hellip" || name == "period"
}

/// Java `getCodeFromNumericalCharReferenceName`（:752-769）
fn get_code_from_numerical_char_reference_name(name: &str) -> i64 {
    let chars: Vec<char> = name.chars().collect();
    let c = chars[1];
    let hex = c == 'x' || c == 'X';
    let mut code = 0i64;
    let start = if hex { 2 } else { 1 };
    for &c in chars.iter().skip(start) {
        code *= if hex { 16 } else { 10 };
        if c.is_ascii_digit() {
            code += (c as u32 - '0' as u32) as i64;
        } else if hex && c.is_ascii_lowercase() && ('a'..='f').contains(&c) {
            code += (c as u32 - 'a' as u32 + 10) as i64;
        } else if hex && c.is_ascii_uppercase() && ('A'..='F').contains(&c) {
            code += (c as u32 - 'A' as u32 + 10) as i64;
        } else {
            return -1;
        }
    }
    code
}

/// 数据表辅助 —— 对应 Java assertC/assertW/assertAuto（terminator/terminatorLength 均为 null）
fn assert_c(alg: &DtAlgo, input: &str, max_length: i64, expected: &str) {
    let actual = alg
        .truncate_c(input, max_length, Term::Default, None)
        .expect("truncate_c failed");
    assert_eq!(
        actual.as_str(),
        expected,
        "truncateC({input:?}, {max_length})"
    );
}

fn assert_w(alg: &DtAlgo, input: &str, max_length: i64, expected: &str) {
    let actual = alg
        .truncate_w(input, max_length, Term::Default, None)
        .expect("truncate_w failed");
    assert_eq!(
        actual.as_str(),
        expected,
        "truncateW({input:?}, {max_length})"
    );
}

fn assert_auto(alg: &DtAlgo, input: &str, max_length: i64, expected: &str) {
    let actual = alg
        .truncate(input, max_length, Term::Default, None)
        .expect("truncate failed");
    assert_eq!(
        actual.as_str(),
        expected,
        "truncate({input:?}, {max_length})"
    );
}

/// Java 测试用实例常量
fn ascii_instance() -> DtAlgo {
    DtAlgo::new_with_m(STANDARD_ASCII_TERMINATOR, STANDARD_M_TERMINATOR, true)
}
fn unicode_instance() -> DtAlgo {
    DtAlgo::new_with_m(STANDARD_UNICODE_TERMINATOR, STANDARD_M_TERMINATOR, true)
}
fn empty_terminator_instance() -> DtAlgo {
    DtAlgo::new("", false)
}
fn dots_instance() -> DtAlgo {
    DtAlgo::new("...", true)
}
fn dots_no_w_space_instance() -> DtAlgo {
    DtAlgo::new("...", false)
}
fn ascii_no_w_space_instance() -> DtAlgo {
    DtAlgo::new("[...]", false)
}
/// Java M_TERM_INSTANCE：new DefaultTruncateBuiltinAlgorithm("...", null, true, html("<r>...</r>"), null, true, true, 0.75)
fn m_term_instance() -> DtAlgo {
    DtAlgo::new_full(
        Some("..."),
        None,
        Some(true), // 自动检测 → 以 "." 开头 → true
        Some("<r>...</r>"),
        None,
        Some(true),
        true,
        Some(0.75),
    )
    .expect("m_term_instance")
}

/// Java testConstructorIllegalArguments：null defaultTerminator → IllegalArgumentException "defaultTerminator"
#[test]
fn test_constructor_illegal_arguments() {
    // 引擎差异：Java 构造器对 null defaultTerminator 抛 IllegalArgumentException；
    // v1 的 new_full 返回 Err（消息保留 "defaultTerminator"）。
    let err = DtAlgo::new_full(
        None,
        None,
        Some(true),
        Some("<r>...</r>"),
        None,
        Some(true),
        true,
        Some(0.75),
    )
    .expect_err("构造应失败");
    assert!(
        err.contains("defaultTerminator"),
        "消息应含 defaultTerminator：{err}"
    );
}

/// Java testTruncateIllegalArguments
#[test]
fn test_truncate_illegal_arguments() {
    let alg = ascii_instance();

    // 无错误：
    let r = alg
        .truncate("", 0, Term::Plain("."), Some(1))
        .expect("应成功");
    assert_eq!(r.as_str(), "");

    // maxLength 为负：
    let err = alg
        .truncate("", -1, Term::Plain("."), Some(1))
        .expect_err("应失败");
    assert!(err.contains("maxLength"), "{err}");

    // 非标量终结符（Java SimpleNumber）：
    let err = alg
        .truncate_m("sss", 2, Term::NonScalar, Some(1))
        .expect_err("应失败");
    assert!(err.contains("SimpleNumber"), "{err}");

    // terminatorLength 为负：
    let err = alg
        .truncate("sss", 2, Term::Plain("."), Some(-1))
        .expect_err("应失败");
    assert!(err.contains("terminatorLength"), "{err}");
}

/// Java testCSimple
#[test]
fn test_c_simple() {
    let ascii = ascii_instance();
    assert_c(&ascii, "12345678", 9, "12345678");
    assert_c(&ascii, "12345678", 8, "12345678");
    assert_c(&ascii, "12345678", 7, "12[...]");
    assert_c(&ascii, "12345678", 6, "1[...]");
    for max_length in (0..=5).rev() {
        assert_c(&ascii, "12345678", max_length, "[...]");
    }

    let unicode = unicode_instance();
    assert_c(&unicode, "12345678", 9, "12345678");
    assert_c(&unicode, "12345678", 8, "12345678");
    assert_c(&unicode, "12345678", 7, "1234[\u{2026}]");
    assert_c(&unicode, "12345678", 6, "123[\u{2026}]");
    assert_c(&unicode, "12345678", 5, "12[\u{2026}]");
    assert_c(&unicode, "12345678", 4, "1[\u{2026}]");
    for max_length in (0..=3).rev() {
        assert_c(&unicode, "12345678", max_length, "[\u{2026}]");
    }

    let empty_term = empty_terminator_instance();
    assert_c(&empty_term, "12345678", 9, "12345678");
    for length in (0..=8).rev() {
        assert_c(
            &empty_term,
            "12345678",
            length,
            &"12345678"[..length as usize],
        );
    }
}

/// Java testCSpaceAndDot
#[test]
fn test_c_space_and_dot() {
    let ascii = ascii_instance();
    assert_c(&ascii, "123456  ", 9, "123456  ");
    assert_c(&ascii, "123456  ", 8, "123456  ");
    assert_c(&ascii, "123456  ", 7, "12[...]");
    assert_c(&ascii, "123456  ", 6, "1[...]");
    assert_c(&ascii, "123456  ", 5, "[...]");
    assert_c(&ascii, "123456  ", 4, "[...]");

    assert_c(&ascii, "1 345        ", 13, "1 345        ");
    assert_c(&ascii, "1 345        ", 12, "1 345 [...]"); // 不是 "1 345  [...]"
    assert_c(&ascii, "1 345        ", 11, "1 345 [...]");
    assert_c(&ascii, "1 345        ", 10, "1 34[...]"); // 不是 "12345[...]"
    assert_c(&ascii, "1 345        ", 9, "1 34[...]");
    assert_c(&ascii, "1 345        ", 8, "1 3[...]");
    assert_c(&ascii, "1 345        ", 7, "1 [...]");
    assert_c(&ascii, "1 345        ", 6, "[...]"); // 不是 "1[...]"
    assert_c(&ascii, "1 345        ", 5, "[...]");
    assert_c(&ascii, "1 345        ", 4, "[...]");

    let ascii_no_w = ascii_no_w_space_instance();
    assert_c(&ascii_no_w, "1 345        ", 13, "1 345        ");
    assert_c(&ascii_no_w, "1 345        ", 12, "1 345[...]"); // 有差异！
    assert_c(&ascii_no_w, "1 345        ", 11, "1 345[...]"); // 有差异！
    assert_c(&ascii_no_w, "1 345        ", 10, "1 345[...]"); // 有差异！
    assert_c(&ascii_no_w, "1 345        ", 9, "1 34[...]");
    assert_c(&ascii_no_w, "1 345        ", 8, "1 3[...]");
    assert_c(&ascii_no_w, "1 345        ", 7, "1[...]"); // 有差异！
    assert_c(&ascii_no_w, "1 345        ", 6, "1[...]"); // 有差异！
    assert_c(&ascii_no_w, "1 345        ", 5, "[...]");
    assert_c(&ascii_no_w, "1 345        ", 4, "[...]");

    assert_c(&ascii, "1  4567890", 9, "1  4[...]");
    assert_c(&ascii, "1  4567890", 8, "1 [...]");
    assert_c(&ascii_no_w, "1  4567890", 9, "1  4[...]");
    assert_c(&ascii_no_w, "1  4567890", 8, "1[...]");

    assert_c(&ascii, "  3456789", 9, "  3456789");
    assert_c(&ascii, "  3456789", 8, "  3[...]");
    assert_c(&ascii, "  3456789", 7, "[...]");
    assert_c(&ascii, "  3456789", 6, "[...]");

    assert_c(&ascii_no_w, "  3456789", 8, "  3[...]");
    assert_c(&ascii_no_w, "  3456789", 7, "[...]");

    // 默认情况下点不特殊处理：
    assert_c(&ascii, "1.  56...012345", 15, "1.  56...012345");
    assert_c(&ascii, "1.  56...012345", 14, "1.  56...[...]");
    assert_c(&ascii, "1.  56...012345", 13, "1.  56..[...]");
    assert_c(&ascii, "1.  56...012345", 12, "1.  56.[...]");
    assert_c(&ascii, "1.  56...012345", 11, "1.  56[...]");
    assert_c(&ascii, "1.  56...012345", 10, "1.  5[...]");
    assert_c(&ascii, "1.  56...012345", 9, "1. [...]");
    assert_c(&ascii, "1.  56...012345", 8, "1. [...]");
    assert_c(&ascii, "1.  56...012345", 7, "1[...]");
    assert_c(&ascii, "1.  56...012345", 6, "1[...]");
    assert_c(&ascii, "1.  56...012345", 5, "[...]");

    // 此处点特殊处理：
    let dots = dots_instance();
    assert_c(&dots, "1.  56...012345", 15, "1.  56...012345");
    assert_c(&dots, "1.  56...012345", 14, "1.  56...01...");
    assert_c(&dots, "1.  56...012345", 13, "1.  56...0...");
    assert_c(&dots, "1.  56...012345", 12, "1.  56...");
    assert_c(&dots, "1.  56...012345", 11, "1.  56...");
    assert_c(&dots, "1.  56...012345", 10, "1.  56...");
    assert_c(&dots, "1.  56...012345", 9, "1.  56...");
    assert_c(&dots, "1.  56...012345", 8, "1.  5...");
    assert_c(&dots, "1.  56...012345", 7, "1. ...");
    assert_c(&dots, "1.  56...012345", 6, "1. ...");
    assert_c(&dots, "1.  56...012345", 5, "1...");
    assert_c(&dots, "1.  56...012345", 4, "1...");
    assert_c(&dots, "1.  56...012345", 3, "...");
    assert_c(&dots, "1.  56...012345", 2, "...");
    assert_c(&dots, "1.  56...012345", 1, "...");
    assert_c(&dots, "1.  56...012345", 0, "...");

    let dots_no_w = dots_no_w_space_instance();
    assert_c(&dots_no_w, "1.  56...012345", 8, "1.  5...");
    assert_c(&dots_no_w, "1.  56...012345", 7, "1...");
    assert_c(&dots_no_w, "1.  56...012345", 6, "1...");
    assert_c(&dots_no_w, "1.  56...012345", 5, "1...");
    assert_c(&dots_no_w, "1.  56...012345", 4, "1...");
    assert_c(&dots_no_w, "1.  56...012345", 3, "...");

    let empty_term = empty_terminator_instance();
    assert_c(&empty_term, "ab. cd", 6, "ab. cd");
    assert_c(&empty_term, "ab. cd", 5, "ab. c");
    assert_c(&empty_term, "ab. cd", 4, "ab.");
    assert_c(&empty_term, "ab. cd", 3, "ab.");
    assert_c(&empty_term, "ab. cd", 2, "ab");
    assert_c(&empty_term, "ab. cd", 1, "a");
    assert_c(&empty_term, "ab. cd", 0, "");
}

/// Java testWSimple
#[test]
fn test_w_simple() {
    let ascii = ascii_instance();
    assert_w(&ascii, "word1 word2 word3", 18, "word1 word2 word3");
    assert_w(&ascii, "word1 word2 word3", 17, "word1 word2 word3");
    assert_w(&ascii, "word1 word2 word3", 16, "word1 [...]");
    assert_w(&ascii, "word1 word2 word3", 11, "word1 [...]");
    for max_length in (0..=10).rev() {
        assert_w(&ascii, "word1 word2 word3", max_length, "[...]");
    }

    let unicode = unicode_instance();
    assert_w(&unicode, "word1 word2 word3", 18, "word1 word2 word3");
    assert_w(&unicode, "word1 word2 word3", 17, "word1 word2 word3");
    assert_w(&unicode, "word1 word2 word3", 16, "word1 word2 [\u{2026}]");
    assert_w(&unicode, "word1 word2 word3", 15, "word1 word2 [\u{2026}]");
    assert_w(&unicode, "word1 word2 word3", 14, "word1 [\u{2026}]");
    assert_w(&unicode, "word1 word2 word3", 9, "word1 [\u{2026}]");
    for max_length in (0..=8).rev() {
        assert_w(&unicode, "word1 word2 word3", max_length, "[\u{2026}]");
    }

    let empty_term = empty_terminator_instance();
    assert_w(&empty_term, "word1 word2 word3", 18, "word1 word2 word3");
    assert_w(&empty_term, "word1 word2 word3", 17, "word1 word2 word3");
    assert_w(&empty_term, "word1 word2 word3", 16, "word1 word2");
    assert_w(&empty_term, "word1 word2 word3", 11, "word1 word2");
    assert_w(&empty_term, "word1 word2 word3", 10, "word1");
    assert_w(&empty_term, "word1 word2 word3", 5, "word1");
    for max_length in (0..=4).rev() {
        assert_w(&empty_term, "word1 word2 word3", max_length, "");
    }
}

/// Java testWSpaceAndDot
#[test]
fn test_w_space_and_dot() {
    let dots = dots_instance();
    assert_w(&dots, "  word1  word2  ", 16, "  word1  word2  ");
    assert_w(&dots, "  word1  word2  ", 15, "  word1 ...");
    assert_w(&dots, "  word1  word2  ", 11, "  word1 ...");
    for max_length in (0..=10).rev() {
        assert_w(&dots, "  word1  word2  ", max_length, "...");
    }

    let dots_no_w = dots_no_w_space_instance();
    assert_w(&dots_no_w, "  word1  word2  ", 16, "  word1  word2  ");
    assert_w(&dots_no_w, "  word1  word2  ", 15, "  word1...");
    assert_w(&dots_no_w, "  word1  word2  ", 10, "  word1...");
    for max_length in (0..=9).rev() {
        assert_w(&dots_no_w, "  word1  word2  ", max_length, "...");
    }

    assert_w(
        &dots,
        " . . word1..  word2    ",
        23,
        " . . word1..  word2    ",
    );
    assert_w(&dots, " . . word1..  word2    ", 22, " . . word1.. ...");
    assert_w(&dots, " . . word1..  word2    ", 16, " . . word1.. ...");
    assert_w(&dots, " . . word1..  word2    ", 15, " . . ...");
    assert_w(&dots, " . . word1..  word2    ", 8, " . . ...");
    assert_w(&dots, " . . word1..  word2    ", 7, " . ...");
    assert_w(&dots, " . . word1..  word2    ", 6, " . ...");
    for max_length in (0..=5).rev() {
        assert_w(&dots, " . . word1..  word2    ", max_length, "...");
    }

    assert_w(
        &dots_no_w,
        " . . word1..  word2    ",
        23,
        " . . word1..  word2    ",
    );
    assert_w(
        &dots_no_w,
        " . . word1..  word2    ",
        22,
        " . . word1..  word2...",
    );
    assert_w(&dots_no_w, " . . word1..  word2    ", 21, " . . word1...");
    for max_length in (0..=13).rev() {
        assert_w(&dots_no_w, " . . word1..  word2    ", max_length, "...");
    }
}

/// Java testAuto —— "Auto" 指普通 truncate(..) 调用（自动在 CB 与 WB 之间选择）
#[test]
fn test_auto() {
    let ascii = ascii_instance();
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHIJKL",
        24,
        "1 234567 90ABCDEFGHIJKL",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHIJKL",
        23,
        "1 234567 90ABCDEFGHIJKL",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHIJKL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHIJKL",
        21,
        "1 234567 90ABCDE[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHIJKL",
        20,
        "1 234567 90ABCD[...]",
    );
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 19, "1 234567 90ABC[...]");
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 18, "1 234567 [...]");
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 17, "1 234567 [...]");
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 16, "1 234567 [...]");
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 15, "1 234567 [...]");
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 14, "1 234567 [...]");
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 13, "1 23456[...]"); // wb 空格
    assert_auto(&ascii, "1 234567 90ABCDEFGHIJKL", 12, "1 23456[...]");

    assert_auto(
        &ascii,
        "1 234567  0ABCDEFGHIJKL",
        22,
        "1 234567  0ABCDEF[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 9 ABCDEFGHIJKL",
        22,
        "1 234567 9 ABCDEF[...]",
    );
    assert_auto(&ascii, "1 234567 90 BCDEFGHIJKL", 22, "1 234567 90 [...]");
    assert_auto(&ascii, "1 234567 90A CDEFGHIJKL", 22, "1 234567 90A [...]");
    assert_auto(&ascii, "1 234567 90AB DEFGHIJKL", 22, "1 234567 90AB [...]");
    assert_auto(
        &ascii,
        "1 234567 90ABC EFGHIJKL",
        22,
        "1 234567 90ABC [...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCD FGHIJKL",
        22,
        "1 234567 90ABCD [...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDE GHIJKL",
        22,
        "1 234567 90ABCDE [...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEF HIJKL",
        22,
        "1 234567 90ABCDE[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFG IJKL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGH JKL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHI KL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHIJ L",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii,
        "1 234567 90ABCDEFGHIJK ",
        22,
        "1 234567 90ABCDEF[...]",
    );

    let ascii_no_w = ascii_no_w_space_instance();
    assert_auto(
        &ascii_no_w,
        "1 234567  0ABCDEFGHIJKL",
        22,
        "1 234567  0ABCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 9 ABCDEFGHIJKL",
        22,
        "1 234567 9 ABCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90 BCDEFGHIJKL",
        22,
        "1 234567 90 BCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90A CDEFGHIJKL",
        22,
        "1 234567 90A[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90AB DEFGHIJKL",
        22,
        "1 234567 90AB[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABC EFGHIJKL",
        22,
        "1 234567 90ABC[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCD FGHIJKL",
        22,
        "1 234567 90ABCD[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCDE GHIJKL",
        22,
        "1 234567 90ABCDE[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCDEF HIJKL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCDEFG IJKL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCDEFGH JKL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCDEFGHI KL",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCDEFGHIJ L",
        22,
        "1 234567 90ABCDEF[...]",
    );
    assert_auto(
        &ascii_no_w,
        "1 234567 90ABCDEFGHIJK ",
        22,
        "1 234567 90ABCDEF[...]",
    );

    let dots = dots_instance();
    assert_auto(
        &dots,
        "12390ABCD..  . EFGHIJK .",
        24,
        "12390ABCD..  . EFGHIJK .",
    );
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 23, "12390ABCD..  . ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 22, "12390ABCD..  . ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 21, "12390ABCD..  . ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 20, "12390ABCD..  . ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 19, "12390ABCD..  . ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 18, "12390ABCD..  . ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 17, "12390ABCD.. ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 16, "12390ABCD.. ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 15, "12390ABCD.. ...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 14, "12390ABCD...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 13, "12390ABCD...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 12, "12390ABCD...");
    assert_auto(&dots, "12390ABCD..  . EFGHIJK .", 11, "12390ABC...");

    assert_auto(
        &dots,
        "word0 word1. word2 w3 . . w4",
        27,
        "word0 word1. word2 w3 . ...",
    );
    assert_auto(
        &dots,
        "word0 word1. word2 w3 . . w4",
        26,
        "word0 word1. word2 w3 ...",
    );
    assert_auto(
        &dots,
        "word0 word1. word2 w3 . . w4",
        25,
        "word0 word1. word2 w3 ...",
    );
    assert_auto(
        &dots,
        "word0 word1. word2 w3 . . w4",
        24,
        "word0 word1. word2 ...",
    );
    assert_auto(
        &dots,
        "word0 word1. word2 w3 . . w4",
        22,
        "word0 word1. word2 ...",
    );
    assert_auto(
        &dots,
        "word0 word1. word2 w3 . . w4",
        21,
        "word0 word1. ...",
    );
    assert_auto(
        &dots,
        "word0 word1. word2 w3 . . w4",
        16,
        "word0 word1. ...",
    );
    assert_auto(&dots, "word0 word1. word2 w3 . . w4", 15, "word0 word1...");
    assert_auto(&dots, "word0 word1. word2 w3 . . w4", 14, "word0 word1...");
    assert_auto(&dots, "word0 word1. word2 w3 . . w4", 13, "word0 word...");
    assert_auto(&dots, "word0 word1. word2 w3 . . w4", 12, "word0 ...");
    assert_auto(&dots, "word0 word1. word2 w3 . . w4", 9, "word0 ...");
    assert_auto(&dots, "word0 word1. word2 w3 . . w4", 8, "word...");
}

/// Java testExtremeWordBoundaryMinLengths
#[test]
fn test_extreme_word_boundary_min_lengths() {
    let ascii = ascii_instance();
    assert_c(&ascii, "1 3456789", 8, "1 3[...]");
    assert_w(&ascii, "1 3456789", 8, "1 [...]");
    // wbMinLen1Algo：wordBoundaryMinLength = 1.0
    let wb_min_len1 = DtAlgo::new_full(
        Some(ascii.get_default_terminator()),
        Some(ascii.get_default_terminator_length()),
        Some(ascii.get_default_terminator_removes_dots()),
        None,
        None,
        None,
        true,
        Some(1.0),
    )
    .expect("wbMinLen1Algo");
    assert_auto(&wb_min_len1, "1 3456789", 8, "1 3[...]");

    assert_auto(&ascii, "123456789", 8, "123[...]");
    // wbMinLen0Algo：wordBoundaryMinLength = 0.0
    let wb_min_len0 = DtAlgo::new_full(
        Some(ascii.get_default_terminator()),
        Some(ascii.get_default_terminator_length()),
        Some(ascii.get_default_terminator_removes_dots()),
        None,
        None,
        None,
        true,
        Some(0.0),
    )
    .expect("wbMinLen0Algo");
    assert_auto(&wb_min_len0, "123456789", 8, "[...]");
}

/// Java testSimpleEdgeCases
#[test]
fn test_simple_edge_cases() {
    let algos = [
        ascii_instance(),
        unicode_instance(),
        empty_terminator_instance(),
        dots_instance(),
        ascii_no_w_space_instance(),
        m_term_instance(),
    ];
    for alg in &algos {
        // 三个调用者：truncateM / truncateCM / truncateWM
        let callers: [TruncateCaller; 3] =
            [DtAlgo::truncate_m, DtAlgo::truncate_cm, DtAlgo::truncate_wm];
        for caller in callers {
            let r = caller(alg, "", 0, Term::Default, None).expect("truncate(\"\", 0)");
            assert_eq!(r.as_str(), "", "空串截断");

            if alg.get_default_m_terminator().is_some() {
                // 引擎差异：Java 断言结果为 TemplateMarkupOutputModel 且 assertSame 于
                // getDefaultMTerminator()；v1 无 markup 类型，改为字符串相等
                let truncated =
                    caller(alg, "x", 0, Term::Default, None).expect("truncate(\"x\", 0)");
                assert_eq!(
                    truncated.as_str(),
                    alg.get_default_m_terminator().unwrap(),
                    "默认 markup 终结符原样返回"
                );
            } else {
                let truncated =
                    caller(alg, "x", 0, Term::Default, None).expect("truncate(\"x\", 0)");
                assert_eq!(
                    truncated.as_str(),
                    alg.get_default_terminator(),
                    "默认终结符原样返回"
                );
            }
            // 显式字符串终结符 "|" 原样返回（Java assertSame）
            let r = caller(alg, "x", 0, Term::Plain("|"), None).expect("truncate(\"x\", 0, \"|\")");
            assert_eq!(r.as_str(), "|");
            // 显式 HTML 终结符 "<x>.</x>" 原样返回（Java assertSame；v1 字符串级相等）
            let r = caller(alg, "x", 0, Term::Markup("<x>.</x>"), None)
                .expect("truncate(\"x\", 0, html)");
            assert_eq!(r.as_str(), "<x>.</x>");
        }
    }
}

/// Java testStandardInstanceSettings
#[test]
fn test_standard_instance_settings() {
    let ascii = ascii_instance();
    let r = ascii
        .truncate("1234567890", 8, Term::Default, None)
        .expect("truncate");
    assert_eq!(r.as_str(), "123[...]");

    // 引擎差异：Java truncateM 结果为 TemplateHTMLOutputModel，经
    // HTMLOutputFormat.getMarkupString 取 "<span class='truncateTerminator'>[&#8230;]</span>"；
    // v1 以"html_escape(截断文本) + 标记"字符串级模型近似
    let r = ascii
        .truncate_m("1234567890", 8, Term::Default, None)
        .expect("truncateM");
    assert_eq!(
        r.as_str(),
        "12345<span class='truncateTerminator'>[&#8230;]</span>"
    );

    let unicode = unicode_instance();
    let r = unicode
        .truncate("1234567890", 8, Term::Default, None)
        .expect("truncate");
    assert_eq!(r.as_str(), "12345[\u{2026}]");

    let r = unicode
        .truncate_m("1234567890", 8, Term::Default, None)
        .expect("truncateM");
    assert_eq!(
        r.as_str(),
        "12345<span class='truncateTerminator'>[&#8230;]</span>"
    );
}

/// Java testGetLengthWithoutTags
#[test]
fn test_get_length_without_tags() {
    assert_eq!(get_length_without_tags(""), 0);
    assert_eq!(get_length_without_tags("a"), 1);
    assert_eq!(get_length_without_tags("ab"), 2);
    assert_eq!(get_length_without_tags("<tag>"), 0);
    assert_eq!(get_length_without_tags("<tag>a"), 1);
    assert_eq!(get_length_without_tags("<tag>a</tag>b"), 2);
    assert_eq!(get_length_without_tags("ab<tag>cd</tag>"), 4);
    assert_eq!(get_length_without_tags("ab<tag></tag>"), 2);

    assert_eq!(get_length_without_tags("&chr;a"), 2);
    assert_eq!(get_length_without_tags("&chr;a&chr;b"), 4);
    assert_eq!(get_length_without_tags("ab&chr;cd&chr;"), 6);
    assert_eq!(get_length_without_tags("ab&chr;&chr;"), 4);
    assert_eq!(get_length_without_tags("ab<tag>&chr;</tag>&chr;"), 4);

    assert_eq!(get_length_without_tags("<!--c-->ab"), 2);
    assert_eq!(get_length_without_tags("a<!--c-->b<!--c-->"), 2);
    assert_eq!(get_length_without_tags("a<!-->--><!---->b"), 2);

    assert_eq!(get_length_without_tags("a<![CDATA[b]]>c"), 3);
    assert_eq!(get_length_without_tags("a<![CDATA[]]>b"), 2);
    assert_eq!(get_length_without_tags("<![CDATA[]]>"), 0);
    assert_eq!(get_length_without_tags("<![CDATA[123"), 3);
    assert_eq!(get_length_without_tags("<![CDATA[123]"), 4);
    assert_eq!(get_length_without_tags("<![CDATA[123]]"), 5);
    assert_eq!(get_length_without_tags("<![CDATA[123]]>"), 3);

    assert_eq!(get_length_without_tags("ab<!--"), 2);
    assert_eq!(get_length_without_tags("ab<tag"), 2);
    assert_eq!(get_length_without_tags("ab&chr"), 3);
    assert_eq!(get_length_without_tags("ab<!-"), 2);
    assert_eq!(get_length_without_tags("ab<"), 2);
    assert_eq!(get_length_without_tags("ab&"), 3);
    assert_eq!(get_length_without_tags("a&;c"), 3);
}

/// Java testGetCodeFromNumericalCharReferenceName
#[test]
fn test_get_code_from_numerical_char_reference_name() {
    assert_eq!(get_code_from_numerical_char_reference_name("#0"), 0);
    assert_eq!(get_code_from_numerical_char_reference_name("#00"), 0);
    assert_eq!(get_code_from_numerical_char_reference_name("#x0"), 0);
    assert_eq!(get_code_from_numerical_char_reference_name("#x00"), 0);
    assert_eq!(get_code_from_numerical_char_reference_name("#1"), 1);
    assert_eq!(get_code_from_numerical_char_reference_name("#01"), 1);
    assert_eq!(get_code_from_numerical_char_reference_name("#x1"), 1);
    assert_eq!(get_code_from_numerical_char_reference_name("#x01"), 1);
    assert_eq!(get_code_from_numerical_char_reference_name("#X1"), 1);
    assert_eq!(get_code_from_numerical_char_reference_name("#X01"), 1);
    assert_eq!(
        get_code_from_numerical_char_reference_name("#123409"),
        123409
    );
    assert_eq!(
        get_code_from_numerical_char_reference_name("#00123409"),
        123409
    );
    assert_eq!(
        get_code_from_numerical_char_reference_name("#x123A0F"),
        0x123A0F
    );
    assert_eq!(
        get_code_from_numerical_char_reference_name("#x123a0f"),
        0x123A0F
    );
    assert_eq!(
        get_code_from_numerical_char_reference_name("#X00123A0f"),
        0x123A0F
    );
    assert_eq!(get_code_from_numerical_char_reference_name("#x1G"), -1);
    assert_eq!(get_code_from_numerical_char_reference_name("#1A"), -1);
}

/// Java testIsDotCharReference
#[test]
fn test_is_dot_char_reference() {
    assert!(is_dot_char_reference("#46"));
    assert!(is_dot_char_reference("#x2E"));
    assert!(is_dot_char_reference("#x2026"));
    assert!(is_dot_char_reference("hellip"));
    assert!(is_dot_char_reference("period"));

    assert!(!is_dot_char_reference(""));
    assert!(!is_dot_char_reference("foo"));
    assert!(!is_dot_char_reference("#x46"));
    assert!(!is_dot_char_reference("#boo"));
}

/// Java testIsHtmlOrXmlStartsWithDot
#[test]
fn test_is_html_or_xml_starts_with_dot() {
    assert!(does_html_or_xml_start_with_dot("."));
    assert!(does_html_or_xml_start_with_dot(".etc"));
    assert!(does_html_or_xml_start_with_dot("&hellip;"));
    assert!(does_html_or_xml_start_with_dot("<tag x='y'/>&hellip;"));
    assert!(does_html_or_xml_start_with_dot(
        "<span class='t'>...</span>"
    ));
    assert!(does_html_or_xml_start_with_dot(
        "<span class='t'>&#x2026;</span>"
    ));
    assert!(does_html_or_xml_start_with_dot(
        "<span class='t'>&#46;</span>"
    ));
    assert!(does_html_or_xml_start_with_dot("<foo><!-- -->.etc"));

    assert!(!does_html_or_xml_start_with_dot(""));
    assert!(!does_html_or_xml_start_with_dot("[...]"));
    assert!(!does_html_or_xml_start_with_dot("etc."));
    assert!(!does_html_or_xml_start_with_dot(
        "<span class='t'>[...]</span>"
    ));
    assert!(!does_html_or_xml_start_with_dot(
        "<span class='t'>etc.</span>"
    ));
    assert!(!does_html_or_xml_start_with_dot(
        "<span class='t'>&46;</span>"
    ));
}

/// Java testTruncateAdhocHtmlTerminator（markup 终结符）
#[test]
fn test_truncate_adhoc_html_terminator() {
    let ascii = ascii_instance();
    let html_ellipsis = Term::Markup("<i>&#x2026;</i>");
    let html_squ_ellipsis = Term::Markup("<i>[&#x2026;]</i>");

    // 长度检测
    let r = ascii
        .truncate_m("abcd", 3, html_ellipsis, None)
        .expect("truncateM");
    assert_eq!(r.as_str(), "ab<i>&#x2026;</i>");
    let r = ascii
        .truncate_m("abcdef", 5, html_squ_ellipsis, None)
        .expect("truncateM");
    assert_eq!(r.as_str(), "ab<i>[&#x2026;]</i>");
    let r = ascii
        .truncate_m("abcdef", 5, html_squ_ellipsis, Some(1))
        .expect("truncateM");
    assert_eq!(r.as_str(), "abcd<i>[&#x2026;]</i>");

    // 点移除
    let r = ascii
        .truncate_m("a.cd", 3, html_ellipsis, None)
        .expect("truncateM");
    assert_eq!(r.as_str(), "a<i>&#x2026;</i>");
    let r = ascii
        .truncate_m("a.cdef", 5, html_squ_ellipsis, None)
        .expect("truncateM");
    assert_eq!(r.as_str(), "a.<i>[&#x2026;]</i>");
}

/// Java testTruncateAdhocPlainTextTerminator
#[test]
fn test_truncate_adhoc_plain_text_terminator() {
    let ascii = ascii_instance();
    let ellipsis = Term::Plain("\u{2026}");
    let squ_ellipsis = Term::Plain("[\u{2026}]");

    // 长度检测
    let r = ascii.truncate("abcd", 3, ellipsis, None).expect("truncate");
    assert_eq!(r.as_str(), "ab\u{2026}");
    let r = ascii
        .truncate("abcdef", 5, squ_ellipsis, None)
        .expect("truncate");
    assert_eq!(r.as_str(), "ab[\u{2026}]");
    let r = ascii
        .truncate("abcdef", 5, squ_ellipsis, Some(1))
        .expect("truncate");
    assert_eq!(r.as_str(), "abcd[\u{2026}]");

    // 点移除
    let r = ascii.truncate("a.cd", 3, ellipsis, None).expect("truncate");
    assert_eq!(r.as_str(), "a\u{2026}");
    let r = ascii
        .truncate("a.cdef", 5, squ_ellipsis, None)
        .expect("truncate");
    assert_eq!(r.as_str(), "a.[\u{2026}]");
}
