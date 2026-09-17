//! 循环迭代上下文 —— LocalEntry/LoopCtx impl/LoopItem/PendingItems/RangeIterState/BodyCtx
//! （对应 Java `LocalContext` + `IteratorBlock.IterationContext` + `BodyInstruction.Context`）。
//!
//! LoopCtx 类型定义在父模块 environment.rs（公开签名引用，路径须保持
//! `environment::LoopCtx` 不变）；本文件提供其 impl 与其余辅助类型。

use super::LoopCtx;
use crate::error::Result;
use crate::template::TModel;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// 局部上下文条目 —— 对应 Java `LocalContext` 接口 + `LocalContextStack`
#[derive(Clone)]
pub(crate) enum LocalEntry {
    /// 循环变量层（Java `IteratorBlock.IterationContext`；提供 `x`/`x_index`/`x_has_next`）
    Loop(Rc<RefCell<LoopCtx>>),
    /// `<#nested x>` 体参数层（Java `BodyInstruction.Context`，BodyInstruction.java:122-155）
    Body(Rc<BodyCtx>),
}

impl LocalEntry {
    pub(crate) fn get(&self, name: &str, fallback_null_loop_var: bool) -> Option<TModel> {
        match self {
            LocalEntry::Loop(lc) => lc.borrow().get(name, fallback_null_loop_var),
            LocalEntry::Body(bc) => bc.vars.get(name).cloned(),
        }
    }
}

/// 单个列表项（key=hashListing 键；value=None 表示 null 项）
#[derive(Clone)]
pub(crate) struct LoopItem {
    pub key: Option<TModel>,
    pub value: Option<TModel>,
}

/// 待迭代项（Java IterationContext.openedIterator，IteratorBlock.java:280-305）：
/// 惰性拉取——已物化项存 cache；集合角色保留底层迭代器按需取项
/// （`<#list (4..) as i>` 不会物化 2^31-1 项，Java 同样惰性驱动）；
/// `has_next` 前视：从迭代器拉一项入 cache（peek 语义，IteratorBlock.java:293-300）。
/// 有界范围（`1..100`）走 `range` 快路径：has_next 按 index/cap 判定（O(1) 零物化，
/// 对应 Java BoundedRangeModel 迭代器同样不构造下一项值）。
pub(crate) struct PendingItems {
    cache: std::collections::VecDeque<LoopItem>,
    iter: Option<Box<dyn Iterator<Item = Result<LoopItem>>>>,
    /// 范围快路径状态（start ± index 按需取值；仅遍历，不物化 ahead）
    range: Option<RangeIterState>,
}

/// 范围迭代状态（start + ascending*index；cap = 元素总数）
#[derive(Clone, Copy)]
pub(crate) struct RangeIterState {
    pub start: i64,
    pub index: usize,
    pub cap: usize,
    pub ascending: bool,
}

impl PendingItems {
    /// 已物化来源（hashListing / TemplateSequenceModel size-get 路径）
    pub(crate) fn eager(items: std::collections::VecDeque<LoopItem>) -> Self {
        PendingItems {
            cache: items,
            iter: None,
            range: None,
        }
    }

    /// 惰性来源（TemplateCollectionModel 迭代器）
    pub(crate) fn lazy(iter: Box<dyn Iterator<Item = Result<LoopItem>>>) -> Self {
        PendingItems {
            cache: std::collections::VecDeque::new(),
            iter: Some(iter),
            range: None,
        }
    }

    /// 有界范围来源（`1..100` 等；has_next 零物化前视）
    pub(crate) fn range(state: RangeIterState) -> Self {
        PendingItems {
            cache: std::collections::VecDeque::new(),
            iter: None,
            range: Some(state),
        }
    }

    /// 取下一项（无 → None）；迭代器错误向上传播
    pub(crate) fn pop(&mut self) -> Result<Option<LoopItem>> {
        if let Some(item) = self.cache.pop_front() {
            return Ok(Some(item));
        }
        if let Some(r) = &mut self.range {
            if r.index < r.cap {
                let v = if r.ascending {
                    r.start + r.index as i64
                } else {
                    r.start - r.index as i64
                };
                r.index += 1;
                return Ok(Some(LoopItem {
                    key: None,
                    value: Some(TModel::from_number(crate::value::TNumber::from_i64(v))),
                }));
            }
            return Ok(None);
        }
        if let Some(it) = self.iter.as_mut() {
            return match it.next() {
                Some(Ok(item)) => Ok(Some(item)),
                Some(Err(e)) => Err(e),
                None => {
                    self.iter = None;
                    Ok(None)
                }
            };
        }
        Ok(None)
    }

    /// 是否还有下一项（前视：从迭代器拉一项进 cache；范围路径 O(1) 判定）
    pub(crate) fn has_next(&mut self) -> Result<bool> {
        if !self.cache.is_empty() {
            return Ok(true);
        }
        if let Some(r) = &self.range {
            return Ok(r.index < r.cap);
        }
        if let Some(it) = self.iter.as_mut() {
            return match it.next() {
                Some(Ok(item)) => {
                    self.cache.push_back(item);
                    Ok(true)
                }
                Some(Err(e)) => Err(e),
                None => {
                    self.iter = None;
                    Ok(false)
                }
            };
        }
        Ok(false)
    }
}

impl LoopCtx {
    /// Java `IterationContext.getLocalVariable`（IteratorBlock.java:452-482）：
    /// 循环变量本身为 null 项时，fallbackOnNullLoopVariable=true → 返回 None（继续外层查找）；
    /// false → 返回 nothing（可见但为 null，读取报缺失）。
    pub(crate) fn get(&self, name: &str, fallback_null_loop_var: bool) -> Option<TModel> {
        if self.var_name.is_empty() {
            return None; // 循环变量不可见（#list 无 as 且未进入 #items）
        }
        if name == self.var_name {
            // var1：hashListing 时为键（Java loopVar1Value = kvp.getKey()）
            let v = if self.var2_name.is_some() {
                self.key.clone()
            } else {
                self.value.clone()
            };
            return match v {
                Some(m) if !m.is_nothing() => Some(m),
                _ if fallback_null_loop_var => None,
                _ => Some(TModel::nothing()),
            };
        }
        if let Some(v2) = &self.var2_name {
            if name == v2 {
                return match &self.value {
                    Some(m) if !m.is_nothing() => Some(m.clone()),
                    _ if fallback_null_loop_var => None,
                    _ => Some(TModel::nothing()),
                };
            }
        }
        // `x_index`/`x_has_next` 判定：strip_suffix 零分配（format! 每次变量查找
        // 都会构造新 String——循环体内热路径）
        if let Some(prefix) = name.strip_suffix("_index") {
            if prefix == self.var_name {
                return Some(TModel::from_number(crate::value::TNumber::from_i64(
                    self.index as i64,
                )));
            }
        }
        if let Some(prefix) = name.strip_suffix("_has_next") {
            if prefix == self.var_name {
                return Some(TModel::from_boolean(self.has_next));
            }
        }
        None
    }
}

/// `<#nested x>` 体参数 —— 对应 Java `BodyInstruction.Context.bodyVars`
pub(crate) struct BodyCtx {
    pub(crate) vars: HashMap<String, TModel>,
}
