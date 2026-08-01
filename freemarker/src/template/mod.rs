//! 对应 Java `freemarker.template` 包：数据模型、配置、模板

pub(crate) mod configuration;
mod dyn_value;
mod object_wrapper;
mod simple_boolean;
mod simple_collection;
mod simple_date;
mod simple_hash;
mod simple_number;
mod simple_object_wrapper;
mod simple_scalar;
mod simple_sequence;
mod t_model;
#[allow(clippy::module_inception)] // Template.java → template/template.rs（一文件一对象约定）
mod template;
mod template_model;
pub(crate) mod utility_transforms;
mod version;

pub use configuration::Configuration;
pub use dyn_value::DynValue;
pub use object_wrapper::ObjectWrapper;
pub use simple_boolean::SimpleBoolean;
pub use simple_collection::SimpleCollection;
pub use simple_date::SimpleDate;
pub use simple_hash::SimpleHash;
pub use simple_number::SimpleNumber;
pub use simple_object_wrapper::{SimpleObjectWrapper, SIMPLE_WRAPPER};
pub use simple_scalar::SimpleScalar;
pub use simple_sequence::SimpleSequence;
pub use t_model::{ModelKind, TModel};
pub use template::Template;
pub use template_model::{
    RangeSpec, TemplateBooleanModel, TemplateCollectionModel, TemplateDateModel,
    TemplateDirectiveBody, TemplateDirectiveModel, TemplateHashModel, TemplateHashModelEx,
    TemplateMethodModelEx, TemplateNodeModel, TemplateNumberModel, TemplateScalarModel,
    TemplateSequenceModel, TemplateTransformModel,
};
pub use version::Version;
