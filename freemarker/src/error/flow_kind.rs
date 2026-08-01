//! 流控信号 —— 对应 Java `freemarker.core.FlowControlException`
//! （break/continue 在指令栈中的内部传播信号，不面向用户）

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowKind {
    Break,
    Continue,
}
