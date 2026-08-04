//! Java `freemarker.core.EnvironmentCustomStateTest` 的 Rust 1:1 实现
//! （对应 Java: EnvironmentCustomStateTest —— `Environment.setCustomState`/
//!   `getCustomState` 的键值存取：初始 null → 设置 → 覆盖 → 置 null）。
//!
//! ENGINE_GAP: 引擎 Environment 没有 custom state API（Java
//!   Environment.setCustomState/getCustomState，自定义指令/函数跨调用共享状态的
//!   键值表），v1 无对应字段/方法。
//!
//! NOT_APPLICABLE: test —— 直接依赖
//!   `t.createProcessingEnvironment(null, null)` + `env.setCustomState`/
//!   `env.getCustomState`（引擎无 createProcessingEnvironment 与 custom state），
//!   Java 原文保留为注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（EnvironmentCustomStateTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java test（Java 原文）：
//   private static final Object KEY_1 = new Object();
//   private static final Object KEY_2 = new Object();
//
//   Configuration cfg = new Configuration(Configuration.VERSION_2_3_24);
//   Template t = new Template(null, "", cfg);
//   Environment env = t.createProcessingEnvironment(null, null);
//   assertNull(env.getCustomState(KEY_1));
//   assertNull(env.getCustomState(KEY_2));
//   env.setCustomState(KEY_1, "a");
//   env.setCustomState(KEY_2, "b");
//   assertEquals("a", env.getCustomState(KEY_1));
//   assertEquals("b", env.getCustomState(KEY_2));
//   env.setCustomState(KEY_1, "c");
//   env.setCustomState(KEY_2, null);
//   assertEquals("c", env.getCustomState(KEY_1));
//   assertNull(env.getCustomState(KEY_2));
