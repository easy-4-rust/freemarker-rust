//! 包含指令 —— 对应 Java `freemarker.core.Include`
//! （accept :25-100 + :138-153：parse/encoding/ignore_missing 属性）

use crate::core::eval;
use crate::core::exec::{eval_to_string, get_yes_no, ExecOutcome};
use crate::core::Expr;
use crate::error::Result;

/// `<#include path args...>`（对应 Include.java）
pub struct Include {
    pub path: Expr,
    pub attrs: Vec<(String, Expr)>,
}

impl Include {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(path: Expr, attrs: Vec<(String, Expr)>) -> Self {
        Include { path, attrs }
    }

    /// 执行（Java accept）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let name = eval_to_string(env, &self.path)?;
        let mut parse = true;
        let mut ignore_missing = false;
        let mut encoding: Option<String> = None;
        for (an, av) in &self.attrs {
            match an.to_ascii_lowercase().as_str() {
                "parse" => {
                    // Java Include.accept :142-151：scalar → getYesNo（legacy 字符串），
                    // 否则 modelToBoolean
                    let m = eval::eval(env, av)?;
                    if m.scalar.is_some() {
                        parse =
                            get_yes_no(av, &crate::core::environment::model_to_string(env, &m)?)?;
                    } else {
                        parse = eval::model_to_boolean(env, &m)?;
                    }
                }
                "encoding" => {
                    // Java Include.accept :131-135：运行时求值（求值错误照常传播）
                    encoding = Some(eval_to_string(env, av)?);
                }
                "ignore_missing" => {
                    let m = eval::eval(env, av)?;
                    ignore_missing = eval::model_to_boolean(env, &m)?;
                }
                _ => unreachable!("解析器已校验 include 参数名"),
            }
        }
        env.include_named(&name, parse, ignore_missing, encoding)?;
        Ok(ExecOutcome::Done)
    }
}
