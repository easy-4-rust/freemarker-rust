//! 列表块 —— 对应 Java `freemarker.core.IteratorBlock`
//! （acceptWithResult :98-111 + IterationContext :190-468 + visitIteratorBlock
//! Environment.java:3465-3476；`<#items>`/`<#sep>` 就地元素语义见 items.rs/sep.rs）

use crate::core::environment::{LocalEntry, LoopCtx, RunSignal};
use crate::core::eval;
use crate::core::exec::ExecOutcome;
use crate::core::{Element, ElementKind, Expr};
use crate::error::{FlowKind, Result, TemplateError};
use crate::template::TModel;
use std::cell::RefCell;
use std::rc::Rc;

/// `<#list>` 块（对应 IteratorBlock.java；items/sep 为就地元素）
pub struct IteratorBlock {
    pub seq: Expr,
    pub var: String,
    pub var2: Option<String>,
    pub body: Vec<Element>,
    pub else_: Option<Vec<Element>>,
}

impl IteratorBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(
        seq: Expr,
        var: String,
        var2: Option<String>,
        body: Vec<Element>,
        else_: Option<Vec<Element>>,
    ) -> Self {
        IteratorBlock {
            seq,
            var,
            var2,
            body,
            else_,
        }
    }

    /// 执行（Java acceptWithResult :97-111）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_list(
            env,
            &self.seq,
            &self.var,
            &self.var2,
            &self.body,
            &self.else_,
        )
    }
}

fn exec_list(
    env: &mut crate::core::Environment,
    seq_expr: &crate::core::Expr,
    var: &str,
    var2: &Option<String>,
    body: &[Element],
    else_: &Option<Vec<Element>>,
) -> Result<ExecOutcome> {
    // Java acceptWithResult :97-102：列表源 null → classic 兼容模式视为空序列
    // （Constants.EMPTY_SEQUENCE）；strict → assertNonNull（本引擎解析层已抛）
    let listed = eval::eval(env, seq_expr)?;
    let listed = if listed.is_nothing() && env.settings.classic_compatible {
        TModel::from_sequence(Vec::new())
    } else {
        listed
    };
    // 列出模式（Java FTL.jj List :2808-2812 与 Items :2943-2953：iterCtx.hashListing
    // 由 `<#list ... as k, v>` 或嵌套 `<#items as k, v>` 置位——`<#list hash>`
    // 无循环变量 + `<#items as k, v>` 同样按键/值对列出，listhash 用例第 40-44 行）
    let hash_listing = var2.is_some()
        || (var.is_empty()
            && body
                .iter()
                .any(|el| matches!(&el.kind, ElementKind::Items { var2: Some(_), .. })));
    let mut items: crate::core::environment::PendingItems =
        materialize_list_items(env, &listed, hash_listing)?;
    if !items.has_next()? {
        // Java ListElseContainer：空 → else（无循环变量）
        if let Some(e) = else_ {
            return Ok(ExecOutcome::Next(e.clone()));
        }
        return Ok(ExecOutcome::Done);
    }
    // 单个迭代上下文贯穿整个列表（Java IterationContext 单对象模型）
    let lc = Rc::new(RefCell::new(LoopCtx {
        var_name: var.to_string(),
        var2_name: var2.clone(),
        value: None,
        key: None,
        index: 0,
        has_next: false,
        pending: items,
        items_entered: false,
    }));
    env.push_local(LocalEntry::Loop(lc.clone()));
    let r = if !var.is_empty() {
        // Java executedNestedContentForCollOrSeqListing 的 loopVar1Name != null 分支
        run_loop_iterations(env, &lc, body)
    } else {
        // Java：body 执行一次；#items 元素驱动迭代；break/continue 上传由外层捕获
        match env.run(body) {
            Ok(RunSignal::Completed) => Ok(ExecOutcome::Done),
            Ok(RunSignal::Returned(v)) => Ok(ExecOutcome::ReturnValue(v)),
            Err(e) => Err(e),
        }
    };
    env.pop_local();
    r
}

fn materialize_list_items(
    env: &mut crate::core::Environment,
    listed: &TModel,
    hash_listing: bool,
) -> Result<crate::core::environment::PendingItems> {
    if hash_listing {
        // Java executedNestedContentForHashListing（:327-431）
        let ex = listed.hash_ex.clone().ok_or_else(|| {
            TemplateError::misc(format!(
                "The value you try to list is a {}, thus you must specify only one loop variable after the \"as\" (there's no separate key and value).",
                listed.type_name
            ))
        })?;
        let mut out = std::collections::VecDeque::new();
        // entries()（Java TemplateHashModelEx2.keyValuePairsIterator，:327-431）：
        // 常规哈希与 keys+get 等价；legacy HashLiteral（重复键）输出原始键值对
        for (key, value) in ex.entries()? {
            out.push_back(crate::core::environment::LoopItem {
                key: Some(TModel::from_scalar(key)),
                value: Some(value),
            });
        }
        return Ok(crate::core::environment::PendingItems::eager(out));
    }
    // 有界范围快路径（Java RangeModel 迭代器 O(1) 前视——has_next 不构造下一项值；
    // 仅限有界范围：无界（`4..`）保持惰性迭代器路径，ICI < 2.3.21 的
    // NonListableRightUnboundedRange（迭代为空）不受影响）
    if let Some(rs) = &listed.range {
        if !rs.unbounded {
            return Ok(crate::core::environment::PendingItems::range(
                crate::core::environment::RangeIterState {
                    start: rs.start,
                    index: 0,
                    cap: rs.count,
                    ascending: rs.ascending,
                },
            ));
        }
    }
    // Java IteratorBlock.java:278：TemplateCollectionModel 优先 → 惰性迭代器
    if let Some(c) = &listed.collection {
        let iter = c.iterator()?;
        return Ok(crate::core::environment::PendingItems::lazy(Box::new(
            iter.map(|r| {
                r.map(|v| crate::core::environment::LoopItem {
                    key: None,
                    value: Some(v),
                })
            }),
        )));
    }
    // Java IteratorBlock.java:310：TemplateSequenceModel → size/get 物化
    if let Some(s) = &listed.sequence {
        let size = s.size()?;
        let mut out = std::collections::VecDeque::new();
        for i in 0..size {
            let v = s.get(i)?;
            out.push_back(crate::core::environment::LoopItem {
                key: None,
                value: Some(v),
            });
        }
        return Ok(crate::core::environment::PendingItems::eager(out));
    }
    // Java IteratorBlock.java:335-352：classic 兼容模式下非序列/集合值 → 单次迭代，
    // 循环变量绑定该值本身（2.1 经典语义，`<#list 'a' as x>` → x = "a" 执行一次）
    if env.settings.classic_compatible {
        return Ok(crate::core::environment::PendingItems::eager(
            std::collections::VecDeque::from([crate::core::environment::LoopItem {
                key: None,
                value: Some(listed.clone()),
            }]),
        ));
    }
    // Java NonSequenceOrCollectionException（v1 消息简化）
    let _ = env;
    Err(TemplateError::misc(format!(
        "The value you try to list is a {}; it must be a sequence or collection.",
        listed.type_name
    )))
}

pub(crate) fn run_loop_iterations(
    env: &mut crate::core::Environment,
    lc: &Rc<RefCell<LoopCtx>>,
    block: &[Element],
) -> Result<ExecOutcome> {
    loop {
        // 单次借用完成 pop + 循环变量绑定 + has_next 前视（热路径减少 RefCell 借用）
        let item_pending = {
            let mut c = lc.borrow_mut();
            match c.pending.pop()? {
                Some(i) => {
                    c.key = i.key.clone();
                    c.value = i.value;
                    c.has_next = c.pending.has_next()?;
                    true
                }
                None => false,
            }
        };
        if !item_pending {
            break;
        }
        match env.run(block) {
            Ok(RunSignal::Completed) => {}
            Ok(RunSignal::Returned(v)) => return Ok(ExecOutcome::ReturnValue(v)),
            Err(TemplateError::Flow(FlowKind::Break)) => break,
            Err(TemplateError::Flow(FlowKind::Continue)) => {
                lc.borrow_mut().index += 1;
                continue;
            }
            Err(e) => return Err(e),
        }
        lc.borrow_mut().index += 1;
    }
    Ok(ExecOutcome::Done)
}
