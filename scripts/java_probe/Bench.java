import freemarker.template.*;
import freemarker.cache.StringTemplateLoader;
import java.io.*;
import java.util.*;

/**
 * Java FreeMarker 2.3.34 性能基准 —— 镜像 Rust benches/simple_render.rs 的 5 个场景，
 * 用于 L3 双引擎性能对比（目标 Rust ≥ Java 0.5×）。
 *
 * 用法: java -cp freemarker.jar:. Bench <iterations>
 * 输出: name=<ns/op> 每行一个场景
 */
public class Bench {

    static int ITERATIONS = 20000;
    static int WARMUP = 2000;

    static Configuration cfg() throws Exception {
        Configuration c = new Configuration(Configuration.VERSION_2_3_34);
        c.setObjectWrapper(new DefaultObjectWrapper(Configuration.VERSION_2_3_34));
        c.setDefaultEncoding("UTF-8");
        return c;
    }

    static Template parse(Configuration c, String name, String text) throws Exception {
        c.setTemplateLoader(new StringTemplateLoader() {{
            putTemplate(name, text);
        }});
        return c.getTemplate(name);
    }

    static double bench(Template t, Map<String, Object> root) throws Exception {
        // warmup
        for (int i = 0; i < WARMUP; i++) {
            StringWriter w = new StringWriter();
            t.process(root, w);
        }
        long start = System.nanoTime();
        for (int i = 0; i < ITERATIONS; i++) {
            StringWriter w = new StringWriter();
            t.process(root, w);
        }
        long end = System.nanoTime();
        return (double) (end - start) / ITERATIONS; // ns/op
    }

    public static void main(String[] args) throws Exception {
        if (args.length > 0) ITERATIONS = Integer.parseInt(args[0]);

        Configuration c = cfg();

        // 1. simple_hello_world
        Template hello = parse(c, "hello", "${message}");
        Map<String, Object> root1 = new HashMap<>();
        root1.put("message", "Hello, World!");
        System.out.printf("simple_hello_world=%.1f%n", bench(hello, root1));

        // 2. simple_loop_100
        Template loop = parse(c, "loop100", "<#list 1..100 as i>${i}</#list>");
        System.out.printf("simple_loop_100=%.1f%n", bench(loop, new HashMap<>()));

        // 3. if_else_chain
        String ifText = "<#if x == 1>A<#elseif x == 2>B<#elseif x == 3>C" +
            "<#elseif x == 4>D<#elseif x == 5>E<#elseif x == 6>F" +
            "<#elseif x == 7>G<#elseif x == 8>H<#elseif x == 9>I<#else>J</#if>";
        Template ifChain = parse(c, "ifchain", ifText);
        Map<String, Object> root3 = new HashMap<>();
        root3.put("x", 5);
        System.out.printf("if_else_chain=%.1f%n", bench(ifChain, root3));

        // 4. macro_call_100
        Template macro = parse(c, "macro100", "<#macro m>hello</#macro><#list 1..100 as i><@m/></#list>");
        System.out.printf("macro_call_100=%.1f%n", bench(macro, new HashMap<>()));

        // 5. big_data_model
        Template big = parse(c, "bigdata", "${big.key_0}${big.key_500}${big.key_999}");
        Map<String, Object> bigHash = new HashMap<>();
        for (int i = 0; i < 1000; i++) bigHash.put("key_" + i, "value_" + i);
        Map<String, Object> root5 = new HashMap<>();
        root5.put("big", bigHash);
        System.out.printf("big_data_model=%.1f%n", bench(big, root5));
    }
}
