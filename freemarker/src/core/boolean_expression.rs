//! 布尔表达式基类 —— 对应 Java `freemarker.core.BooleanExpression`
//! （抽象类；AndExpression/OrExpression/NotExpression 的父类；
//!  `_eval` 返回 TemplateBooleanModel.TRUE/FALSE）

/// 对应 Java `BooleanExpression`（ExprKind::And/Or/Not 变体承载语义）
#[allow(dead_code)]
pub(crate) struct BooleanExpression;
