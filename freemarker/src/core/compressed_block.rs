//! 压缩块 —— 对应 Java `freemarker.core.CompressedBlock`
//! （块输出空白压缩——Java StandardCompress 正则语义 v1 简化为
//! template::utility::standard_compress_text）

use crate::core::environment::RunSignal;
use crate::core::exec::ExecOutcome;
use crate::core::Element;
use crate::error::Result;

/// `<#compress>` 块（对应 CompressedBlock.java）
pub struct CompressedBlock {
    pub body: Vec<Element>,
}

impl CompressedBlock {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(body: Vec<Element>) -> Self {
        CompressedBlock { body }
    }

    /// 执行（Java accept → StandardCompress.INSTANCE 变换）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let captured = env.capture(|env| env.run(&self.body))?;
        match captured.0 {
            RunSignal::Returned(v) => Ok(ExecOutcome::ReturnValue(v)),
            RunSignal::Completed => {
                env.emit(&crate::template::utility::standard_compress_text(
                    &captured.1,
                    false,
                ))?;
                Ok(ExecOutcome::Done)
            }
        }
    }
}
