//! Java `freemarker.core.EnvironmentCustomStateTest` 的 Rust 1:1 实现
//! （对应 Java: EnvironmentCustomStateTest —— `Environment.setCustomState`/
//!   `getCustomState` 的键值存取：初始 null → 设置 → 覆盖 → 置 null）。
//!
//! 引擎实现：Environment.custom_state（RefCell<HashMap<String, Option<TModel>>>，
//! environment.rs；get_custom_state/set_custom_state 对应 Java
//! Environment.getCustomState/setCustomState（Environment.java:3405-3446）——
//! Java 键按对象 identity，Rust 侧键为 String；值可为 null → Option 槽位，
//! 缺失与存 null 均读回 None，与 Java null-for-both 语义一致）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::template::{TModel, Template};
use std::collections::HashMap;
use std::rc::Rc;

/// Java test（Java 原文）：
///   private static final Object KEY_1 = new Object();
///   private static final Object KEY_2 = new Object();
///
///   Configuration cfg = new Configuration(Configuration.VERSION_2_3_24);
///   Template t = new Template(null, "", cfg);
///   Environment env = t.createProcessingEnvironment(null, null);
///   assertNull(env.getCustomState(KEY_1));
///   assertNull(env.getCustomState(KEY_2));
///   env.setCustomState(KEY_1, "a");
///   env.setCustomState(KEY_2, "b");
///   assertEquals("a", env.getCustomState(KEY_1));
///   assertEquals("b", env.getCustomState(KEY_2));
///   env.setCustomState(KEY_1, "c");
///   env.setCustomState(KEY_2, null);
///   assertEquals("c", env.getCustomState(KEY_1));
///   assertNull(env.getCustomState(KEY_2));
///
/// （Java `new Template(null, "", cfg)` 空模板 → Rust 空根元素树；渲染期
///   可见性：custom state 存于 Environment 字段，process() 全程可读）
#[test]
fn test() {
    let (c, _loader) = test_config();
    let cfg = Rc::new(c);
    let t = Template::new(String::new(), vec![], HashMap::new(), cfg);
    let mut out: Vec<u8> = Vec::new();
    let mut env = freemarker::core::Environment::new(
        &t,
        TModel::from_hash(indexmap::IndexMap::new()),
        &mut out,
    );

    // assertNull(env.getCustomState(KEY_1/KEY_2))
    assert!(env.get_custom_state("KEY_1").is_none());
    assert!(env.get_custom_state("KEY_2").is_none());

    env.set_custom_state("KEY_1", Some(TModel::from_scalar("a".to_string())));
    env.set_custom_state("KEY_2", Some(TModel::from_scalar("b".to_string())));
    assert_eq!(
        env.get_custom_state("KEY_1").unwrap().get_scalar().unwrap(),
        "a"
    );
    assert_eq!(
        env.get_custom_state("KEY_2").unwrap().get_scalar().unwrap(),
        "b"
    );

    // 覆盖 + 置 null（Java setCustomState(KEY_2, null)；put 返回旧值 "b"）
    let prev = env.set_custom_state("KEY_1", Some(TModel::from_scalar("c".to_string())));
    assert_eq!(prev.unwrap().get_scalar().unwrap(), "a");
    let prev = env.set_custom_state("KEY_2", None);
    assert_eq!(prev.unwrap().get_scalar().unwrap(), "b");
    assert_eq!(
        env.get_custom_state("KEY_1").unwrap().get_scalar().unwrap(),
        "c"
    );
    // 存 null → 读回 null（Java assertNull）
    assert!(env.get_custom_state("KEY_2").is_none());

    // 渲染期可见性：空模板渲染不影响 custom state（Java 测试同口径——
    // custom state 是 Environment 级状态，process() 前后均可读写）
    env.process().unwrap();
    assert_eq!(
        env.get_custom_state("KEY_1").unwrap().get_scalar().unwrap(),
        "c"
    );
    assert!(env.get_custom_state("KEY_2").is_none());
}
