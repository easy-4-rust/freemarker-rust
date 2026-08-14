//! 右无界范围模型（ICI < 2.3.21）—— 对应 Java `NonListableRightUnboundedRangeModel`：
//! 旧版兼容：size() = 0、迭代为空（`(4..)?size` == 0、`<#list 4.. as i>` 不执行、
//! `(4..)[0]` 越界 → 数字键路径按 invalid reference 报错）

use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateCollectionModel, TemplateSequenceModel};
use std::rc::Rc;

/// 右无界范围（ICI < 2.3.21）—— 对应 Java `NonListableRightUnboundedRangeModel`：
/// 旧版兼容：size() = 0、迭代为空（`(4..)?size` == 0、`<#list 4.. as i>` 不执行、
/// `(4..)[0]` 越界 → 数字键路径按 invalid reference 报错）
pub(crate) struct NonListableRightUnboundedRange;

pub(crate) fn nonlistable_right_unbounded_range_model(_start: i64) -> TModel {
    // Java NonListable 同样持有 begin（构造函数），但 size=0 时无从可见
    let seq: Rc<dyn TemplateSequenceModel> = Rc::new(NonListableRightUnboundedRange);
    let coll: Rc<dyn TemplateCollectionModel> = Rc::new(NonListableRightUnboundedRange);
    TModel {
        sequence: Some(seq),
        collection: Some(coll),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

impl TemplateSequenceModel for NonListableRightUnboundedRange {
    fn get(&self, _index: usize) -> Result<TModel> {
        Err(TemplateError::misc("Range item index is out of bounds."))
    }
    fn size(&self) -> Result<usize> {
        Ok(0)
    }
}

impl TemplateCollectionModel for NonListableRightUnboundedRange {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        Ok(Box::new(std::iter::empty()))
    }
}
