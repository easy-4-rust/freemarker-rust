//! 动态键名方法调用前缀 —— 对应 Java `freemarker.core.DynamicKeyNameBeforeMethodCall`
//! （DynamicKeyName 子类；MethodCallAwareTemplateHashModel 的 getBeforeMethodCall 分支）

/// 对应 Java `DynamicKeyNameBeforeMethodCall`（ExprKind::DynamicKeyName 的 MethodCall 前缀语义）
#[allow(dead_code)]
pub(crate) struct DynamicKeyNameBeforeMethodCall;
