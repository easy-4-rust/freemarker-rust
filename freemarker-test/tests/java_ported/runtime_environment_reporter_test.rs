//! Java `freemarker.test.RuntimeEnvironmentReporterTest` 的 Rust 1:1 实现
//! （对应 Java: RuntimeEnvironmentReporterTest —— 打印/记录 JUnit 运行环境的
//!   JVM 系统属性；类注释原文："Prints and logs what the JUnit test are running
//!   on (doesn't actually test anything)"）
//!
//! NOT_APPLICABLE: logRuntimeEnvironment —— Java 读取 JVM System 属性
//!   （java.version/java.vendor/java.vm.name/java.home/os.name/os.arch/os.version）
//!   并打印 + slf4j 日志（RuntimeEnvironmentReporterTest.java:33-41）；Rust 侧无
//!   JVM 系统属性 API，且该测试无任何断言（"doesn't actually test anything"）——
//!   无可 1:1 行为，Java 原文保留于方法注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java logRuntimeEnvironment：打印 JVM 运行时环境属性（无断言）
#[test]
fn log_runtime_environment() {
    // NOT_APPLICABLE: JVM System 属性（java.*/os.*）与 slf4j 日志——Rust 无对应
    // 系统属性 API；测试无断言（Java 原文见 RuntimeEnvironmentReporterTest.java）：
    // for (String propName : new String[] {
    //         "java.version", "java.vendor", "java.vm.name", "java.home",
    //         "os.name", "os.arch", "os.version" }) {
    //     String propValue = System.getProperty(propName);
    //     System.out.println(propName + ": " + propValue);
    //     log.info("{}: {}", propName, propValue);
    // }
}
