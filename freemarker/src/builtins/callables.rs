//! 柯里化内建 —— 对应 Java `BuiltInsForCallables.java`（with_args/with_args_last 基础版）
//!
//! 语义要点（Java 对照）：`?with_args(a, b)` 预绑定参数，返回部分应用（再调用时前置参数）；
//! v1 仅支持目标为方法模型（TemplateMethodModelEx）的基础版——宏/函数的 `?with_args`
//! （Java Macro.WithArgsCallable）属 P4。

use crate::core::eval_util::check_arg_count;
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateMethodModelEx};
use std::rc::Rc;

/// 求值全部参数（Java 预绑定：参数在 ?with_args 求值时求值）
fn eval_args(env: &mut Environment, args: Option<&[Expr]>) -> Result<Vec<TModel>> {
    let mut out = Vec::new();
    if let Some(as_) = args {
        for a in as_ {
            out.push(crate::core::eval::eval(env, a)?);
        }
    }
    Ok(out)
}

/// ?with_args —— Java with_argsBI：目标方法预绑定参数
pub fn with_args(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("with_args", args, 1, usize::MAX)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    let method = m
        .method
        .clone()
        .ok_or_else(|| TemplateError::misc("?with_args is only supported on methods in v1"))?;
    let bound = eval_args(env, args)?;
    Ok(Some(TModel::from_method(WithArgsMethod {
        method,
        bound,
        at_end: false,
    })))
}

/// ?with_args_last —— Java with_args_lastBI：参数追加在尾部
pub fn with_args_last(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("with_args_last", args, 1, usize::MAX)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    let method = m
        .method
        .clone()
        .ok_or_else(|| TemplateError::misc("?with_args_last is only supported on methods in v1"))?;
    let bound = eval_args(env, args)?;
    Ok(Some(TModel::from_method(WithArgsMethod {
        method,
        bound,
        at_end: true,
    })))
}

/// 部分应用方法（Java with_argsBI.WithArgsMethod：exec = bound + args / args + bound）
struct WithArgsMethod {
    method: Rc<dyn TemplateMethodModelEx>,
    bound: Vec<TModel>,
    at_end: bool,
}

impl TemplateMethodModelEx for WithArgsMethod {
    fn exec(&self, env: &mut Environment, args: Vec<TModel>) -> Result<TModel> {
        let mut all = Vec::with_capacity(args.len() + self.bound.len());
        if self.at_end {
            all.extend(args);
            all.extend(self.bound.iter().cloned());
        } else {
            all.extend(self.bound.iter().cloned());
            all.extend(args);
        }
        self.method.exec(env, all)
    }
}
