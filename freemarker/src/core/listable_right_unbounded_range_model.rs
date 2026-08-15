//! 右无界范围模型（ICI ≥ 2.3.21）—— 对应 Java `ListableRightUnboundedRangeModel`：
//! 序列 + 集合双角色；`?size` = Integer.MAX_VALUE；`r[i]` = begin + i（越界抛
//! "Range item index ... is out of bounds."）；迭代器带上限防呆

use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateCollectionModel, TemplateSequenceModel};
use crate::value::TNumber;
use std::rc::Rc;

/// 无界范围迭代上限（防呆；Java 为真正的无限惰性序列）
const UNBOUNDED_RANGE_ITER_CAP: usize = 1_000_000;

/// 右无界范围序列长度 —— Java `ListableRightUnboundedRangeModel.size()`
/// 返回 `Integer.MAX_VALUE`（2147483647）
const UNBOUNDED_RANGE_SIZE: usize = i32::MAX as usize;

/// 右无界范围（ICI ≥ 2.3.21）—— 对应 Java `ListableRightUnboundedRangeModel`：
/// 序列 + 集合双角色；`?size` = Integer.MAX_VALUE；`r[i]` = begin + i（越界抛
/// "Range item index ... is out of bounds."）；迭代器带上限防呆
pub(crate) struct ListableRightUnboundedRange {
    start: i64,
}

pub(crate) fn listable_right_unbounded_range_model(start: i64) -> TModel {
    let inner = Rc::new(ListableRightUnboundedRange { start });
    let seq: Rc<dyn TemplateSequenceModel> = inner.clone();
    let coll: Rc<dyn TemplateCollectionModel> = inner;
    TModel {
        sequence: Some(seq),
        collection: Some(coll),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

impl TemplateSequenceModel for ListableRightUnboundedRange {
    fn get(&self, index: usize) -> Result<TModel> {
        if index >= UNBOUNDED_RANGE_SIZE {
            return Err(TemplateError::misc(format!(
                "Range item index {index} is out of bounds."
            )));
        }
        Ok(TModel::from_number(TNumber::from_i64(
            self.start + index as i64,
        )))
    }
    fn size(&self) -> Result<usize> {
        Ok(UNBOUNDED_RANGE_SIZE)
    }
}

impl TemplateCollectionModel for ListableRightUnboundedRange {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        let start = self.start;
        Ok(Box::new((0..UNBOUNDED_RANGE_ITER_CAP).map(move |i| {
            Ok(TModel::from_number(TNumber::from_i64(start + i as i64)))
        })))
    }
}
