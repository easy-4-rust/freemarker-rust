//! 固定结果表达式 —— 对应 Java `freemarker.core.ExpressionWithFixedResult`
//! （包装一个固定的 TemplateModel 值，_eval 直接返回；isLiteral 委托 sourceExpression）

/// 对应 Java `ExpressionWithFixedResult`（ExprKind 变体或包装器）
#[allow(dead_code)]
pub(crate) struct ExpressionWithFixedResult;
