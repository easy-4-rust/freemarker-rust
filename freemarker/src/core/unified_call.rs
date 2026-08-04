//! 统一调用指令 —— 对应 Java `freemarker.core.UnifiedCall`
//! （accept :66-103：宏 → invokeMacro；用户指令 → execute；变换 → visitAndTransform）

use crate::core::environment::{expr_desc, RunSignal};
use crate::core::eval;
use crate::core::exec::ExecOutcome;
use crate::core::{CallTarget, Element};
use crate::error::{Result, TemplateError};
use crate::span::Span;
use crate::template::{TModel, TemplateDirectiveBody};
use std::collections::HashMap;
use std::rc::Rc;

/// `<@callee args>body</@callee>`（对应 UnifiedCall.java）
pub struct UnifiedCall {
    pub callee: CallTarget,
    pub args: Vec<(String, crate::core::Expr)>,
    pub body: Option<Vec<Element>>,
    /// body 参数名（<@m ; a, b>；对应 Java UnifiedCall.bodyParameters 列表）
    pub body_params: Vec<String>,
    pub span: Span,
}

impl UnifiedCall {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        callee: CallTarget,
        args: Vec<(String, crate::core::Expr)>,
        body: Option<Vec<Element>>,
        body_params: Vec<String>,
        span: Span,
    ) -> Self {
        UnifiedCall {
            callee,
            args,
            body,
            body_params,
            span,
        }
    }

    /// 执行（Java accept）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_call_impl(
            env,
            &self.callee,
            &self.args,
            self.body.clone(),
            self.body_params.clone(),
            self.span,
        )
    }
}

pub(crate) fn exec_call_impl(
    env: &mut crate::core::Environment,
    callee: &CallTarget,
    args: &[(String, crate::core::Expr)],
    body: Option<Vec<Element>>,
    body_params: Vec<String>,
    call_span: crate::span::Span,
) -> Result<ExecOutcome> {
    // call_name 仅在报错时构造（热路径 `@m/` 调用避免每次 String 克隆）
    let tm = match callee {
        CallTarget::Name(name) => {
            // 宏快路径：解析链直接取宏值（热路径 `<@m/>` 跳过 macro_model TModel
            // 构造与 downcast；名字解析为其他值/未找到时回退 get_variable）
            if let Some(mv) = env.get_macro(name) {
                return call_macro(env, &mv, args, body, body_params);
            }
            match env.get_variable(name) {
                Ok(tm) => tm,
                // Java UnifiedCall.accept：callee 表达式（Ident）的起始位置——
                // `@notdefmacro` 的 blame `==> notdefmacro  [in template ... at line 1,
                // column 3]`（jar 实测 missing_macro 基线；名称始于 `@` 之后，即
                // 元素起始列 + 2）
                Err(e) => {
                    return Err(crate::core::environment::attach_location(
                        e,
                        &env.current_template_name,
                        crate::span::Span::new(call_span.line, call_span.col.saturating_add(2)),
                    ))
                }
            }
        }
        CallTarget::Namespaced { ns, name } => {
            let nsm = env.get_variable(ns)?;
            match env.as_namespace(&nsm) {
                Some(nsr) => nsr
                    .get_member(name)
                    .ok_or_else(|| TemplateError::invalid_reference(format!("{ns}.{name}")))?,
                // Java：UnifiedCall 的 callee 是普通表达式（UnifiedCall.accept :66-67
                // nameExp.eval(env)），无 namespace 强制——ns 非 namespace 时按 Dot
                // 求值（hash 成员可为 directive/transform：compress.ftl
                // `<@utility.standardCompress>`）；`<#import>` 产生的 namespace 走
                // 上分支
                None => eval::eval(
                    env,
                    &crate::core::Expr::new(
                        crate::core::ExprKind::Dot {
                            target: Box::new(crate::core::Expr::new(
                                crate::core::ExprKind::Ident(ns.clone()),
                                crate::span::Span::new(0, 0),
                            )),
                            name: name.clone(),
                        },
                        crate::span::Span::new(0, 0),
                    ),
                )?,
            }
        }
        CallTarget::Expr(e) => eval::eval(env, e)?,
    };
    if let Some(mv) = env.as_macro(&tm) {
        return call_macro(env, &mv, args, body, body_params);
    }
    if let Some(d) = &tm.directive {
        // Java :84-95：env.visit(childBuffer, directiveModel, args, bodyParameterNames)
        // 参数：命名参数 → params；位置参数对指令模型忽略（Java EmptyMap 语义）
        let mut params = HashMap::new();
        for (k, e) in args {
            if !k.is_empty() {
                params.insert(k.clone(), eval::eval(env, e)?);
            }
        }
        // Java :432-465：outArgs 槽位按 body 参数名数量（bodyParameters 列表）
        let mut loop_vars: Vec<TModel> = vec![TModel::nothing(); body_params.len()];
        let has_body = body.is_some();
        let call_body = CallBody {
            elements: body.unwrap_or_default(),
        };
        let body_ref: Option<&dyn TemplateDirectiveBody> =
            if has_body { Some(&call_body) } else { None };
        d.execute(env, &params, &mut loop_vars, body_ref)?;
        return Ok(ExecOutcome::Done);
    }
    if let Some(ttm) = env.as_transform(&tm) {
        // Java UnifiedCall.java:86-103：TemplateTransformModel callee 同样求值
        // 命名参数 → env.visitAndTransform（getWriter 先产出变换输出，body 写入
        // 变换 writer；`<@t /><@m/>` —— ?interpret 产物调用后解释模板的宏可见）
        let mut params = HashMap::new();
        for (k, e) in args {
            if !k.is_empty() {
                params.insert(k.clone(), eval::eval(env, e)?);
            }
        }
        let body_elems: &[crate::core::Element] = body.as_deref().unwrap_or(&[]);
        let signal = ttm.transform_with_body(env, &params, body_elems)?;
        return match signal {
            RunSignal::Returned(v) => Ok(ExecOutcome::ReturnValue(v)),
            _ => Ok(ExecOutcome::Done),
        };
    }
    let call_name = match callee {
        CallTarget::Name(name) => name.clone(),
        CallTarget::Namespaced { ns, name } => format!("{ns}.{name}"),
        CallTarget::Expr(e) => expr_desc(e),
    };
    Err(TemplateError::misc(format!(
        "The value of {call_name} is not a macro or user-defined directive (it's a {})",
        tm.type_name
    )))
}

pub(crate) fn call_macro(
    env: &mut crate::core::Environment,
    mv: &Rc<crate::core::environment::MacroValue>,
    args: &[(String, crate::core::Expr)],
    body: Option<Vec<Element>>,
    body_params: Vec<String>,
) -> Result<ExecOutcome> {
    if mv.def.is_function {
        // Java UnifiedCall.java:76-80：Routine "f" is a function, not a directive.
        return Err(TemplateError::misc(format!(
            "Routine \"{}\" is a function, not a directive. Functions can only be called from expressions, like in ${{f()}}.",
            mv.def.name
        )));
    }
    let r = env.invoke_macro(mv, args, body, body_params)?;
    match r {
        RunSignal::Completed => Ok(ExecOutcome::Done),
        RunSignal::Returned(v) => Ok(ExecOutcome::ReturnValue(v)),
    }
}

pub struct CallBody {
    elements: Vec<Element>,
}

impl TemplateDirectiveBody for CallBody {
    fn render(&self, env: &mut crate::core::Environment) -> Result<()> {
        env.run_elements(&self.elements)
    }
}
