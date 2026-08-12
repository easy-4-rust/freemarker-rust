//! 模板模型工具 —— 对应 Java `freemarker.template.utility.TemplateModelUtils`
//! （getKeyValuePairIterator / wrapAsHashUnion / modelsEqual 等；
//! v1 的 models_equal 在 builtins/eval_util.rs，本文件为公开聚合）

use crate::error::Result;
use crate::template::{TModel, TemplateHashModelEx};

/// 模板模型工具（对应 TemplateModelUtils.java）
pub struct TemplateModelUtils;

impl TemplateModelUtils {
    /// 键值对迭代（Java `getKeyValuePairIterator`；v1 = Ex.entries）
    pub fn get_key_value_pair_iterator(
        hash: &dyn TemplateHashModelEx,
    ) -> Result<Vec<(String, TModel)>> {
        hash.entries()
    }

    /// 哈希并集（Java `wrapAsHashUnion`：多哈希合并为联合视图）
    /// v1 差异：需逐键合并——仅支持 TemplateHashModelEx 输入（可枚举键），
    /// 结果为新哈希（Java 返回惰性联合视图）
    pub fn wrap_as_hash_union(hashes: &[&dyn TemplateHashModelEx]) -> Result<TModel> {
        let mut map = indexmap::IndexMap::new();
        for h in hashes {
            for (k, v) in h.entries()? {
                map.insert(k, v);
            }
        }
        Ok(TModel::from_hash(map))
    }
}
