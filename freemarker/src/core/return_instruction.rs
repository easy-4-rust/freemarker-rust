//! 返回指令 —— 对应 Java `freemarker.core.ReturnInstruction`
//! （accept :35-51：设置返回值后抛 Return.INSTANCE）

use crate::core::environment::expr_desc;
use crate::core::eval::eval;
use crate::core::exec::ExecOutcome;
use crate::core::Expr;
use crate::error::{Result, TemplateError};

/// `<#return>` / `<#return expr>` 指令（对应 ReturnInstruction.java）
pub struct ReturnInstruction {
    pub expr: Option<Expr>,
}

impl ReturnInstruction {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(expr: Option<Expr>) -> Self {
        ReturnInstruction { expr }
    }

    /// 执行（Java accept）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        // Java ReturnInstruction.java:35-51：设置返回值后抛 Return.INSTANCE；
        // 记录发起时的宏帧深度（nested body 执行时 exec_nested 已弹出被调宏帧，
        // 栈顶即 return 的归属宏——Java Return 异常携带发起 Macro.Context）
        env.return_depth = Some(env.macro_frames.len());
        let v = match &self.expr {
            Some(e) => {
                let m = eval(env, e)?;
                if m.is_nothing() {
                    return Err(TemplateError::invalid_reference(expr_desc(e)));
                }
                Some(m)
            }
            None => None,
        };
        Ok(ExecOutcome::ReturnValue(v))
    }
}
