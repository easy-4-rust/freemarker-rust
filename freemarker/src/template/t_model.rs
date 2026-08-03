//! 模板模型（角色槽位结构）—— Rust 特有设计，对应 Java `TemplateModel` 多接口实现
//! （设计决策见 docs/02 §4.1：单对象多角色用 Option 槽位表达）
//! 构造辅助对应 Java SimpleObjectWrapper.wrap 的映射结果

use crate::error::{Result, TemplateError};
use crate::template::SimpleBoolean;
use crate::template::SimpleCollection;
use crate::template::SimpleDate;
use crate::template::SimpleHash;
use crate::template::SimpleScalar;
use crate::template::SimpleSequence;
use crate::template::{
    NodeHashModel, TemplateBooleanModel, TemplateCollectionModel, TemplateDateModel,
    TemplateDirectiveModel, TemplateHashModel, TemplateHashModelEx, TemplateMethodModelEx,
    TemplateNodeModel, TemplateNumberModel, TemplateScalarModel, TemplateSequenceModel,
    TemplateTransformModel,
};
use crate::value::{DateValue, TNumber};
use indexmap::IndexMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelKind {
    Nothing,
    Scalar,
    Number,
    Boolean,
    Date,
    Sequence,
    Collection,
    Hash,
    Method,
    Directive,
    Node,
    Markup,
    Wrapped,
    /// 宏/函数值（对应 Java `Macro` 对象；渲染引擎构造，docs/04 §6）
    Macro,
    /// lambda 值（对应 Java `LocalLambdaExpression` 求值结果；v1 仅存槽位，见 docs/04 §5）
    Lambda,
}

/// 数字角色：TNumber 内嵌（热路径构造零分配——`${i}` 循环输出、字面量求值、
/// 范围迭代值等）或外部 trait 对象（自定义 TemplateNumberModel）
#[derive(Clone)]
pub enum ModelNumber {
    /// 内嵌数值（SimpleNumber 角色的零分配等价物）
    Inline(TNumber),
    /// 外部数字模型（trait 对象角色）
    Dyn(Rc<dyn TemplateNumberModel>),
}

impl ModelNumber {
    /// 数值读数（Inline 直接取；Dyn 走 trait 的 as_number）
    pub fn as_number(&self) -> Result<TNumber> {
        match self {
            ModelNumber::Inline(n) => Ok(n.clone()),
            ModelNumber::Dyn(d) => d.as_number(),
        }
    }
}

/// 模板模型：每个槽位对应一个角色 trait（可多角色，如 Python 通用模型）

#[derive(Clone)]
pub struct TModel {
    pub scalar: Option<Rc<dyn TemplateScalarModel>>,
    pub number: Option<ModelNumber>,
    pub boolean: Option<Rc<dyn TemplateBooleanModel>>,
    pub date: Option<Rc<dyn TemplateDateModel>>,
    pub sequence: Option<Rc<dyn TemplateSequenceModel>>,
    pub collection: Option<Rc<dyn TemplateCollectionModel>>,
    pub hash: Option<Rc<dyn TemplateHashModel>>,
    pub hash_ex: Option<Rc<dyn TemplateHashModelEx>>,
    pub method: Option<Rc<dyn TemplateMethodModelEx>>,
    /// 方法模型的可索引性 —— 对应 Java BeansWrapper 的 `GenericMethodModel`
    /// 实现 `TemplateSequenceModel`（`?is_indexable` → true；`?is_sequence` 在
    /// ICI ≥ 2.3.24 仍排除——不可 #list）。自定义方法模型（TemplateMethodModelEx
    /// 匿名类）不实现 TemplateSequenceModel → false。
    pub method_indexable: bool,
    /// 集合的 Ex 角色 —— 对应 Java `TemplateCollectionModelEx`（?is_collection_ex）。
    /// SimpleSequence 实现 Ex；SimpleCollection 不实现（SimpleCollection.java:41-42）。
    pub collection_ex: bool,
    pub directive: Option<Rc<dyn TemplateDirectiveModel>>,
    /// 变换模型角色（对应 Java TemplateTransformModel；`<#transform>` 目标）
    pub transform: Option<Rc<dyn TemplateTransformModel>>,
    /// 范围模型标记（对应 Java `RangeModel`；`seq[range]` 切片键类型判定）
    pub range: Option<Rc<crate::template::RangeSpec>>,
    pub node: Option<Rc<dyn TemplateNodeModel>>,
    /// 节点哈希角色（对应 Java NodeModel 的 TemplateHashModel；`doc.foo`/`doc['//x']` 访问）。
    /// 与 `hash` 槽位分开：get 需要 Environment 解析 ns_prefixes（Java 线程局部
    /// Environment；Rust 显式传参，见 template_model.rs NodeHashModel 注释）。
    pub node_hash: Option<Rc<dyn NodeHashModel>>,
    /// 内部扩展槽位（渲染引擎专用，docs/04 §1）：承载宏/函数值、lambda、命名空间等
    /// Rust 特有设计（Java 中这些是 `TemplateModel` 实现类，Rust 侧统一用 `Any` 下沉）。
    pub internal: Option<Rc<dyn std::any::Any>>,
    /// 用户可见的类型描述（错误消息 `has evaluated to a {actual}` 使用）
    pub type_name: &'static str,
    pub kind: ModelKind,
}

impl fmt::Debug for TModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TModel")
            .field("type_name", &self.type_name)
            .finish()
    }
}

impl TModel {
    pub fn nothing() -> TModel {
        TModel {
            scalar: None,
            number: None,
            boolean: None,
            date: None,
            sequence: None,
            collection: None,
            hash: None,
            hash_ex: None,
            method: None,
            method_indexable: false,
            collection_ex: false,
            directive: None,
            transform: None,
            range: None,
            node: None,
            node_hash: None,
            internal: None,
            type_name: "nothing",
            kind: ModelKind::Nothing,
        }
    }

    /// `?if_exists` 的缺失结果 —— 对应 Java `TemplateModel.NOTHING`
    /// （GeneralPurposeNothing.java：全能空角色模型——scalar ""、boolean false、
    /// 空序列、空哈希、方法返回 null）。与真缺失（Java null → TModel::nothing()）
    /// 不同：任何使用点按"最合理方式"解释（插值 → ""、布尔 → false、
    /// 序列/哈希 → 空），不触发 InvalidReference。
    pub fn gpn() -> TModel {
        let empty_seq = Rc::new(SimpleSequence(Vec::new()));
        let empty_hash = Rc::new(SimpleHash(IndexMap::with_hasher(
            crate::utility::FnvBuildHasher::default(),
        )));
        TModel {
            scalar: Some(Rc::new(SimpleScalar(String::new()))),
            boolean: Some(Rc::new(SimpleBoolean(false))),
            // Java GeneralPurposeNothing 有 TemplateSequenceModel（size 0）但无
            // TemplateCollectionModel 角色（?is_collection → false）
            sequence: Some(empty_seq),
            hash: Some(empty_hash.clone()),
            hash_ex: Some(empty_hash),
            type_name: "nothing",
            kind: ModelKind::Scalar,
            ..Self::nothing()
        }
    }

    pub fn from_scalar(v: String) -> TModel {
        TModel {
            scalar: Some(Rc::new(SimpleScalar(v))),
            type_name: "string",
            kind: ModelKind::Scalar,
            ..Self::nothing()
        }
    }

    pub fn from_number(v: TNumber) -> TModel {
        // 内嵌零分配（SimpleNumber 角色仅在外部模型/显式 Dyn 构造时使用）
        TModel {
            number: Some(ModelNumber::Inline(v)),
            type_name: "number",
            kind: ModelKind::Number,
            ..Self::nothing()
        }
    }

    pub fn from_boolean(b: bool) -> TModel {
        TModel {
            boolean: Some(Rc::new(SimpleBoolean(b))),
            type_name: "boolean",
            kind: ModelKind::Boolean,
            ..Self::nothing()
        }
    }

    pub fn from_date(d: DateValue) -> TModel {
        TModel {
            date: Some(Rc::new(SimpleDate(d))),
            type_name: "date",
            kind: ModelKind::Date,
            ..Self::nothing()
        }
    }

    pub fn from_sequence(v: Vec<TModel>) -> TModel {
        // Java SimpleSequence（SimpleSequence.java:67）只实现 TemplateSequenceModel
        // ——无 TemplateCollectionModel 角色（?is_collection/?is_collection_ex → false）
        let s = Rc::new(SimpleSequence(v));
        TModel {
            sequence: Some(s),
            type_name: "sequence",
            kind: ModelKind::Sequence,
            ..Self::nothing()
        }
    }

    /// 一次性集合（仅 Collection 角色，无 Sequence 角色）—— 对应 Java `SimpleCollection`
    /// （只实现 TemplateCollectionModel，无 Ex：?is_collection_ex → false）。
    /// 注意与 from_sequence 的区别：只能枚举一次（iterator 消费语义，docs/06 §2）。
    pub fn from_collection(v: Vec<TModel>) -> TModel {
        TModel {
            collection: Some(Rc::new(SimpleCollection(v))),
            type_name: "collection",
            kind: ModelKind::Collection,
            ..Self::nothing()
        }
    }

    pub fn from_hash(v: IndexMap<String, TModel>) -> TModel {
        // 转换到 FNV 哈希（构造期 O(n) 一次性成本；成员访问热路径受益；
        // 插入序保持——indexmap 序由内部向量维持，与哈希器无关）
        let v: IndexMap<String, TModel, crate::utility::FnvBuildHasher> = v.into_iter().collect();
        let h = Rc::new(SimpleHash(v));
        let ex = h.clone();
        TModel {
            hash: Some(h),
            hash_ex: Some(ex),
            type_name: "hash",
            kind: ModelKind::Hash,
            ..Self::nothing()
        }
    }

    pub fn from_method(m: impl TemplateMethodModelEx + 'static) -> TModel {
        TModel {
            method: Some(Rc::new(m)),
            method_indexable: false,
            type_name: "method",
            kind: ModelKind::Method,
            ..Self::nothing()
        }
    }

    pub fn from_directive(d: impl TemplateDirectiveModel + 'static) -> TModel {
        TModel {
            directive: Some(Rc::new(d)),
            type_name: "directive",
            kind: ModelKind::Directive,
            ..Self::nothing()
        }
    }

    pub fn from_transform(t: impl TemplateTransformModel + 'static) -> TModel {
        TModel {
            transform: Some(Rc::new(t)),
            type_name: "transform",
            kind: ModelKind::Directive,
            ..Self::nothing()
        }
    }

    /// 从 XML 文本构造文档节点模型 —— 对应 Java `NodeModel.parse(InputSource)`
    /// （freemarker.ext.dom：SAX 解析 → simplify —— 移除注释/PI、合并相邻文本）。
    /// 返回"document"节点；子节点经 `?children` / 哈希键访问导航。
    pub fn from_xml_str(s: &str) -> Result<TModel> {
        crate::xml::parse_xml(s)
    }

    // ---- 角色判定（?is_* 内建）----
    pub fn is_scalar(&self) -> bool {
        self.scalar.is_some()
    }
    pub fn is_number(&self) -> bool {
        self.number.is_some()
    }
    pub fn is_boolean(&self) -> bool {
        self.boolean.is_some()
    }
    pub fn is_date(&self) -> bool {
        self.date.is_some()
    }
    pub fn is_sequence(&self) -> bool {
        self.sequence.is_some()
    }
    pub fn is_collection(&self) -> bool {
        self.collection.is_some()
    }
    pub fn is_hash(&self) -> bool {
        self.hash.is_some()
    }
    pub fn is_hash_ex(&self) -> bool {
        self.hash_ex.is_some()
    }
    pub fn is_method(&self) -> bool {
        self.method.is_some()
    }
    pub fn is_directive(&self) -> bool {
        // Java is_directiveBI（BuiltInsForMultipleTypes.java:308-314）：
        // TemplateTransformModel || Macro || TemplateDirectiveModel
        self.directive.is_some() || self.transform.is_some() || self.is_macro()
    }
    pub fn is_node(&self) -> bool {
        self.node.is_some()
    }
    pub fn is_nothing(&self) -> bool {
        self.kind == ModelKind::Nothing
    }

    /// 对应 Java `instanceof TemplateSequenceModel || TemplateCollectionModel`（?is_enumerable）
    pub fn is_enumerable(&self) -> bool {
        self.sequence.is_some() || self.collection.is_some()
    }

    /// 对应 Java `instanceof TemplateCollectionModelEx`（?is_collection_ex）——
    /// 由 collection_ex 标记承载（SimpleSequence 是 Ex，SimpleCollection 不是）
    pub fn is_collection_ex(&self) -> bool {
        self.collection_ex
    }

    /// 对应 Java `instanceof TemplateModelWithAPISupport`（?has_api；Rust 版不支持 → false）
    pub fn has_api(&self) -> bool {
        false
    }

    /// 对应 Java `instanceof TemplateTransformModel`（?is_transform；`?interpret` 产物）
    pub fn is_transform(&self) -> bool {
        self.transform.is_some()
    }

    /// 对应 Java `instanceof TemplateMarkupOutputModel`（?is_markup_output）
    pub fn is_markup_output(&self) -> bool {
        self.kind == ModelKind::Markup
    }

    /// 对应 Java `instanceof TemplateMacroModel`（?is_macro；宏/函数值由渲染引擎构造，
    /// 对应 Java `Macro` 对象——docs/04 §6）
    pub fn is_macro(&self) -> bool {
        self.kind == ModelKind::Macro
    }

    /// 对应 Java `LocalLambdaExpression` 求值结果（?is_callable 家族；v1 仅存槽位）
    pub fn is_lambda(&self) -> bool {
        self.kind == ModelKind::Lambda
    }

    /// 取内部扩展槽位（渲染引擎专用；对应 Java 中无法用 FTL 接口表达的模型值）
    pub fn internal<T: 'static>(&self) -> Option<Rc<T>> {
        self.internal
            .as_ref()
            .and_then(|any| any.clone().downcast::<T>().ok())
    }

    // ---- 角色取用（求值辅助；错误消息对齐 Java "For ... a X is required"）----
    pub fn get_scalar(&self) -> Result<String> {
        self.scalar
            .as_ref()
            .ok_or_else(|| TemplateError::type_mismatch("string", self.type_name))?
            .as_string()
    }
    pub fn get_number(&self) -> Result<TNumber> {
        self.number
            .as_ref()
            .ok_or_else(|| TemplateError::type_mismatch("number", self.type_name))?
            .as_number()
    }
    pub fn get_boolean(&self) -> Result<bool> {
        self.boolean
            .as_ref()
            .ok_or_else(|| TemplateError::type_mismatch("boolean", self.type_name))?
            .as_boolean()
    }
    pub fn get_date(&self) -> Result<DateValue> {
        self.date
            .as_ref()
            .ok_or_else(|| TemplateError::type_mismatch("date", self.type_name))?
            .as_date()
    }
    pub fn get_sequence(&self) -> Result<Rc<dyn TemplateSequenceModel>> {
        self.sequence
            .clone()
            .ok_or_else(|| TemplateError::type_mismatch("sequence", self.type_name))
    }
    pub fn get_hash(&self) -> Result<Rc<dyn TemplateHashModel>> {
        self.hash
            .clone()
            .ok_or_else(|| TemplateError::type_mismatch("hash", self.type_name))
    }
    pub fn get_method(&self) -> Result<Rc<dyn TemplateMethodModelEx>> {
        self.method
            .clone()
            .ok_or_else(|| TemplateError::type_mismatch("method", self.type_name))
    }
    pub fn get_directive(&self) -> Result<Rc<dyn TemplateDirectiveModel>> {
        self.directive
            .clone()
            .ok_or_else(|| TemplateError::type_mismatch("directive", self.type_name))
    }
    pub fn get_transform(&self) -> Result<Rc<dyn TemplateTransformModel>> {
        self.transform
            .clone()
            .ok_or_else(|| TemplateError::type_mismatch("transform", self.type_name))
    }

    /// 缺失语义：Nothing 视为 null（?has_content 判空、${x!} 抑制）
    pub fn is_null_or_missing(&self) -> bool {
        self.kind == ModelKind::Nothing
    }

    /// 空值判定（?has_content 内建：标量空串 / 空序列 / 空哈希）
    pub fn has_content(&self) -> Result<bool> {
        if self.is_null_or_missing() {
            return Ok(false);
        }
        if let Some(s) = &self.scalar {
            return Ok(!s.as_string()?.is_empty());
        }
        if let Some(seq) = &self.sequence {
            return Ok(seq.size()? > 0);
        }
        if let Some(h) = &self.hash {
            return Ok(!h.is_empty()?);
        }
        if let Some(c) = &self.collection {
            let mut it = c.iterator()?;
            return Ok(it.next().is_some());
        }
        Ok(true)
    }

    /// 布尔值求值（`<#if x>` 语义；非布尔抛 NonBooleanException）
    pub fn eval_boolean(&self) -> Result<bool> {
        if let Some(b) = &self.boolean {
            return b.as_boolean();
        }
        if self.scalar.is_some() {
            // classicCompatible 下字符串为真（v1 严格模式：报错）
            return Err(TemplateError::type_mismatch("boolean", self.type_name));
        }
        Err(TemplateError::type_mismatch("boolean", self.type_name))
    }
}

// ---------------------------------------------------------------------------
// Simple* 实现
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::TemplateDirectiveBody;
    use std::collections::HashMap;

    /// 各构造器 type_name 与 Java FTL 类型名对齐（docs/06 §2；
    /// Java ClassUtil.getFTLTypeDescription 的简化名：string/number/boolean/date/
    /// sequence/collection/hash/method/directive/node/nothing）
    #[test]
    fn constructor_type_names_match_ftl_types() {
        assert_eq!(TModel::nothing().type_name, "nothing");
        assert_eq!(TModel::from_scalar("s".into()).type_name, "string");
        assert_eq!(TModel::from_number(TNumber::Int(1)).type_name, "number");
        assert_eq!(TModel::from_boolean(true).type_name, "boolean");
        assert_eq!(
            TModel::from_date(DateValue {
                dt: chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").unwrap(),
                kind: crate::value::DateType::DateTime,
                is_sql: false,
            })
            .type_name,
            "date"
        );
        assert_eq!(TModel::from_sequence(vec![]).type_name, "sequence");
        assert_eq!(TModel::from_collection(vec![]).type_name, "collection");
        assert_eq!(TModel::from_hash(IndexMap::new()).type_name, "hash");
        assert_eq!(TModel::from_method(MethodStub).type_name, "method");
        assert_eq!(TModel::from_directive(DirectiveStub).type_name, "directive");
    }

    /// 一次性集合：仅 Collection 角色（不可作 Sequence 索引），可枚举
    #[test]
    fn collection_is_enumerable_but_not_sequence() {
        let m = TModel::from_collection(vec![
            TModel::from_scalar("a".into()),
            TModel::from_scalar("b".into()),
        ]);
        assert!(m.is_collection());
        assert!(!m.is_sequence());
        assert!(m.is_enumerable());
        assert_eq!(m.kind, ModelKind::Collection);
        let items: Vec<_> = m
            .collection
            .as_ref()
            .unwrap()
            .iterator()
            .unwrap()
            .map(|r| r.unwrap().get_scalar().unwrap())
            .collect();
        assert_eq!(items, vec!["a", "b"]);
    }

    struct MethodStub;
    impl TemplateMethodModelEx for MethodStub {
        fn exec(&self, _args: Vec<TModel>) -> Result<TModel> {
            Ok(TModel::nothing())
        }
    }

    struct DirectiveStub;
    impl TemplateDirectiveModel for DirectiveStub {
        fn execute(
            &self,
            _env: &mut crate::core::Environment,
            _params: &HashMap<String, TModel>,
            _loop_vars: &mut [TModel],
            _body: Option<&dyn TemplateDirectiveBody>,
        ) -> Result<()> {
            Ok(())
        }
    }
}
