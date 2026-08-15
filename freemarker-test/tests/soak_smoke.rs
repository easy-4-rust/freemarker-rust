//! 并发渲染 soak 套件（Stage 4：稳定性验证）。
//!
//! 测试矩阵：
//! 1. 8 线程 x 2000 迭代 = 16000 次混合模板渲染，验证并发安全与输出稳定性。
//! 2. 单线程 5000 轮内存稳定探针：输出字节恒定 + 耗时无趋势性增长。
//! 3. 整体 120s 超时守卫（后台线程 + 主线程 recv_timeout 防死锁）。
//!
//! Configuration 含 `RefCell`（`!Sync`），无法通过 `Arc<Configuration>` 跨线程共享。
//! 每线程独立构建 Configuration，共享模板文本字符串（`Arc<str>`），验证并发安全。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

// ─── 模板定义 ───────────────────────────────────────────────────────────────

/// 1. 深嵌套：30 层 `<#if>`，每层判断 x gte depth，数据 x=15 命中前 15 层。
///
/// 注：FreeMarker 标签内 `>=` 被词法器解析为 `>`（标签结束）+ `=`（文本），
/// 因此必须用 `gte` 替代 `>=` 做大于等于比较（与 Java FreeMarker 一致）。
const DEEP_NESTING_TPL: &str = "\
<#if x gte 1>L1\
<#if x gte 2>L2\
<#if x gte 3>L3\
<#if x gte 4>L4\
<#if x gte 5>L5\
<#if x gte 6>L6\
<#if x gte 7>L7\
<#if x gte 8>L8\
<#if x gte 9>L9\
<#if x gte 10>L10\
<#if x gte 11>L11\
<#if x gte 12>L12\
<#if x gte 13>L13\
<#if x gte 14>L14\
<#if x gte 15>L15\
<#if x gte 16>L16\
<#if x gte 17>L17\
<#if x gte 18>L18\
<#if x gte 19>L19\
<#if x gte 20>L20\
<#if x gte 21>L21\
<#if x gte 22>L22\
<#if x gte 23>L23\
<#if x gte 24>L24\
<#if x gte 25>L25\
<#if x gte 26>L26\
<#if x gte 27>L27\
<#if x gte 28>L28\
<#if x gte 29>L29\
<#if x gte 30>L30\
</#if></#if></#if></#if></#if>\
</#if></#if></#if></#if></#if>\
</#if></#if></#if></#if></#if>\
</#if></#if></#if></#if></#if>\
</#if></#if></#if></#if></#if>\
</#if></#if></#if></#if></#if>";

/// 2. 宏递归：macro 自调用 3 层 + nested content。
const MACRO_RECURSE_TPL: &str = "\
<#macro bomb d><#if d gt 0>pre-${d} <@bomb d-1>inner-${d}</@bomb> post-${d}</#if></#macro>\
<@bomb 3>leaf</@bomb>";

/// 3. XML `<#recurse>` 遍历模板。配合 XML_VISIT_TPL 使用。
///    定义 @default visitor 宏包装元素，@text 输出文本内容，递归遍历节点树。
///    参考 freemarker-test/tests/suite/cases/xmlns1/xmlns1.ftl 的 visitor 宏惯例。
const XML_VISIT_TPL: &str = "\
<#macro @default>[<#recurse>]</#macro>\
<#macro @text>${.node}</#macro>\
<#recurse doc>";

/// 生成约 50 节点的 XML 字符串（10 group x 5 child = 50+ 节点）。
fn build_xml_50_nodes() -> String {
    let mut xml = String::from("<root>");
    for g in 0..10 {
        xml.push_str(&format!("<group id=\"g{g}\">"));
        for i in 0..5 {
            xml.push_str(&format!("<item n=\"{i}\">val-{g}-{i}</item>"));
        }
        xml.push_str("</group>");
    }
    xml.push_str("</root>");
    xml
}

/// 4. 大循环 + 内建链：`<#list 1..500>` 输出索引，带 `?c` 数字内建。
const BIG_LOOP_TPL: &str = "<#list 1..500 as i>${i?c}<#if i_has_next>,</#if></#list>";

/// 模板名称常量（注册到 StringLoader）。
const TPL_NAMES: &[&str] = &["deep", "macro", "xml", "loop"];
const TPL_TEXTS: &[&str] = &[
    DEEP_NESTING_TPL,
    MACRO_RECURSE_TPL,
    XML_VISIT_TPL,
    BIG_LOOP_TPL,
];

// ─── 辅助函数 ───────────────────────────────────────────────────────────────

/// 构建每线程独立的 Configuration（Configuration 含 RefCell，!Sync，不可跨线程共享）。
fn thread_local_cfg(template_texts: &[(&str, &str)]) -> Rc<Configuration> {
    let mut cfg = Configuration::new();
    let loader = Arc::new(freemarker::cache::StringLoader::default());
    for (name, text) in template_texts {
        loader.put(name, text);
    }
    cfg.template_loader = loader;
    Rc::new(cfg)
}

/// 渲染单个模板，返回输出字符串。
fn render(cfg: &Configuration, name: &str, root: TModel) -> freemarker::error::Result<String> {
    let tpl = cfg.get_template(name)?;
    let mut out = Vec::new();
    tpl.process(root, &mut out)?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// 计算字符串的快速哈希（用于字节一致性校验，不求密码学安全）。
fn fast_hash(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// 为 deep 模板构建数据模型（x = 15）。
fn deep_model() -> TModel {
    let mut m = IndexMap::new();
    m.insert(
        "x".to_string(),
        TModel::from_number(freemarker::value::TNumber::from_i64(15)),
    );
    TModel::from_hash(m)
}

/// 为 xml 模板构建数据模型（doc = parse_xml(...)）。
fn xml_model() -> TModel {
    let xml = build_xml_50_nodes();
    let doc = freemarker::xml::parse_xml(&xml).expect("parse_xml failed");
    let mut m = IndexMap::new();
    m.insert("doc".to_string(), doc);
    TModel::from_hash(m)
}

/// 空数据模型。
fn empty_model() -> TModel {
    TModel::from_hash(IndexMap::new())
}

// ─── 测试 1：并发渲染稳定性 ─────────────────────────────────────────────────

#[test]
fn soak_concurrent_render_stability() {
    // 死锁守卫：后台线程执行测试，主线程 120s recv_timeout，超时 panic 报死锁。
    let (tx, rx) = mpsc::channel();
    let guard = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(run_concurrent_soak);
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(Ok(())) => {}                            // 测试通过
        Ok(Err(e)) => std::panic::resume_unwind(e), // 测试内部 panic
        Err(_) => {
            // 超时——强杀守卫线程并 panic
            drop(guard);
            panic!("DEADLOCK DETECTED: soak_concurrent_render_stability exceeded 120s timeout");
        }
    }
    let _ = guard.join();
}

fn run_concurrent_soak() {
    const NUM_THREADS: usize = 8;
    const ITERS_PER_THREAD: usize = 2000;
    const NUM_TEMPLATES: usize = 4;
    // 每线程每迭代渲染 4 个模板，总计 8 x 2000 x 4 = 64000 次渲染。
    let expected_total: u64 =
        (NUM_THREADS as u64) * (ITERS_PER_THREAD as u64) * (NUM_TEMPLATES as u64);

    // 共享模板文本（Arc<str>），各线程独立构建 Configuration。
    let shared_tpls: Vec<(&str, &str)> = TPL_NAMES
        .iter()
        .zip(TPL_TEXTS.iter())
        .map(|(n, t)| (*n, *t))
        .collect();

    // 收集各线程首末次渲染哈希，验证输出稳定性。
    let first_hashes: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let last_hashes: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let total_ok: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));

    let start = Instant::now();

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|_tid| {
            let tpls = shared_tpls.clone();
            let first_h = first_hashes.clone();
            let last_h = last_hashes.clone();
            let ok_count = total_ok.clone();

            std::thread::spawn(move || {
                // 每线程独立构建 Configuration（!Sync）。
                let cfg = thread_local_cfg(&tpls);
                let models: Vec<(&str, TModel)> = vec![
                    ("deep", deep_model()),
                    ("macro", empty_model()),
                    ("xml", xml_model()),
                    ("loop", empty_model()),
                ];

                // 跟踪 "deep" 模板的首次与末次输出，验证渲染确定性。
                let mut first_deep: Option<String> = None;
                let mut last_deep = String::new();
                let mut local_ok: u64 = 0;

                for iter in 0..ITERS_PER_THREAD {
                    for (name, model) in &models {
                        let model_clone = model.clone();
                        match render(&cfg, name, model_clone) {
                            Ok(output) => {
                                if *name == "deep" {
                                    if iter == 0 {
                                        first_deep = Some(output.clone());
                                    }
                                    if iter == ITERS_PER_THREAD - 1 {
                                        last_deep = output.clone();
                                    }
                                }
                                local_ok += 1;
                            }
                            Err(e) => {
                                panic!("thread render failed: template={name} iter={iter} err={e}")
                            }
                        }
                    }
                }

                // 首末次哈希比对（取 "deep" 模板输出，验证确定性渲染）。
                let first_s = first_deep.expect("no first deep output");
                first_h.lock().unwrap().push(fast_hash(&first_s));
                last_h.lock().unwrap().push(fast_hash(&last_deep));
                *ok_count.lock().unwrap() += local_ok;
            })
        })
        .collect();

    for h in handles {
        h.join().expect("thread panicked");
    }

    let elapsed = start.elapsed();
    let ok = *total_ok.lock().unwrap();

    // 断言：全部成功。
    assert_eq!(
        ok, expected_total,
        "expected {expected_total} successful renders, got {ok}"
    );

    // 断言：首末哈希一致（逐字节稳定）。
    let fh = first_hashes.lock().unwrap();
    let lh = last_hashes.lock().unwrap();
    for (i, (f, l)) in fh.iter().zip(lh.iter()).enumerate() {
        assert_eq!(
            f, l,
            "thread {i}: first render hash {f} != last render hash {l}"
        );
    }

    println!(
        "[soak] concurrent: {ok}/{expected_total} renders OK in {:.2?}",
        elapsed
    );
}

// ─── 测试 2：内存稳定探针 ───────────────────────────────────────────────────

/// 单线程 5000 轮渲染，验证输出字节恒定 + 耗时无趋势性增长。
///
/// 方法：每 1000 轮检查输出长度一致；首 500 轮均值 vs 末 500 轮均值差 < 50%。
/// 理由：避免引入 GlobalAlloc wrapper 的复杂度（需全局替换 allocator），
///      用「输出字节恒定 + 耗时无趋势性增长」作为轻量内存稳定代理指标。
///      若存在内存泄漏，GC 压力或分配器碎片会导致后期迭代显著变慢。
#[test]
fn soak_memory_stability_probe() {
    const TOTAL_ROUNDS: usize = 5000;
    const SAMPLE_INTERVAL: usize = 1000;

    let cfg = thread_local_cfg(&[("loop", BIG_LOOP_TPL)]);
    let model = empty_model();

    // 用于首/末 500 轮计时。
    let mut batch1_times: Vec<Duration> = Vec::with_capacity(500);
    let mut batch2_times: Vec<Duration> = Vec::with_capacity(500);
    let mut last_len: Option<usize> = None;

    for round in 0..TOTAL_ROUNDS {
        let t0 = Instant::now();
        let output =
            render(&cfg, "loop", model.clone()).unwrap_or_else(|e| panic!("round {round}: {e}"));
        let dt = t0.elapsed();

        // 每 SAMPLE_INTERVAL 次检查输出长度一致。
        if round % SAMPLE_INTERVAL == 0 {
            let len = output.len();
            if let Some(prev) = last_len {
                assert_eq!(
                    len, prev,
                    "output length changed at round {round}: was {prev}, now {len}"
                );
            }
            last_len = Some(len);
        }

        // 收集首/末 500 轮耗时。
        if round < 500 {
            batch1_times.push(dt);
        }
        if round >= TOTAL_ROUNDS - 500 {
            batch2_times.push(dt);
        }
    }

    // 耗时趋势检查：首 500 轮均值 vs 末 500 轮均值差 < 50%。
    let avg1: Duration = batch1_times.iter().sum::<Duration>() / batch1_times.len() as u32;
    let avg2: Duration = batch2_times.iter().sum::<Duration>() / batch2_times.len() as u32;
    let ratio = avg2.as_nanos() as f64 / avg1.as_nanos().max(1) as f64;

    println!(
        "[soak] memory probe: {TOTAL_ROUNDS} rounds, avg_first_500={avg1:?}, avg_last_500={avg2:?}, ratio={ratio:.2}"
    );

    assert!(
        ratio < 1.5,
        "performance degradation detected: avg_last_500 ({avg2:?}) is {ratio:.2}x avg_first_500 ({avg1:?}), threshold 1.5x"
    );
}
