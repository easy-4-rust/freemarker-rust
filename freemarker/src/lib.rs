//! freemarker —— Apache FreeMarker 语义兼容的 Rust 模板引擎（freemarker-core 迁移）
//!
//! 迁移基线：apache/freemarker 2.3-gae 分支 commit 7926e97（incompatibleImprovements 2.3.34）。
//! 详细设计见 `docs/`（02 架构、03 解析器、04 渲染引擎、05 内建函数、06 数据模型、07 配置缓存）。
//!
//! 模块布局对齐 Java 包：`freemarker.core` → core/、`freemarker.template` → template/、
//! `freemarker.cache` → cache/、`freemarker.template.utility` → utility/。

pub mod builtins;
pub mod cache;
pub mod core;
pub mod error;
pub mod parser;
pub mod span;
pub mod template;
pub mod utility;
pub mod value;
pub mod xml;

pub use core::{eval, exec, Environment};
pub use error::{Result, TemplateError};
pub use template::{Configuration, TModel, Template};
