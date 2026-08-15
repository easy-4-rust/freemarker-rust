//! ?interpret 内建 —— 对应 Java `freemarker.core.Interpret`
//! （OutputFormatBoundBuiltIn 子类；动态编译 FTL 源码为模板并返回
//!  TemplateTransformModel；参数1=源码，参数2=可选模板名）

/// 对应 Java `Interpret`（BuiltIn::interpret 分支承载语义）
#[allow(dead_code)]
pub(crate) struct Interpret;
