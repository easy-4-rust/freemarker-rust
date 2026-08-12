//! 空写入器 —— 对应 Java `freemarker.template.utility.NullWriter`
//! （丢弃所有输出的 Writer；Java 内部用于 import 初始化等场景——v1 无
//! writer 对象，输出丢弃由 env.capture 承担，本类型为 API 对应物）

/// 空写入器（对应 NullWriter.java；v1 输出丢弃经 `Environment::capture`）
pub struct NullWriter;

impl NullWriter {
    /// Java `NullWriter.INSTANCE`
    pub const INSTANCE: NullWriter = NullWriter;
}
