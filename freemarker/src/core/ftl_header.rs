//! FTL 头 —— 对应 Java `freemarker.core.FTLHeader`
//! （encoding 属性解析期已处理，渲染期无操作）

use crate::core::exec::ExecOutcome;
use crate::error::Result;

/// `[<#ftl encoding="...">]` 指令（对应 FTLHeader.java）
pub struct FtlHeader {
    /// encoding 属性（Java FTLHeader 持有；解析期已处理）
    #[allow(dead_code)]
    pub encoding: Option<String>,
}

impl FtlHeader {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(encoding: Option<String>) -> Self {
        FtlHeader { encoding }
    }

    /// 执行（Java accept：无操作——解析期已处理）
    pub(crate) fn exec(&self, _env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        Ok(ExecOutcome::Done)
    }
}
