//! 数据模型角色 trait 家族 —— 对应 Java `freemarker.template.TemplateModel` 接口家族
//! （接口→trait 映射见 docs/06 §1；全部 object-safe，支持 `Rc<dyn>` 槽位）
//! 一文件一 Java 对象：各 trait 独立文件（template_model/ 子目录），本文件仅
//! 聚合声明 + re-export（聚合文件不承载对象定义，等同 mod.rs 角色）

mod node_hash_model;
mod range_model;
mod template_boolean_model;
mod template_collection_model;
mod template_date_model;
mod template_directive_body;
mod template_directive_model;
mod template_hash_model;
mod template_hash_model_ex;
mod template_hash_model_ex2;
mod template_method_model_ex;
mod template_model_with_api_support;
mod template_node_model;
mod template_number_model;
mod template_scalar_model;
mod template_sequence_model;
mod template_transform_model;

pub use node_hash_model::NodeHashModel;
pub use range_model::RangeSpec;
pub use template_boolean_model::TemplateBooleanModel;
pub use template_collection_model::TemplateCollectionModel;
pub use template_date_model::TemplateDateModel;
pub use template_directive_body::TemplateDirectiveBody;
pub use template_directive_model::TemplateDirectiveModel;
pub use template_hash_model::TemplateHashModel;
pub use template_hash_model_ex::TemplateHashModelEx;
pub use template_hash_model_ex2::TemplateHashModelEx2;
pub use template_method_model_ex::TemplateMethodModelEx;
pub use template_model_with_api_support::TemplateApiSupport;
pub use template_node_model::TemplateNodeModel;
pub use template_number_model::TemplateNumberModel;
pub use template_scalar_model::TemplateScalarModel;
pub use template_sequence_model::TemplateSequenceModel;
pub use template_transform_model::TemplateTransformModel;
