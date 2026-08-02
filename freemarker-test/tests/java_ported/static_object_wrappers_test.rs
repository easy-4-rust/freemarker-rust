//! Java `freemarker.template.StaticObjectWrappersTest` 的 Rust 1:1 实现
//! （StaticObjectWrappersTest.java：ObjectWrapper 静态实例非 null 测试）
//!
//! 任务约定：Java 静态包装（无引擎等价物）→ 空 mod + 注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testNotNull：静态初始化顺序问题下的包装器实例非 null。
/// 引擎差异：v1 无 ObjectWrapper 静态单例家族（SimpleObjectWrapper 见
/// simple_object_wrapper_test.rs；BeansWrapper/DefaultObjectWrapper 未移植）——
/// 跳过。v1 等价物：SIMPLE_WRAPPER 常量恒可用（编译期单例）。
#[test]
fn test_not_null() {
    // Java 断言（注释保留）：
    //   assertNotNull(ObjectWrapper.BEANS_WRAPPER);
    //   assertNotNull(ObjectWrapper.DEFAULT_WRAPPER);
    //   assertNotNull(ObjectWrapper.SIMPLE_WRAPPER);
    // v1 无 ObjectWrapper 静态实例（freemarker::template::SIMPLE_WRAPPER 为
    // 无状态常量，不存在静态初始化顺序问题）
    let _ = freemarker::template::SIMPLE_WRAPPER;
}
