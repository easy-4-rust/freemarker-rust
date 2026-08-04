//! 对应 Java `Rust 特有（对应 Java NodeModel 的 TemplateHashModel 角色；Java 无独立文件）`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::core::Environment;
use crate::error::Result;
use crate::template::TModel;

/// 节点哈希访问 —— 对应 Java `NodeModel` 的 `TemplateHashModel` 角色
/// （`doc.foo` / `doc['//x']` / `doc.@@markup` 等节点键访问）。与普通哈希不同，
/// get 需要 `Environment` 以解析当前命名空间的 `ns_prefixes`（Java 用线程局部
/// Environment.getCurrentEnvironment，Rust 显式传参；docs/06）。
pub trait NodeHashModel {
    /// 键查找：`@@` 特殊键 / 子元素名 / XPath 子集查询。返回 None = 键缺失
    /// （Java SimpleHash.get 返回 null 的语义，由使用点决定报错/回退）。
    fn get(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>>;
}
