//! 对象包装与解包 —— 对应 Java `freemarker.template.ObjectWrapperAndUnwrapper`
//! （Java :92 行：ObjectWrapper + unwrap 双向能力）
//! v1：unwrap 已并入主 trait `ObjectWrapper`（object_wrapper.rs 头注释——
//! Java 的 ObjectWrapperAndUnwrapper.unwrap 在 Rust 为 ObjectWrapper 的
//! 必需方法）→ 本类型为标记扩展（对应 Java 子接口形态）

use crate::template::ObjectWrapper;

/// 对象包装与解包（对应 ObjectWrapperAndUnwrapper.java；
/// v1 标记扩展——unwrap 见 ObjectWrapper::unwrap）
pub trait ObjectWrapperAndUnwrapper: ObjectWrapper {}
