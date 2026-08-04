//! 对应 Java `freemarker.core` 包：解析器产物、渲染引擎、算术引擎、设置项

mod arithmetic_engine;
mod configurable;
pub(crate) mod environment;
pub(crate) mod eval;
mod exec;
mod expression;
mod get_optional_template_method;
mod hash_literal;
mod macro_def;
mod output_format;
mod template_class_resolver;
mod template_configuration;
mod template_element;

pub use arithmetic_engine::{ArithmeticEngine, BigDecimalEngine};
pub use configurable::{canonical_setting_key, java_time_zone_id, Settings, TzSetting};
pub use environment::{render, Environment, MacroValue, Namespace};
pub use eval::{compare_models, eval, CmpOp};
pub use exec::{exec, ExecOutcome};
pub use expression::{BuiltinVar, Expr, ExprKind, RangeKind, StrPart};
pub use macro_def::{MacroDef, MacroParam};
pub use output_format::{AutoEscaping, OutputFormatKind};
pub use template_class_resolver::{NewBuiltinClassResolver, OptInClassResolver};
pub use template_configuration::TemplateConfiguration;
pub use template_element::{AssignOp, CallTarget, CaseDef, Element, ElementKind};
