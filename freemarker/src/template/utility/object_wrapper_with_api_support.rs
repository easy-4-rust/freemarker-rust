//! 带 API 支持的对象包装器 —— 对应 Java
//! `freemarker.template.utility.ObjectWrapperWithAPISupport`
//! （包装器可为模型提供 ?api 视图；Java 反射表面——v1 由包装方经
//! TemplateApiSupport 提供）

use crate::template::ObjectWrapper;

/// 带 API 支持的对象包装器（对应 ObjectWrapperWithAPISupport.java）
pub trait ObjectWrapperWithAPISupport: ObjectWrapper {}
