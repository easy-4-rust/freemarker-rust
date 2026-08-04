//! 富对象包装器 —— 对应 Java
//! `freemarker.template.utility.RichObjectWrapper`
//! （ObjectWrapper 扩展：JavaBean 支持；v1 依附决策 1（JVM 反射 NA））

use crate::template::ObjectWrapper;

/// 富对象包装器（对应 RichObjectWrapper.java；JVM 反射决策 1 → NA）
pub trait RichObjectWrapper: ObjectWrapper {}
