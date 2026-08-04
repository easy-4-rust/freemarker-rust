//! 对应 Java `freemarker.template` 包：数据模型、配置、模板
//! （`template/utility/` 对应 Java `freemarker.template.utility` 包——去包名映射）

mod adapter_template_model;
mod attempt_exception_reporter;
pub(crate) mod configuration;
mod default_array_adapter;
mod default_enumeration_adapter;
mod default_iterable_adapter;
mod default_iterator_adapter;
mod default_list_adapter;
mod default_map_adapter;
mod default_non_list_collection_adapter;
mod dyn_value;
mod empty_map;
mod false_template_boolean_model;
mod general_purpose_nothing;
mod iterator_to_template_model_iterator_adapter;
mod localized_string;
mod logging_attempt_exception_reporter;
mod malformed_template_name_exception;
mod map_key_value_pair_iterator;
mod method_call_aware_template_hash_model;
mod object_wrapper;
mod object_wrapper_and_unwrapper;
mod resource_bundle_localized_string;
mod serializable_template_boolean_model;
mod simple_boolean;
mod simple_collection;
mod simple_date;
mod simple_hash;
mod simple_list;
mod simple_number;
mod simple_object_wrapper;
mod simple_scalar;
mod simple_sequence;
mod t_model;
#[allow(clippy::module_inception)] // Template.java → template/template.rs（一文件一对象约定）
mod template;
mod template_exception;
mod template_model;
mod template_model_adapter;
mod template_model_exception;
mod template_model_iterator;
mod template_model_list_sequence;
mod template_not_found_exception;
mod transform_control;
mod true_template_boolean_model;
/// 对应 Java `freemarker.template.utility` 包（原顶层 utility/ 迁入）
pub mod utility;
pub(crate) mod utility_transforms;
mod version;
mod wrapping_template_model;

pub use adapter_template_model::AdapterTemplateModel;
pub use attempt_exception_reporter::AttemptExceptionReporter;
pub use configuration::Configuration;
pub use default_array_adapter::DefaultArrayAdapter;
pub use default_enumeration_adapter::DefaultEnumerationAdapter;
pub use default_iterable_adapter::DefaultIterableAdapter;
pub use default_iterator_adapter::DefaultIteratorAdapter;
pub use default_list_adapter::DefaultListAdapter;
pub use default_map_adapter::DefaultMapAdapter;
pub use default_non_list_collection_adapter::DefaultNonListCollectionAdapter;
pub use dyn_value::DynValue;
pub use empty_map::EmptyMap;
pub use false_template_boolean_model::FalseTemplateBooleanModel;
pub use general_purpose_nothing::GeneralPurposeNothing;
pub use iterator_to_template_model_iterator_adapter::IteratorToTemplateModelIteratorAdapter;
pub use localized_string::LocalizedString;
pub use logging_attempt_exception_reporter::LoggingAttemptExceptionReporter;
pub use malformed_template_name_exception::MalformedTemplateNameException;
pub use map_key_value_pair_iterator::{KeyValuePair, MapKeyValuePairIterator};
pub use method_call_aware_template_hash_model::MethodCallAwareTemplateHashModel;
pub use object_wrapper::ObjectWrapper;
pub use object_wrapper_and_unwrapper::ObjectWrapperAndUnwrapper;
pub use resource_bundle_localized_string::ResourceBundleLocalizedString;
pub use serializable_template_boolean_model::SerializableTemplateBooleanModel;
pub use simple_boolean::SimpleBoolean;
pub use simple_collection::SimpleCollection;
pub use simple_date::SimpleDate;
pub use simple_hash::SimpleHash;
pub use simple_list::SimpleList;
pub use simple_number::SimpleNumber;
pub use simple_object_wrapper::{SimpleObjectWrapper, SIMPLE_WRAPPER};
pub use simple_scalar::SimpleScalar;
pub use simple_sequence::SimpleSequence;
pub use t_model::{ModelKind, ModelNumber, TModel};
pub use template::Template;
pub use template_exception::TemplateException;
pub use template_model::{
    NodeHashModel, RangeSpec, TemplateApiSupport, TemplateBooleanModel, TemplateCollectionModel,
    TemplateDateModel, TemplateDirectiveBody, TemplateDirectiveModel, TemplateHashModel,
    TemplateHashModelEx, TemplateHashModelEx2, TemplateMethodModelEx, TemplateNodeModel,
    TemplateNumberModel, TemplateScalarModel, TemplateSequenceModel, TemplateTransformModel,
};
pub use template_model_adapter::TemplateModelAdapter;
pub use template_model_exception::TemplateModelException;
pub use template_model_iterator::TemplateModelIterator;
pub use template_model_list_sequence::TemplateModelListSequence;
pub use template_not_found_exception::TemplateNotFoundException;
pub use transform_control::{TransformControl, END, REPAINT, START};
pub use true_template_boolean_model::TrueTemplateBooleanModel;
pub use version::Version;
pub use wrapping_template_model::WrappingTemplateModel;
