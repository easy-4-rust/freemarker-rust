//! 类流式中间操作内建 —— 对应 Java `freemarker.core.IntermediateStreamOperationLikeBuiltIn`
//! （抽象类；BuiltInWithParseTimeParameters 子类；接受 lambda 参数的序列变换；
//!  如 ?filter、?transform 等；lazilyGeneratedResultEnabled 控制惰性求值）

/// 对应 Java `IntermediateStreamOperationLikeBuiltIn`（BuiltIn 变体承载流式操作语义）
#[allow(dead_code)]
pub(crate) struct IntermediateStreamOperationLikeBuiltIn;
