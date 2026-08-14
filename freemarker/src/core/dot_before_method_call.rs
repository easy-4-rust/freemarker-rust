//! 点方法调用前缀 —— 对应 Java `freemarker.core.DotBeforeMethodCall`
//! （Dot 子类；MethodCallAwareTemplateHashModel 的 getBeforeMethodCall 分支；
//!  若 hash 不支持 MethodCallAware → 降级为普通 Dot.evalOnHash）

/// 对应 Java `DotBeforeMethodCall`（ExprKind::Dot 变体的 MethodCall 前缀语义）
#[allow(dead_code)]
pub(crate) struct DotBeforeMethodCall;
