//! 外部命令执行 —— 对应 Java `freemarker.template.utility.Execute`
//! （<@execute ...> 变换：执行外部命令并输出 stdout/stderr）
//! v1 差异：安全决策——外部命令执行不实现（security.md），类型保留以兼容
//! ?new 白名单的 Java 对应名（Java Execute 类在 v1 报 "Class not found"）

/// 外部命令执行（对应 Execute.java；v1 安全决策不实现——?new 时报
/// "Class not found: freemarker.template.utility.Execute"）
pub struct Execute;
