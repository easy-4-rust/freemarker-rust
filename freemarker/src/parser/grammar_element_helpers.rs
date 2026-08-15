//! 元素树遍历与终结节点检查（空白剥离的支撑函数）。
//!
//! 包含：children_mut（子块遍历）、is_trim_element/is_non_outputting（元素类型判定）、
//! leaf_heeds_opening/heeds_opening/heeds_trailing（空白敏感判定）、
//! first_leaf/last_leaf/walk_prev/walk_next（终结节点链遍历）。

use crate::core::{Element, ElementKind, MacroDef};
use std::collections::HashMap;

/// 终结节点信息（空白剥离同行检查用）
#[derive(Clone, Copy)]
pub(super) struct Term {
    /// 是否"care"空白（无换行的文本才 care；Java heedsOpening/TrailingWhitespace）
    pub(super) heeds: bool,
    /// 行号（prev 用结束行，next 用开始行）
    pub(super) line: u32,
}

/// 将树上（已打剥离标记的）宏定义同步回解析期注册表
/// （Java 解析期 Macro 元素与 Template.macros 引用同一 Macro 对象）
pub(super) fn sync_macro_defs(els: &mut [Element], macros: &mut HashMap<String, MacroDef>) {
    for el in els.iter_mut() {
        if let ElementKind::Macro { def } = &el.kind {
            macros.insert(def.name.clone(), def.clone());
        }
        for child in children_mut(el) {
            sync_macro_defs(child, macros);
        }
    }
}

/// 返回元素的直接子块（空白剥离递归用）
pub(super) fn children_mut(el: &mut Element) -> Vec<&mut Vec<Element>> {
    match &mut el.kind {
        ElementKind::If { then, else_, .. } => {
            let mut v = vec![then];
            if let Some(e) = else_ {
                v.push(e);
            }
            v
        }
        ElementKind::List { body, else_, .. } => {
            let mut v = vec![body];
            if let Some(e) = else_ {
                v.push(e);
            }
            v
        }
        ElementKind::Macro { def, .. } => vec![&mut def.body],
        ElementKind::Trim(b)
        | ElementKind::Compress(b)
        | ElementKind::NoEscape(b)
        | ElementKind::AutoEsc(b)
        | ElementKind::NoAutoEsc(b)
        | ElementKind::BlockAssign { body: b, .. }
        | ElementKind::Items { body: b, .. }
        | ElementKind::Sep { body: b }
        | ElementKind::Transform { body: b, .. }
        | ElementKind::On { body: b, .. }
        | ElementKind::Call { body: Some(b), .. }
        | ElementKind::Escape { body: b, .. }
        | ElementKind::OutputFormat { body: b, .. }
        | ElementKind::Nested { body: Some(b), .. }
        | ElementKind::Global { body: Some(b), .. }
        | ElementKind::Local { body: Some(b), .. } => vec![b],
        ElementKind::Attempt { try_, recover, .. } => vec![try_, recover],
        ElementKind::Switch { cases, default, .. } => {
            let mut v = Vec::new();
            for c in cases {
                v.push(&mut c.body);
            }
            if let Some(d) = default {
                v.push(d);
            }
            v
        }
        _ => Vec::new(),
    }
}

/// Trim 指令元素（Java TrimInstruction；ignorable，terminal 遍历跳过）
pub(super) fn is_trim_element(el: &Element) -> bool {
    matches!(
        el.kind,
        ElementKind::TrimLineStart
            | ElementKind::NoTrimLineStart
            | ElementKind::TrimLineEnd
            | ElementKind::LeftTrimLine
    )
}

/// 元素作为叶时的 heedsOpeningWhitespace（Java TextBlock.java:215-226 的
/// heedsOpeningWhitespace；getFirstLeaf 简化：不深入块内部，Macro 视为叶；
/// ignorable 全空白文本 → false，Java TextBlock.java:316-318 —— 由调用方
/// （HEINOUS 扫描）结合兄弟上下文判定）
pub(super) fn leaf_heeds_opening(el: &Element) -> bool {
    match &el.kind {
        ElementKind::Text { text, .. } | ElementKind::NoParse { text, .. } => heeds_opening(text),
        ElementKind::Interpolation { .. } => true,
        _ => false,
    }
}

/// 非输出型元素（Java TextBlock.nonOutputtingType :374-381：
/// Macro/Assignment/AssignmentInstruction/PropertySetting/LibraryLoad/Comment；
/// Global/Local 继承 Assignment）
pub(super) fn is_non_outputting(el: &Element) -> bool {
    matches!(
        el.kind,
        ElementKind::Macro { .. }
            | ElementKind::Assign { .. }
            | ElementKind::Assignments(_)
            | ElementKind::BlockAssign { .. }
            | ElementKind::Global { .. }
            | ElementKind::Local { .. }
            | ElementKind::Setting { .. }
            | ElementKind::Import { .. }
            | ElementKind::Comment { .. }
    )
}

/// Java TextBlock.isIgnorable(stripWhitespace=true)（TextBlock.java:349-366）：
/// 全空白文本 +（顶层边界或两侧均为非输出元素）→ ignorable（heeds=false）。
/// `j` 为文本在 `els` 中的下标；`is_root` 表示本层为模板根。
pub(super) fn is_ignorable_text(els: &[Element], j: usize, is_root: bool) -> bool {
    let (empty, all_ws) = match &els[j].kind {
        ElementKind::Text { text, .. } | ElementKind::NoParse { text, .. } => {
            (text.is_empty(), text.chars().all(is_ws))
        }
        _ => return false,
    };
    // Java isIgnorable（TextBlock.java:349-352）：空文本**无条件** ignorable
    // （剥离后文本可为空 —— 链上后文所见即最终内容）
    if empty {
        return true;
    }
    if !all_ws {
        return false;
    }
    let prev_ok = (j == 0 && is_root) || (j > 0 && is_non_outputting(&els[j - 1]));
    let next_ok =
        (j + 1 == els.len() && is_root) || (j + 1 < els.len() && is_non_outputting(&els[j + 1]));
    prev_ok && next_ok
}

/// 沿同级向前逐叶回溯，返回首个"care 空白"且结束行 == `line` 的叶
/// （Java openingCharsToStrip 的 prevTerminalNode 链，TextBlock.java:280-288）。
/// 链上每个叶都检查：跨行叶 → 链终止（None，照剥）；heeds 叶 → 阻止（Some）；
/// 其余（含 ignorable 全空白文本，Java TextBlock.java:316-318 heeds=false）→ 继续。
/// 列表走尽 → 用父级传入的 prev 兜底（Java 上溯 parent.prevTerminalNode）。
pub(super) fn walk_prev(
    els: &[Element],
    i: usize,
    line: u32,
    parent_prev: Option<Term>,
    is_root: bool,
) -> Option<Term> {
    let mut j = i;
    loop {
        if j == 0 {
            return parent_prev;
        }
        j -= 1;
        let e = &els[j];
        if is_trim_element(e) || is_ignorable_text(els, j, is_root) {
            continue;
        }
        match last_leaf(e) {
            Some(t) => {
                if t.line != line {
                    return None;
                }
                if t.heeds {
                    return Some(t);
                }
            }
            // 空块：Java getLastLeaf 返回元素自身（heeds=false，默认）→ 按行号判定
            None => {
                if e.span.line != line {
                    return None;
                }
            }
        }
    }
}

/// 沿同级向前逐叶前进，返回首个"care 空白"且起始行 == `line` 的叶
/// （Java trailingCharsToStrip 的 nextTerminalNode 链，TextBlock.java:304-312；
/// 语义与 walk_prev 对称）。
pub(super) fn walk_next(
    els: &[Element],
    i: usize,
    line: u32,
    parent_next: Option<Term>,
    is_root: bool,
) -> Option<Term> {
    let mut j = i;
    loop {
        j += 1;
        let Some(e) = els.get(j) else {
            return parent_next;
        };
        if is_trim_element(e) || is_ignorable_text(els, j, is_root) {
            continue;
        }
        match first_leaf(e) {
            Some(t) => {
                if t.line != line {
                    return None;
                }
                if t.heeds {
                    return Some(t);
                }
            }
            None => {
                if e.span.line != line {
                    return None;
                }
            }
        }
    }
}

/// 终结节点信息（空白剥离同行检查用）
pub(super) fn first_leaf(el: &Element) -> Option<Term> {
    match &el.kind {
        ElementKind::Text { text, .. } => Some(Term {
            // Java TextBlock.heedsTrailingWhitespace（正向扫描：先遇换行 → false，先遇非空白 → true）
            heeds: heeds_trailing(text),
            line: el.span.line,
        }),
        ElementKind::Interpolation { .. } => Some(Term {
            // Java DollarVariable.heedsTrailingWhitespace = true（DollarVariable.java:132-135）
            heeds: true,
            line: el.span.line,
        }),
        ElementKind::If { then, else_, .. } => {
            first_leaf_slice(then).or_else(|| else_.as_deref().and_then(first_leaf_slice))
        }
        ElementKind::List { body, else_, .. } => {
            first_leaf_slice(body).or_else(|| else_.as_deref().and_then(first_leaf_slice))
        }
        // Java：宏定义是"不可见"元素（Macro.heedsOpening/TrailingWhitespace = false），
        // 不深入 body——其后/前的文本按行首剥离处理
        ElementKind::Macro { .. } => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
        // Java：块赋值与宏一样视为叶（TemplateElement.getFirstLeaf :488-495 的
        // `!(te instanceof BlockAssignment)` 特例），heedsOpeningWhitespace=false ——
        // 不深入 body，其后文本的行首空白照剥（BlockAssignment 的捕获输出不参与
        // 相邻文本的空白剥离判定；同因影响 examples_test.rs 的 testCapture 等）
        ElementKind::BlockAssign { .. } => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
        ElementKind::Trim(b)
        | ElementKind::Compress(b)
        | ElementKind::NoEscape(b)
        | ElementKind::AutoEsc(b)
        | ElementKind::NoAutoEsc(b)
        | ElementKind::Items { body: b, .. }
        | ElementKind::Sep { body: b }
        | ElementKind::Transform { body: b, .. }
        | ElementKind::On { body: b, .. }
        | ElementKind::Call { body: Some(b), .. }
        | ElementKind::Escape { body: b, .. }
        | ElementKind::OutputFormat { body: b, .. }
        | ElementKind::Nested { body: Some(b), .. }
        | ElementKind::Global { body: Some(b), .. }
        | ElementKind::Local { body: Some(b), .. } => first_leaf_slice(b),
        ElementKind::Attempt { try_, recover, .. } => {
            first_leaf_slice(try_).or_else(|| first_leaf_slice(recover))
        }
        ElementKind::Switch { cases, default, .. } => {
            for c in cases {
                if !c.body.is_empty() {
                    return first_leaf(&c.body[0]);
                }
            }
            default.as_deref().and_then(first_leaf_slice)
        }
        _ => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
    }
}

pub(super) fn first_leaf_slice(els: &[Element]) -> Option<Term> {
    // 跳过 trim 标记（`<#t>`/`<#nt>`/`<#lt>`）：Java postParseCleanup 把
    // TrimInstruction 从树中移除（TemplateElement.java:404-420），prev/next
    // TerminalNode 链直达其后的叶——`<#list>...${x}<#t></#list>` 的末叶是
    // ${x} 而非 TrimLineStart（heeds=false 会错误放行行首剥离）
    els.iter()
        .find(|e| !is_trim_element(e))
        .and_then(first_leaf)
}

/// 元素的末个终结叶（Java getLastLeaf；Text 用**原始**结束行 ——
/// Java TextBlock 的 endLine 在空白剥离时不变，TextBlock.java:206-208 只动 beginLine）
pub(super) fn last_leaf(el: &Element) -> Option<Term> {
    match &el.kind {
        ElementKind::Text {
            text,
            orig_end_line,
            ..
        } => Some(Term {
            // Java TextBlock.heedsOpeningWhitespace（反向扫描：先遇换行 → false，先遇非空白 → true）
            heeds: heeds_opening(text),
            line: *orig_end_line,
        }),
        ElementKind::Interpolation { .. } => Some(Term {
            // Java DollarVariable.heedsOpeningWhitespace = true（DollarVariable.java:127-130）
            heeds: true,
            line: el.span.line,
        }),
        ElementKind::Comment { text } | ElementKind::RawText(text) => Some(Term {
            heeds: false,
            line: el.span.line + newline_count(text),
        }),
        ElementKind::NoParse {
            text,
            orig_end_line,
            ..
        } => Some(Term {
            // Java：unparsed TextBlock 的 heeds 由内容决定（TextBlock.java:315-345）
            heeds: heeds_opening(text),
            line: *orig_end_line,
        }),
        ElementKind::If { then, else_, .. } => {
            last_leaf_slice(then).or_else(|| else_.as_deref().and_then(last_leaf_slice))
        }
        ElementKind::List { body, else_, .. } => {
            last_leaf_slice(body).or_else(|| else_.as_deref().and_then(last_leaf_slice))
        }
        // Java：宏定义不可见（Macro.heeds*Whitespace = false），不深入 body
        ElementKind::Macro { .. } => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
        // Java：块赋值视为叶（TemplateElement.getLastLeaf :497-504 的
        // `!(te instanceof BlockAssignment)` 特例），heedsOpeningWhitespace=false
        ElementKind::BlockAssign { .. } => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
        ElementKind::Trim(b)
        | ElementKind::Compress(b)
        | ElementKind::NoEscape(b)
        | ElementKind::AutoEsc(b)
        | ElementKind::NoAutoEsc(b)
        | ElementKind::Items { body: b, .. }
        | ElementKind::Sep { body: b }
        | ElementKind::Transform { body: b, .. }
        | ElementKind::On { body: b, .. }
        | ElementKind::Call { body: Some(b), .. }
        | ElementKind::Escape { body: b, .. }
        | ElementKind::OutputFormat { body: b, .. }
        | ElementKind::Nested { body: Some(b), .. }
        | ElementKind::Global { body: Some(b), .. }
        | ElementKind::Local { body: Some(b), .. } => last_leaf_slice(b),
        ElementKind::Attempt { try_, recover, .. } => {
            last_leaf_slice(recover).or_else(|| last_leaf_slice(try_))
        }
        ElementKind::Switch { cases, default, .. } => {
            if let Some(d) = default {
                if !d.is_empty() {
                    return last_leaf(&d[d.len() - 1]);
                }
            }
            for c in cases.iter().rev() {
                if !c.body.is_empty() {
                    return last_leaf(&c.body[c.body.len() - 1]);
                }
            }
            None
        }
        _ => Some(Term {
            heeds: false,
            line: el.span.line,
        }),
    }
}

pub(super) fn last_leaf_slice(els: &[Element]) -> Option<Term> {
    // 同 first_leaf_slice：跳过 trim 标记（Java 已从树中移除 TrimInstruction）
    els.iter()
        .rev()
        .find(|e| !is_trim_element(e))
        .and_then(last_leaf)
}

/// Java `TextBlock.heedsOpeningWhitespace`（TextBlock.java:215-226）：从文本末尾反向扫描，
/// 先遇换行 → false（不 care 行首空白）；先遇非空白字符 → true（care）；全空白 → true。
/// 空文本 → false（Java isIgnorable("") → true → :316-318 早退）。
pub(super) fn heeds_opening(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    for c in text.chars().rev() {
        if c == '\n' || c == '\r' {
            return false;
        }
        if !is_ws(c) {
            return true;
        }
    }
    true
}

/// Java `TextBlock.heedsTrailingWhitespace`（TextBlock.java:228-239）：从文本开头正向扫描，
/// 先遇换行 → false；先遇非空白字符 → true；全空白 → true。空文本 → false（同上）。
pub(super) fn heeds_trailing(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    for c in text.chars() {
        if c == '\n' || c == '\r' {
            return false;
        }
        if !is_ws(c) {
            return true;
        }
    }
    true
}

/// 换行数（`\n` 计数；`\r\n` 由 `\n` 计一次 —— 与 lexer 的行号一致，孤立 `\r` 不计）
pub(super) fn newline_count(text: &str) -> u32 {
    text.chars().filter(|c| *c == '\n').count() as u32
}

/// 首个换行（含）之后的起始下标（Java openingCharsToStrip 的裁剪量）
pub(super) fn first_newline_end(s: &str) -> usize {
    match s.find('\n') {
        Some(i) => i + 1,
        None => s.len(),
    }
}

/// 最后一个换行之后的起始下标（Java trailingCharsToStrip 的保留起点）
pub(super) fn last_newline_start(s: &str) -> usize {
    match s.rfind('\n') {
        Some(i) => i + 1,
        None => s.len(),
    }
}

pub(super) fn is_ws(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r' || c == '\n'
}

/// 文本首部空白（到首个换行含换行）是否全为空白（Java openingCharsToStrip 判定）
pub(super) fn leading_ws_through_newline(text: &str) -> bool {
    for c in text.chars() {
        if c == '\n' || c == '\r' {
            return true;
        }
        if !is_ws(c) {
            return false;
        }
    }
    false
}

/// 文本尾部（最后一个换行后）是否全为空白且非空（Java trailingCharsToStrip 判定；
/// 无换行时仅当文本整行空白且始于列 1 才可剥）
pub(super) fn trailing_ws_after_newline(text: &str, begin_col: u32) -> bool {
    match text.rfind(['\n', '\r']) {
        Some(i) => {
            let trail = &text[i + 1..];
            !trail.is_empty() && trail.chars().all(is_ws)
        }
        None => begin_col == 1 && !text.is_empty() && text.chars().all(is_ws),
    }
}
