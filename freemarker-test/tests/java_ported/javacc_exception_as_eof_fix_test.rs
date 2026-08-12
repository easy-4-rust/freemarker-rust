//! Java `freemarker.template.JavaCCExceptionAsEOFFixTest` 的 Rust 1:1 实现
//! （JavaCCExceptionAsEOFFixTest.java：JavaCC 把 Reader 抛出的异常静默当作
//!   EOF 的问题及 FreeMarker 修复测试）
//!
//! 引擎差异：v1 解析器直接读取完整字符串（无 Reader 流式读取），不存在
//! "Reader 中途抛异常被吞"的 JavaCC 行为——整体跳过并注释。
//!
//! NOT_APPLICABLE: testIOException/testRuntimeException/testError —— Java 用
//!   FailingReader（Reader 流在内容 "abc" 后抛 IOException/RuntimeException/Error）
//!   验证 JavaCC 不把 Reader 异常静默吞成 EOF（JavaCCExceptionAsEOFFixTest.java:29-87）；
//!   v1 解析器直接接收完整 &str，无 Reader 流式输入面，该缺陷不存在——Java 原文
//!   保留于各方法注释，testIOException/testNoException 仅保留引擎字符串解析正常性检查。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testIOException：Reader 抛 IOException 时解析必须失败（不被当作 EOF）
#[test]
fn test_io_exception() {
    // NOT_APPLICABLE: FailingReader(IOException("test")) 传播断言——v1 无
    // Reader 输入流（parse 接收 &str），无此缺陷面；以下仅作引擎字符串解析
    // 正常性检查（Java 原文见 JavaCCExceptionAsEOFFixTest.java:90-97）。
    // Java：new Template(null, FailingReader(IOException("test")), cfg) 抛
    // IOException，消息 "test"——JavaCC 修复前被吞成 EOF。
    let (c, _loader) = test_config();
    let cfg = std::rc::Rc::new(c.clone());
    let _ = freemarker::parser::parse(&cfg, "adhoc", "abc").expect("字符串解析正常");
}

/// Java testRuntimeException：Reader 抛 RuntimeException 时同样必须传播
#[test]
fn test_runtime_exception() {
    // NOT_APPLICABLE: FailingReader(NullPointerException("test")) → 解析抛 NPE
    // 消息 "test"（JavaCCExceptionAsEOFFixTest.java:99-107）——v1 无 Reader 流。
    // Java：FailingReader(NullPointerException("test")) → 解析抛 NPE 消息 "test"
    // —— v1 无 Reader 流（注释保留）
}

/// Java testError：Reader 抛 Error 时同样必须传播
#[test]
fn test_error() {
    // NOT_APPLICABLE: FailingReader(OutOfMemoryError("test")) → 解析抛 OOM
    // 消息 "test"（JavaCCExceptionAsEOFFixTest.java:110-117）——v1 无 Reader 流。
    // Java：FailingReader(OutOfMemoryError("test")) → 解析抛 OOM
    // —— v1 无 Reader 流（注释保留）
}

/// Java testNoException：Reader 正常结束 → 模板内容 == "abc"
#[test]
fn test_no_exception() {
    let (c, _loader) = test_config();
    let cfg = std::rc::Rc::new(c.clone());
    let t = freemarker::parser::parse(&cfg, "adhoc", "abc").expect("解析成功");
    let mut out = Vec::new();
    t.process(
        freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    )
    .unwrap();
    // Java：assertEquals("abc", t.toString())
    assert_eq!(String::from_utf8_lossy(&out), "abc");
}
