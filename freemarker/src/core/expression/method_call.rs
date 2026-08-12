//! 方法调用 —— 对应 Java `freemarker.core.MethodCall`
//! （`_eval` :51-71：宏 → invokeFunction；方法模型 → TemplateMethodModelEx.exec）

use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 方法调用表达式（对应 MethodCall.java；解析器经 `ExprKind::Call` 承载）
pub struct MethodCall {
    pub callee: Expr,
    pub args: Vec<Expr>,
}

impl MethodCall {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(callee: Expr, args: Vec<Expr>) -> Self {
        MethodCall { callee, args }
    }

    /// 求值（Java `_eval` :51-71）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        let c = eval(env, &self.callee)?;
        if let Some(mv) = env.as_macro(&c) {
            // Java MethodCall :68-71：instanceof Macro → invokeFunction
            if !mv.def.is_function {
                return Err(TemplateError::misc(
                    "A macro cannot be called in an expression. (Functions can be.)",
                ));
            }
            let args: Vec<(String, Expr)> = self
                .args
                .iter()
                .map(|e| (String::new(), e.clone()))
                .collect();
            return env.invoke_function(&mv, &args);
        }
        if let Some(m) = &c.method {
            let mut vals = Vec::with_capacity(self.args.len());
            for a in &self.args {
                // Java：标识符求值不抛错（Environment.getVariable 返回 null），缺失
                // 参数以 null 传入方法（`m.bar(null, 11)` 的 null 即缺失变量——
                // jar 实测合法）；本引擎解析层抛 Err → 此处按 Java 语义转为 nothing
                match eval(env, a) {
                    Ok(v) => vals.push(v),
                    Err(TemplateError::InvalidReference { .. }) => vals.push(TModel::nothing()),
                    Err(e) => return Err(e),
                }
            }
            // Java :60-66：TemplateMethodModelEx.exec(arguments.getModelList(env))；
            // Java 经线程局部 Environment 访问上下文，Rust 显式传 env
            return m.exec(env, vals);
        }
        Err(TemplateError::misc(format!(
            "The value of {} is not a method or function (it's a {})",
            crate::core::environment::expr_desc(&self.callee),
            c.type_name
        )))
    }
}
