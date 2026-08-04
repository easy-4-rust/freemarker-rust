//! 柯里化内建 —— 对应 Java `BuiltInsForCallables.java`（with_args/with_args_last）
//!
//! 语义要点（Java 对照）：`x?with_args(arg)` 按 Java `AbstractWithArgsBI._eval`
//! （BuiltInsForCallables.java:47-63）分派目标类型：
//! - 宏/函数 → BIMethodForMacroAndFunction（:65-94）：实参为扩展哈希 → 按名预绑定
//!   （函数 + 哈希 → 报错）、序列 → 按位预绑定；产物为带 `Macro.WithArgs` 的新宏值
//!   （`new Macro(macro, withArgs)`，Macro.java:98-104），宏体参数在调用时合并
//!   （Environment.setMacroContextLocalsFromArguments :919-1094，见
//!   environment.rs bind_macro_args）；
//! - 方法 → BIMethodForMethod（:96-185）：序列 → 部分应用方法（调用时拼接参数）；
//!   哈希 → 报错 "When applied on a method, ?with_args can't have a hash argument..."；
//! - 指令 → BIMethodForDirective（:187-254）：v1 未实现（保持基础版错误）。

use crate::core::environment::{MacroValue, WithArgs, WithArgsKind};
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::{TModel, TemplateMethodModelEx};
use std::rc::Rc;

/// 求值单个预绑定实参（Java BIMethodFor*.exec 的 checkMethodArgCount(args.size(), 1)
/// —— `x?with_args(arg)` 恰 1 个实参；v1 语法差异：实参即内建参数，此处检查个数）
fn eval_bound_arg(env: &mut Environment, bi: &str, args: Option<&[Expr]>) -> Result<TModel> {
    let argc = args.map_or(0, |a| a.len());
    if argc != 1 {
        return Err(TemplateError::misc(format!(
            "?{bi}(...) expects 1 argument but has received {}.",
            if argc == 0 {
                "none".to_string()
            } else {
                argc.to_string()
            }
        )));
    }
    let a = &args.unwrap()[0];
    crate::core::eval::eval(env, a)
}

/// 实参类型错误 —— Java `_MessageUtil.newMethodArgMustBeExtendedHashOrSequnceException`
/// （_MessageUtil.java:256-258 → newMethodArgUnexpectedTypeException）：
/// `?with_args(...) argument #1 must be an extended hash or sequence, but was {type}.`
fn must_be_extended_hash_or_sequence(bi: &str, m: &TModel) -> TemplateError {
    let (article, t) = ftl_type_desc_article(m);
    TemplateError::misc(format!(
        "?{bi}(...) argument #1 must be an extended hash or sequence, but was {article} {t}."
    ))
}

/// 类型描述（Java `_DelayedAOrAn`：a/an + 类型名；nothing → "a Null"）
fn ftl_type_desc_article(m: &TModel) -> (&'static str, &'static str) {
    if m.is_nothing() {
        return ("a", "Null");
    }
    let t = m.type_name;
    if t.starts_with(['a', 'e', 'i', 'o', 'u']) {
        ("an", t)
    } else {
        ("a", t)
    }
}

/// ?with_args —— Java with_argsBI（orderLast=false）
pub fn with_args(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    with_args_common(env, target, args, false)
}

/// ?with_args_last —— Java with_args_lastBI（orderLast=true）
pub fn with_args_last(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    with_args_common(env, target, args, true)
}

fn with_args_common(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
    order_last: bool,
) -> Result<Option<TModel>> {
    let bi = if order_last {
        "with_args_last"
    } else {
        "with_args"
    };
    let m = crate::core::eval::eval(env, target)?;
    if m.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    // Java AbstractWithArgsBI._eval（BuiltInsForCallables.java:47-63）：
    // Macro → BIMethodForMacroAndFunction；TemplateDirectiveModel →
    // BIMethodForDirective；TemplateMethodModel → BIMethodForMethod；
    // 其余 → UnexpectedTypeException("macro, function, directive, or method")
    // （v1 语法差异：Java 的实参经方法调用 `x?withArgs(arg)` 传入 BIMethod.exec，
    // Rust 解析器把 `(...)` 视为内建参数 —— 本函数直接求值实参并产出最终可调用值）
    if let Some(mv) = env.as_macro(&m) {
        let arg = eval_bound_arg(env, bi, args)?;
        let kind = if arg.is_sequence() {
            let seq = arg.get_sequence()?;
            let mut vals = Vec::with_capacity(seq.size()?);
            for i in 0..seq.size()? {
                vals.push(seq.get(i)?);
            }
            WithArgsKind::ByPosition(vals)
        } else if arg.is_hash_ex() {
            if mv.def.is_function {
                // Java :82-85：函数 + 哈希 → 报错
                return Err(TemplateError::misc(format!(
                    "When applied on a function, ?{bi} can't have a hash argument. Use a sequence argument."
                )));
            }
            let mut map = indexmap::IndexMap::new();
            for (k, v) in arg.hash_ex.as_ref().unwrap().entries()? {
                map.insert(k, v);
            }
            WithArgsKind::ByName(map)
        } else {
            return Err(must_be_extended_hash_or_sequence(bi, &arg));
        };
        // Java `new Macro(macroOrFunction, withArgs)`（Macro.java:98-104）——
        // 同定义/同命名空间的宏值 + 预绑定参数
        return Ok(Some(crate::core::environment::macro_model(Rc::new(
            MacroValue {
                def: mv.def.clone(),
                ns: mv.ns.clone(),
                with_args: Some(WithArgs { kind, order_last }),
            },
        ))));
    }
    if m.is_directive() {
        // Java BIMethodForDirective（:187-254）：指令的 ?with_args 按名合并参数
        // （v1 未实现，保持基础版错误——文档化差异，P4 项）
        return Err(TemplateError::misc(format!(
            "?{bi} is only supported on methods in v1"
        )));
    }
    if let Some(method) = &m.method {
        // Java BIMethodForMethod.exec（:96-185）：实参须为序列（哈希/其他 → 报错）
        let arg = eval_bound_arg(env, bi, args)?;
        if arg.is_sequence() {
            let seq = arg.get_sequence()?;
            let mut bound = Vec::with_capacity(seq.size()?);
            for i in 0..seq.size()? {
                bound.push(seq.get(i)?);
            }
            return Ok(Some(TModel::from_method(WithArgsMethod {
                method: method.clone(),
                bound,
                at_end: order_last,
            })));
        }
        if arg.is_hash_ex() {
            return Err(TemplateError::misc(format!(
                "When applied on a method, ?{bi} can't have a hash argument. Use a sequence argument."
            )));
        }
        return Err(must_be_extended_hash_or_sequence(bi, &arg));
    }
    Err(TemplateError::misc(format!(
        "The value of {} is not a macro, function, directive, or method (it's a {})",
        crate::core::environment::expr_desc(target),
        m.type_name
    )))
}

/// 方法目标的部分应用方法（Java withArgsBI.WithArgsMethod：exec = bound + args /
/// args + bound）
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
