//! 对应 Java `freemarker.template.TemplateHashModelEx`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::error::Result;
use crate::template::TModel;

pub trait TemplateHashModelEx: crate::template::TemplateHashModel {
    fn size(&self) -> Result<usize>;
    fn keys(&self) -> Result<Vec<String>>;
    /// 插入序条目（默认 = keys() 逐个 get；重复键模型可覆盖为原始键值对列表）
    fn entries(&self) -> Result<Vec<(String, TModel)>> {
        let mut out = Vec::new();
        for key in self.keys()? {
            if let Some(v) = self.get(&key)? {
                out.push((key, v));
            }
        }
        Ok(out)
    }
}
