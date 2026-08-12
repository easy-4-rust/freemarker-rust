//! 算术表达式 —— 对应 Java `freemarker.core.ArithmeticExpression`
//! （`_eval` :48-94；操作符字段 :52-66；`-`/`*`/`/`/`%` 四则，`+` 属 AddConcatExpression）

use crate::core::arithmetic_engine::{ArithmeticEngine, BigDecimalEngine};
use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 算术操作符（Java ArithmeticExpression.operation 字段 :52-66：
/// MINUS / MULTIPLY / DIVIDE / MODULO）
#[derive(Clone, Copy)]
pub enum NumOp {
    Sub,
    Mul,
    Div,
    Mod,
}

impl NumOp {
    /// 操作符符号（错误消息用，Java :52-66）
    pub fn symbol(self) -> &'static str {
        match self {
            NumOp::Sub => "-",
            NumOp::Mul => "*",
            NumOp::Div => "/",
            NumOp::Mod => "%",
        }
    }
}

/// 算术表达式（对应 ArithmeticExpression.java；解析器经 `ExprKind::Sub/Mul/Div/Mod`
/// 承载，dispatch 时按 variant 选定 `op` 构造）
pub struct ArithmeticExpression {
    pub left: Expr,
    pub right: Expr,
    pub op: NumOp,
}

impl ArithmeticExpression {
    /// 构造（Java 构造器；Rust 侧由解析器/求值 dispatch 产生）
    pub fn new(left: Expr, right: Expr, op: NumOp) -> Self {
        ArithmeticExpression { left, right, op }
    }

    /// 求值（Java `_eval` :48-94）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_binary_number(env, &self.left, &self.right, self.op)
    }
}

fn eval_binary_number(
    env: &mut crate::core::Environment,
    a: &Expr,
    b: &Expr,
    op: NumOp,
) -> Result<TModel> {
    // Java ArithmeticExpression._eval（:50-51）：lho.evalToNumber → rho.evalToNumber；
    // 操作数 null → Expression.modelToNumber（:154-160）→ NonNumericalException(blamed, null)
    // → UnexpectedTypeException 对 null 模型输出 "The following has evaluated to null or missing"
    let l = eval(env, a)?;
    if l.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(a),
        ));
    }
    let r = eval(env, b)?;
    if r.is_nothing() {
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(b),
        ));
    }
    // Java ArithmeticExpression._eval：操作数类型失败时 blame 对应操作数——
    // `For "-" left-hand operand: Expected a number, ... ==> lho` /
    // `For "-" right-hand operand: ... ==> rho`（位置 = 操作数表达式起始）
    let l = l
        .get_number()
        .map_err(|e| blame_number_operand(e, env, op.symbol(), "left-hand operand", a))?;
    let r = r
        .get_number()
        .map_err(|e| blame_number_operand(e, env, op.symbol(), "right-hand operand", b))?;
    let engine = BigDecimalEngine::default();
    let out = match op {
        NumOp::Sub => engine.sub(&l, &r)?,
        NumOp::Mul => engine.mul(&l, &r)?,
        NumOp::Div => engine.div(&l, &r)?,
        NumOp::Mod => engine.mod_op(&l, &r)?,
    };
    Ok(TModel::from_number(out))
}

/// 数字操作数类型错误 → Java `For "{op}" {side}: ... ==> {expr}` 形式
/// （NonNumericalException 的 blamer/blame 表达式/位置）
fn blame_number_operand(
    e: TemplateError,
    env: &crate::core::Environment,
    op: &str,
    side: &str,
    blamed: &Expr,
) -> TemplateError {
    match e {
        TemplateError::TypeMismatch {
            expected,
            actual,
            ctx,
        } => TemplateError::TypeMismatch {
            expected,
            actual,
            ctx: Box::new(crate::error::ErrorCtx {
                blamer: Some(format!("For \"{op}\" {side}: ")),
                blamed_expr: Some(crate::core::environment::expr_desc(blamed)),
                span: blamed.span,
                template_name: Some(env.current_template_name.clone()),
                ..*ctx
            }),
        },
        other => other,
    }
}
