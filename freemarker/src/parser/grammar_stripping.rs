//! 空白剥离标记（对应 TextBlock.postParseCleanup；docs/08 §5.2）。

use super::grammar_element_helpers::{
    first_leaf, first_leaf_slice, is_ignorable_text, last_leaf, leaf_heeds_opening, walk_next,
    walk_prev, Term,
};
use super::grammar_element_helpers::{
    first_newline_end, last_newline_start, leading_ws_through_newline, trailing_ws_after_newline,
};
use super::grammar_helpers::{trim_first_line_trailing, trim_last_line_leading};
use super::Parser;
use crate::core::{Element, ElementKind};

impl<'a> Parser<'a> {
    pub(crate) fn mark_stripping(root: &mut [Element], strip_ws: bool) {
        if !strip_ws {
            return;
        }
        Self::mark_block(root, None, None, None, None, true);
    }

    /// 对文本元素做 deliberate 扫描（Java TextBlock.deliberateLeftTrim/RightTrim，
    /// TextBlock.java:143-236）：只依赖元素类型与行号，与文本内容无关
    /// → 可在可变借用前计算。返回 (left_trim, left_blocked, right_trim, right_blocked, heinous_drop)。
    /// Java nextTerminalNode/prevTerminalNode 链**跨块**（沿同级后续兄弟块继续同行
    /// 扫描——如 `<#on 1>C1\n    <#on 2>C2<#t>` 里 on-2 的 `<#t>` 裁 on-1 尾文本，
    /// SwitchTest testOnWhitespace 断言；`next_els`/`prev_els` 为下一/上一兄弟块）。
    pub(crate) fn deliberate_scan(
        els: &[Element],
        i: usize,
        begin_line: u32,
        end_line: u32,
        is_root: bool,
        next_els: Option<&[Element]>,
        prev_els: Option<&[Element]>,
    ) -> (bool, bool, bool, bool, bool) {
        // deliberateLeftTrim：后面同行 lt/t 裁最后一行行首；nt 阻止
        // （Java 循环条件 `elem.beginLine == this.endLine` 对每个元素生效，换行即终止）
        let mut left_trim = false;
        let mut left_blocked = false;
        for e in Self::scan_next_chain(els, i, end_line, next_els) {
            match &e.kind {
                ElementKind::LeftTrimLine | ElementKind::TrimLineStart => {
                    left_trim = true;
                    break;
                }
                ElementKind::NoTrimLineStart => {
                    left_blocked = true;
                    break;
                }
                _ => {}
            }
        }
        // deliberateRightTrim：前面同行 rt/t 裁第一行尾部；nt 阻止
        let mut right_trim = false;
        let mut right_blocked = false;
        for e in Self::scan_prev_chain(els, i, begin_line, prev_els) {
            match &e.kind {
                ElementKind::TrimLineEnd | ElementKind::TrimLineStart => {
                    right_trim = true;
                    break;
                }
                ElementKind::NoTrimLineStart => {
                    right_blocked = true;
                    break;
                }
                _ => {}
            }
        }
        // HEINOUS 块（TextBlock.java:239-254）：right_trim 且 opening 不可整段裁时，
        // trailing（首行后的空白段）是否裁掉由后文同行决定：遇 heedsOpeningWhitespace
        // 元素 → 保留（ignorable 文本 heeds=false，:316-318）；遇 lt/t → 裁掉并终止
        let heinous_drop = if right_trim {
            let mut drop = true;
            for (k, e) in els.iter().enumerate().skip(i + 1) {
                if e.span.line > end_line {
                    break;
                }
                if leaf_heeds_opening(e) && !is_ignorable_text(els, k, is_root) {
                    drop = false;
                }
                if matches!(
                    e.kind,
                    ElementKind::LeftTrimLine | ElementKind::TrimLineStart
                ) {
                    drop = true;
                    break;
                }
            }
            // 块内耗尽后继续下一兄弟块（Java nextTerminalNode 跨块链）
            if let Some(nx) = next_els {
                for e in nx {
                    if e.span.line > end_line {
                        break;
                    }
                    if leaf_heeds_opening(e) {
                        drop = false;
                    }
                    if matches!(
                        e.kind,
                        ElementKind::LeftTrimLine | ElementKind::TrimLineStart
                    ) {
                        drop = true;
                        break;
                    }
                }
            }
            drop
        } else {
            false
        };
        (
            left_trim,
            left_blocked,
            right_trim,
            right_blocked,
            heinous_drop,
        )
    }

    /// 后继元素链（Java nextTerminalNode 的块内 + 下一兄弟块部分）：
    /// 逐元素（span 起始行 <= end_line 才参与，Java `elem.beginLine == this.endLine`）
    pub(crate) fn scan_next_chain<'x>(
        els: &'x [Element],
        i: usize,
        end_line: u32,
        next_els: Option<&'x [Element]>,
    ) -> impl Iterator<Item = &'x Element> {
        els.iter()
            .skip(i + 1)
            .chain(next_els.into_iter().flatten())
            .take_while(move |e| e.span.line <= end_line)
    }

    /// 前驱元素链（deliberateRightTrim 用；块内逆序 + 上一兄弟块逆序，
    /// span 起始行 >= begin_line 才参与）
    pub(crate) fn scan_prev_chain<'x>(
        els: &'x [Element],
        i: usize,
        begin_line: u32,
        prev_els: Option<&'x [Element]>,
    ) -> impl Iterator<Item = &'x Element> {
        els[..i]
            .iter()
            .rev()
            .chain(prev_els.into_iter().flatten().rev())
            .take_while(move |e| e.span.line >= begin_line)
    }

    /// `prev_els`/`next_els`：上一/下一兄弟块的元素（deliberate 链跨块扫描用；
    /// Java nextTerminalNode/prevTerminalNode 沿整树同行扫描）
    pub(crate) fn mark_block(
        els: &mut [Element],
        prev: Option<Term>,
        next: Option<Term>,
        prev_els: Option<&[Element]>,
        next_els: Option<&[Element]>,
        is_root: bool,
    ) {
        for i in 0..els.len() {
            // Java prevTerminalNode/nextTerminalNode（TextBlock.java:443-464）：逐叶线性
            // 回溯/前进，**跳过 ignorable 节点**（TrimInstruction.isIgnorable=true）；
            // openingCharsToStrip/trailingCharsToStrip（:280-312）的同行循环对链上每个叶
            // 检查 heeds，命中 care 的叶 → 不剥；遇到跨行叶 → 链终止 → 照剥。
            // 注意：链**不止一跳**——`  ${""}<@x/><#lt>\n` 的 `\n` 须穿过
            // Trim(heeds=false) 与 Call(heeds=false) 直达 Interp(heeds=true) 才判定不剥。
            // prev 链用文本 beginLine 匹配、next 链用 endLine 匹配（Java 循环条件）；
            // 文本链上叶的 heeds/行号按**当前（已裁剪）内容 + 原始 endLine** 读取
            // （本层按序处理，之前的文本已 deliberate 裁剪 —— 与 Java 逐元素顺序一致）
            let (begin_line, end_line) = match &els[i].kind {
                ElementKind::Text { orig_end_line, .. }
                | ElementKind::NoParse { orig_end_line, .. } => (els[i].span.line, *orig_end_line),
                // 块元素：next 链行号取**末叶**的结束行（Java 链从块内最后一个叶继续，
                // 条件 `elem.beginLine == this.endLine` 用的是该叶的 endLine，
                // 如 `<#if y>foo\n  </#if>bar` —— "bar" 在行 2 与 "foo\n  " 的 endLine 匹配）
                _ => (
                    els[i].span.line,
                    last_leaf(&els[i])
                        .map(|t| t.line)
                        .unwrap_or(els[i].span.line),
                ),
            };
            let prev_i = walk_prev(els, i, begin_line, prev, is_root);
            let next_i = walk_next(els, i, end_line, next, is_root);
            let span_col = els[i].span.col;
            let (left_trim, left_blocked, right_trim, right_blocked, heinous_drop) =
                Self::deliberate_scan(els, i, begin_line, end_line, is_root, next_els, prev_els);
            match &mut els[i].kind {
                ElementKind::Text {
                    text,
                    strip_before,
                    strip_after,
                    ..
                }
                | ElementKind::NoParse {
                    text,
                    strip_before,
                    strip_after,
                    ..
                } => {
                    // Java 顺序（TextBlock.java:110-111, 119-124）：deliberateLeftTrim 先、
                    // deliberateRightTrim 后，自动剥离（openingCharsToStrip/trailingCharsToStrip
                    // :264-312）作用于**裁剪后**文本；deliberate 侧不应用自动剥离。
                    // 首文本的早退（:116-118）在 deliberate 裁剪**之后** —— 先裁后跳过
                    if left_trim {
                        trim_last_line_leading(text, span_col);
                    }
                    let first_line_dropped = if right_trim {
                        trim_first_line_trailing(text, heinous_drop)
                    } else {
                        false
                    };
                    if is_root && i == 0 {
                        continue; // 模板首文本：自动剥离跳过（Java 守卫）
                    }
                    let sb = !(right_trim || right_blocked)
                        && leading_ws_through_newline(text)
                        && !prev_i.is_some_and(|t| t.heeds && t.line == begin_line);
                    // 裁剪掉首行后 Java beginColumn=1（:206-208）→ trailing 剥离列判定用有效列
                    let eff_col = if first_line_dropped { 1 } else { span_col };
                    let sa = !(left_trim || left_blocked)
                        && trailing_ws_after_newline(text, eff_col)
                        && !next_i.is_some_and(|t| t.heeds && t.line == end_line);
                    // Java postParseCleanup 在**解析期直接改写文本**（text = substring，
                    // TextBlock.java:128）——后续文本的 prev/next 链所见即最终内容
                    // （如 rt 裁后剩 "  "、再经自身 trailing 剥离成 "" → 空文本 ignorable
                    //  heeds=false，链继续穿过）。标记随置 false，渲染期 strip_text 为 no-op。
                    if sb {
                        *text = text[first_newline_end(text)..].to_string();
                    }
                    if sa {
                        let end = if text.contains('\n') {
                            last_newline_start(text)
                        } else {
                            0 // 无换行 → 整段剥（Java beginColumn==1 全空白）
                        };
                        *text = text[..end].to_string();
                    }
                    *strip_before = false;
                    *strip_after = false;
                }
                ElementKind::If { then, else_, .. } => {
                    let else_first = else_.as_deref().and_then(first_leaf_slice);
                    let then_last = if then.is_empty() {
                        prev_i
                    } else {
                        last_leaf(&then[then.len() - 1])
                    };
                    Self::mark_block(
                        then,
                        prev_i,
                        else_first.or(next_i),
                        prev_els,
                        else_.as_deref(),
                        false,
                    );
                    if let Some(e) = else_ {
                        let then_s: &[Element] = then.as_slice();
                        Self::mark_block(e, then_last, next_i, Some(then_s), next_els, false);
                    }
                }
                ElementKind::List { body, else_, .. } => {
                    let else_first = else_.as_deref().and_then(first_leaf_slice);
                    let body_last = if body.is_empty() {
                        prev_i
                    } else {
                        last_leaf(&body[body.len() - 1])
                    };
                    Self::mark_block(
                        body,
                        prev_i,
                        else_first.or(next_i),
                        prev_els,
                        else_.as_deref(),
                        false,
                    );
                    if let Some(e) = else_ {
                        let body_s: &[Element] = body.as_slice();
                        Self::mark_block(e, body_last, next_i, Some(body_s), next_els, false);
                    }
                }
                ElementKind::Macro { def, .. } => {
                    Self::mark_block(&mut def.body, prev_i, next_i, prev_els, next_els, false);
                }
                ElementKind::Trim(body)
                | ElementKind::Compress(body)
                | ElementKind::NoEscape(body)
                | ElementKind::AutoEsc(body)
                | ElementKind::NoAutoEsc(body)
                | ElementKind::BlockAssign { body, .. }
                | ElementKind::Items { body, .. }
                | ElementKind::Sep { body }
                | ElementKind::Transform { body, .. }
                | ElementKind::On { body, .. }
                | ElementKind::Call {
                    body: Some(body), ..
                } => {
                    Self::mark_block(body, prev_i, next_i, prev_els, next_els, false);
                }
                ElementKind::Escape { body, .. }
                | ElementKind::OutputFormat { body, .. }
                | ElementKind::Nested {
                    body: Some(body), ..
                }
                | ElementKind::Global {
                    body: Some(body), ..
                }
                | ElementKind::Local {
                    body: Some(body), ..
                } => {
                    Self::mark_block(body, prev_i, next_i, prev_els, next_els, false);
                }
                ElementKind::Attempt { try_, recover, .. } => {
                    let rec_first = if recover.is_empty() {
                        next_i
                    } else {
                        first_leaf(&recover[0])
                    };
                    let try_last = if try_.is_empty() {
                        prev_i
                    } else {
                        last_leaf(&try_[try_.len() - 1])
                    };
                    Self::mark_block(
                        try_,
                        prev_i,
                        rec_first,
                        prev_els,
                        Some(recover.as_slice()),
                        false,
                    );
                    Self::mark_block(
                        recover,
                        try_last,
                        next_i,
                        Some(try_.as_slice()),
                        next_els,
                        false,
                    );
                }
                ElementKind::Switch { cases, default, .. } => {
                    let mut cur_prev = prev_i;
                    for idx in 0..cases.len() {
                        // split_at_mut：邻居切片与当前 case 的可变借用互不冲突
                        // （索引借用会覆盖整个 vec——E0502）
                        let (before, rest) = cases.split_at_mut(idx);
                        let (cur, after) = rest.split_at_mut(1);
                        let cur_next = if !after.is_empty() {
                            first_leaf_slice(&after[0].body)
                        } else if let Some(d) = default {
                            first_leaf_slice(d.as_slice())
                        } else {
                            next_i
                        };
                        let prev_s: Option<&[Element]> = before.last().map(|c| c.body.as_slice());
                        let next_s: Option<&[Element]> = if !after.is_empty() {
                            Some(after[0].body.as_slice())
                        } else {
                            default.as_deref().or(next_els)
                        };
                        Self::mark_block(
                            cur[0].body.as_mut_slice(),
                            cur_prev,
                            cur_next,
                            prev_s,
                            next_s,
                            false,
                        );
                        cur_prev = if cur[0].body.is_empty() {
                            cur_prev
                        } else {
                            last_leaf(&cur[0].body[cur[0].body.len() - 1])
                        };
                    }
                    if let Some(d) = default {
                        let last_s = cases.last().map(|c| c.body.as_slice());
                        Self::mark_block(d, cur_prev, next_i, last_s.or(prev_els), next_els, false);
                    }
                }
                ElementKind::Call { body: None, .. }
                | ElementKind::Nested { body: None, .. }
                | ElementKind::Global { body: None, .. }
                | ElementKind::Local { body: None, .. }
                | ElementKind::Interpolation { .. }
                | ElementKind::Assignments(_)
                | ElementKind::Assign { .. }
                | ElementKind::Break
                | ElementKind::Continue
                | ElementKind::Return { .. }
                | ElementKind::Stop { .. }
                | ElementKind::Flush
                | ElementKind::Comment { .. }
                | ElementKind::Include { .. }
                | ElementKind::Import { .. }
                | ElementKind::Setting { .. }
                | ElementKind::FtlHeader { .. }
                | ElementKind::TrimLineStart
                | ElementKind::NoTrimLineStart
                | ElementKind::TrimLineEnd
                | ElementKind::LeftTrimLine
                | ElementKind::Visit { .. }
                | ElementKind::Recurse { .. }
                | ElementKind::Fallback
                | ElementKind::RawText(_) => {}
            }
        }
    }
}
