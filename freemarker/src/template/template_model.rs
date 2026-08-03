//! 数据模型角色 trait 家族 —— 对应 Java `freemarker.template.TemplateModel` 接口家族
//! （接口→trait 映射见 docs/06 §1；全部 object-safe，支持 `Rc<dyn>` 槽位）

use crate::core::Environment;
use crate::error::Result;
use crate::template::TModel;
use crate::value::{DateValue, TNumber};
use std::collections::HashMap;

pub trait TemplateScalarModel {
    fn as_string(&self) -> Result<String>;
}

pub trait TemplateNumberModel {
    fn as_number(&self) -> Result<TNumber>;
}

pub trait TemplateBooleanModel {
    fn as_boolean(&self) -> Result<bool>;
}

pub trait TemplateDateModel {
    fn as_date(&self) -> Result<DateValue>;
}

pub trait TemplateSequenceModel {
    fn get(&self, index: usize) -> Result<TModel>;
    fn size(&self) -> Result<usize>;
}

/// 一次性集合（对应 TemplateCollectionModel：iterator 只能消费一次）
pub trait TemplateCollectionModel {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>>;
}
pub trait TemplateHashModel {
    fn get(&self, key: &str) -> Result<Option<TModel>>;
    fn is_empty(&self) -> Result<bool>;
}

pub trait TemplateHashModelEx: TemplateHashModel {
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

pub trait TemplateMethodModelEx {
    fn exec(&self, args: Vec<TModel>) -> Result<TModel>;
}

pub trait TemplateNodeModel {
    fn parent(&self) -> Result<Option<TModel>>;
    fn children(&self) -> Result<Vec<TModel>>;
    fn name(&self) -> Result<Option<String>>;
    fn node_type(&self) -> Result<String>;
    fn namespace(&self) -> Result<Option<String>>;
    /// Java TemplateNodeModelEx.getNextSibling（BuiltInsForNodes.nextSiblingBI）：
    /// 无兄弟节点 → None（?next_sibling 求值为 null）
    fn next_sibling(&self) -> Result<Option<TModel>> {
        Ok(None)
    }
    /// Java TemplateNodeModelEx.getPreviousSibling（BuiltInsForNodes.previousSiblingBI）
    fn previous_sibling(&self) -> Result<Option<TModel>> {
        Ok(None)
    }
}

/// 节点哈希访问 —— 对应 Java `NodeModel` 的 `TemplateHashModel` 角色
/// （`doc.foo` / `doc['//x']` / `doc.@@markup` 等节点键访问）。与普通哈希不同，
/// get 需要 `Environment` 以解析当前命名空间的 `ns_prefixes`（Java 用线程局部
/// Environment.getCurrentEnvironment，Rust 显式传参；docs/06）。
pub trait NodeHashModel {
    /// 键查找：`@@` 特殊键 / 子元素名 / XPath 子集查询。返回 None = 键缺失
    /// （Java SimpleHash.get 返回 null 的语义，由使用点决定报错/回退）。
    fn get(&self, env: &mut Environment, key: &str) -> Result<Option<TModel>>;
}

/// 自定义指令 body 回插（对应 TemplateDirectiveBody）
pub trait TemplateDirectiveBody {
    fn render(&self, env: &mut Environment) -> Result<()>;
}

pub trait TemplateDirectiveModel {
    fn execute(
        &self,
        env: &mut Environment,
        params: &HashMap<String, TModel>,
        loop_vars: &mut [TModel],
        body: Option<&dyn TemplateDirectiveBody>,
    ) -> Result<()>;
}

/// 范围模型规格 —— 对应 Java `RangeModel`（`seq[range]`/`"str"[range]` 的切片键
/// 类型判定，Java DynamicKeyName `instanceof RangeModel`；有界与无界共用）
#[derive(Clone, Copy)]
pub struct RangeSpec {
    pub start: i64,
    /// 有界范围长度（`2..5` = 4；`2..!5` = 3；`2..*3` = 3）
    pub count: usize,
    pub ascending: bool,
    /// `2..*` 无界（切片时以目标长度为准）
    pub unbounded: bool,
    /// 右自适应（Java `range.isRightAdaptive()`：`..*` size-limited 与无界；
    /// 切片时越界索引被裁剪而非报错，DynamicKeyName.java:244-262）
    pub adaptive: bool,
    /// Java `RangeModel.isAffectedByStringSlicingBug()`（BoundedRangeModel.java:40-41：
    /// = inclusiveEnd，仅 `a..b` 闭区间范围受影响；`..<`/`..!`/`..*`/无界不受影响）。
    /// 字符串降序切片且结果长为 2 时模拟旧版 bug 返回 "" 而非报错
    /// （DynamicKeyName.java:322-330："foo"[n .. n-1] 给 "" 而非错误）
    pub affected_by_string_slicing_bug: bool,
}

/// 变换模型 —— 对应 Java `TemplateTransformModel`（`<#transform>` 旧式指令目标
/// 与 `<@transformModel/>` 调用）。Java 语义（Environment.visitAndTransform
/// :495-543）：`getWriter(out, args)` 先产出变换自身输出（?interpret 即
/// env.include 解释模板），随后调用方把 body 写入变换 writer
/// （Interpret.TemplateProcessorModel.getWriter 返回透传 writer，body 原样输出；
/// StandardCompress 等在 close 时对 body 做压缩/转义）。
pub trait TemplateTransformModel {
    /// 变换自身输出（Java getWriter 阶段；v1 无 writer 对象，变换直接向 env 输出）
    fn transform(&self, env: &mut Environment) -> Result<()> {
        let _ = env;
        Ok(())
    }

    /// 带 body 的变换 —— 对应 Java `visitAndTransform` 的完整流程
    /// （Environment.java:495-543）：getWriter(out, args) → body 写入变换 writer →
    /// close 时变换输出。v1：捕获 body 渲染文本 → 变换 → emit。
    /// 返回 body 渲染的流控信号（`<#return>` 值 / 完成）。
    fn transform_with_body(
        &self,
        env: &mut Environment,
        params: &std::collections::HashMap<String, TModel>,
        body: &[crate::core::Element],
    ) -> Result<crate::core::environment::RunSignal> {
        let _ = params;
        self.transform(env)?;
        env.run(body)
    }
}

// ---------------------------------------------------------------------------
// TModel —— 角色槽位结构（对应 Java 单对象多接口实现）
// ---------------------------------------------------------------------------
