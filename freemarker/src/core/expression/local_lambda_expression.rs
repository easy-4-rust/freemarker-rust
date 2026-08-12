//! Lambda 表达式 —— 对应 Java `freemarker.core.LocalLambdaExpression`
//! （`_eval` :46-59：构造 LambdaArgument；v1 仅构造槽位模型，
//! ?map/?filter 等消费方由内建智能体扩展）

use crate::core::environment::lambda_model;
use crate::core::Expr;
use crate::error::Result;
use crate::template::TModel;
use std::rc::Rc;

/// Lambda 表达式（对应 LocalLambdaExpression.java；解析器经 `ExprKind::Lambda` 承载）
pub struct LocalLambdaExpression {
    pub params: Vec<String>,
    pub body: Expr,
}

impl LocalLambdaExpression {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(params: Vec<String>, body: Expr) -> Self {
        LocalLambdaExpression { params, body }
    }

    /// 求值（Java `_eval` → LambdaArgument 槽位模型）
    pub(crate) fn eval(&self, _env: &mut crate::core::Environment) -> Result<TModel> {
        Ok(lambda_model(
            self.params.clone(),
            Rc::new(self.body.clone()),
        ))
    }
}
