//! 有界范围模型 —— 对应 Java `freemarker.core.BoundedRangeModel`
//! （TemplateSequenceModel：get(i) = begin ± i，size 惰性计算；不急切物化，
//! 避免超大范围内存爆炸）

use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateCollectionModel, TemplateSequenceModel};
use crate::value::TNumber;
use std::rc::Rc;

/// 有界范围序列 —— 对应 Java `BoundedRangeModel`（TemplateSequenceModel：get(i) = begin ± i，
/// size 惰性计算；不急切物化，避免超大范围内存爆炸）
pub(crate) struct BoundedRangeSeq {
    start: i64,
    count: usize,
    ascending: bool,
}

pub(crate) fn bounded_range_model(start: i64, count: usize, ascending: bool) -> TModel {
    let seq = Rc::new(BoundedRangeSeq {
        start,
        count,
        ascending,
    });
    TModel {
        sequence: Some(seq.clone()),
        collection: Some(seq),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

impl TemplateSequenceModel for BoundedRangeSeq {
    fn get(&self, index: usize) -> Result<TModel> {
        if index >= self.count {
            return Err(TemplateError::misc(format!(
                "Sequence index out of bounds: {index}"
            )));
        }
        let v = if self.ascending {
            self.start + index as i64
        } else {
            self.start - index as i64
        };
        Ok(TModel::from_number(TNumber::from_i64(v)))
    }
    fn size(&self) -> Result<usize> {
        Ok(self.count)
    }
}

impl TemplateCollectionModel for BoundedRangeSeq {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        let (start, count, ascending) = (self.start, self.count, self.ascending);
        Ok(Box::new((0..count).map(move |i| {
            let v = if ascending {
                start + i as i64
            } else {
                start - i as i64
            };
            Ok(TModel::from_number(TNumber::from_i64(v)))
        })))
    }
}
