//! 库加载指令 —— 对应 Java `freemarker.core.LibraryLoad`
//! （`<#import>`；accept :26-47 → env.importLib（:3232-3290））

use crate::core::exec::{eval_to_string, ExecOutcome};
use crate::core::Expr;
use crate::error::Result;

/// `<#import path as ns>`（对应 LibraryLoad.java）
pub struct LibraryLoad {
    pub path: Expr,
    pub ns: String,
}

impl LibraryLoad {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(path: Expr, ns: String) -> Self {
        LibraryLoad { path, ns }
    }

    /// 执行（Java accept → env.importLib）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let name = eval_to_string(env, &self.path)?;
        env.import_lib(&name, &self.ns)?;
        Ok(ExecOutcome::Done)
    }
}
