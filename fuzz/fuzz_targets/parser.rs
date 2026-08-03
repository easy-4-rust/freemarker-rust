//! cargo-fuzz target: freemarker 模板解析器
//!
//! 随机输入下 parse() 必须返回 Result（允许 Err）但不得 panic。
//! 运行: cargo fuzz run parser -- -max_total_time=60

use std::rc::Rc;

use freemarker::parser::parse;
use freemarker::template::Configuration;

fuzz_target!(|data: &[u8]| {
    // 限制 UTF-8 输入（模板资源 API 是 &str）；空字符串/无效 UTF-8 直接返回
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return,
    };
    // 限制大小防止长跑内存爆炸
    if text.len() > 32_768 {
        return;
    }
    let cfg = Rc::new(Configuration::default());
    let _ = parse(&cfg, "fuzz", text);
});
