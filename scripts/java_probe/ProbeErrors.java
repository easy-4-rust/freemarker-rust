import freemarker.template.*;
import java.io.*;
import java.util.*;

/**
 * Java FreeMarker error message probe for the M5 error-alignment milestone.
 *
 * Renders ~45 failing templates with FreeMarker 2.3.34, captures the FULL
 * error message (TemplateException.getMessage(), which includes the FTL stack
 * trace section) and writes one baseline file per scenario into the output
 * directory (default: freemarker/src/error/expected_messages/).
 *
 * Also prints one JSON line per scenario to stdout:
 *   {"scenario": "...", "template": "...", "message": "..."}
 * (message with \n / \t / \" / \\ escaped so the JSON is one physical line).
 *
 * Usage: java ProbeErrors <output-dir>
 */
public class ProbeErrors {

    /** scenario name → (template source, optional data JSON or null) */
    static final String[][] SCENARIOS = {
        // ---------------------------------------------------------------
        // Runtime errors: invalid references
        // ---------------------------------------------------------------
        {"missing_var", "${missing}", null},
        {"missing_var_dot", "${user.name}", null},
        {"missing_var_nested", "${a.b.c}", "{\"a\": {\"b\": {}}}"},
        {"missing_var_if", "<#if missing>yes</#if>", null},
        {"missing_var_list", "<#list [1,2] as x>${missing}</#list>", null},
        {"missing_var_builtin_string", "${missing?string}", null},
        {"missing_in_if_body", "<#if true>${missing}</#if>", null},
        {"missing_in_nested_if", "<#if true><#if false>${missing}</#if></#if>", null},
        {"missing_in_assign", "<#assign x = missing>", null},
        {"missing_in_include_body", "<#include \"sub.ftl\">", null},
        {"missing_in_escape", "<#escape x as x?upper_case>${missing}</#escape>", null},

        // ---------------------------------------------------------------
        // Runtime errors: type mismatches
        // ---------------------------------------------------------------
        {"type_minus", "${1 - \"a\"}", null},
        {"type_minus_var", "${n - s}", "{\"n\": 1, \"s\": \"a\"}"},
        {"type_if_boolean", "<#if 1>yes</#if>", null},
        {"type_index_boolean", "${x[0]}", "{\"x\": true}"},
        {"type_string_index_oob", "${x[1]}", "{\"x\": \"a\"}"},
        {"type_dot_hash", "${x.y}", "{\"x\": \"abc\"}"},
        {"type_dot_boolean", "${x.y}", "{\"x\": true}"},
        {"type_upper_case_seq", "${x?upper_case}", "{\"x\": [1]}"},
        {"type_string_seq", "${x?string}", "{\"x\": [1]}"},
        {"type_matches_number", "${x?matches(\"a\")}", "{\"x\": 1}"},
        {"type_assign_minus_eq", "<#assign x = \"a\"><#assign x -= 1>${x}", null},
        {"type_interp_sequence", "${list}", "{\"list\": [1, 2]}"},
        {"type_interp_hash", "${hash}", "{\"hash\": {\"a\": 1}}"},
        {"type_if_sequence", "<#if list>yes</#if>", "{\"list\": [1]}"},

        // ---------------------------------------------------------------
        // Runtime errors: misc
        // ---------------------------------------------------------------
        {"div_by_zero", "${1 / 0}", null},
        {"mod_by_zero", "${1 % 0}", null},
        {"stop", "<#stop \"bye\">", null},
        {"stop_plain", "<#stop>", null},
        {"break_outside", "<#break>", null},
        {"continue_outside", "<#continue>", null},
        {"return_outside", "<#return>", null},
        {"unknown_builtin", "${x?nonexistent_builtin}", "{\"x\": 1}"},
        {"boolean_format_legacy", "${true}", null},
        {"unknown_output_format", "<#outputformat \"nope\">x</#outputformat>", null},

        // ---------------------------------------------------------------
        // Runtime errors: include / not found
        // ---------------------------------------------------------------
        {"include_not_found", "<#include \"nope.ftl\">", null},
        {"include_parse_error", "<#include \"broken.ftl\">", null},

        // ---------------------------------------------------------------
        // Runtime errors: macro / function invocation
        // ---------------------------------------------------------------
        {"macro_too_many_args", "<#macro m a>${a}</#macro><@m 1 2/>", null},
        {"macro_undeclared_param", "<#macro m a>${a}</#macro><@m b=1/>", null},
        {"macro_missing_required_param", "<#macro m a>${a}</#macro><@m/>", null},
        {"function_too_many_args", "<#function f a>${a}</#function>${f(1, 2)}", null},
        {"macro_error_stack", "<#macro m>${missing}</#macro><@m/>", null},
        {"macro_nested_error_stack", "<#macro a><#macro b>${missing}</#macro><@b/></#macro><@a/>", null},
        {"macro_loop_error_stack", "<#macro m><#list [1] as x>${missing}</#list></#macro><@m/>", null},
        {"nested_in_macro_error_stack", "<#macro m><#nested></#macro><@m>${missing}</@m>", null},
        {"nested_body_in_macro_stack", "<#macro m><#nested></#macro><@m><#list [1] as y>${missing}</#list></@m>", null},
        {"macro_default_undefined", "<#macro m a=a>${a}</#macro><@m/>", null},
        {"missing_macro", "<@notdefmacro/>", null},

        // ---------------------------------------------------------------
        // Parse errors
        // ---------------------------------------------------------------
        {"parse_unclosed_tag", "<#if x>", null},
        {"parse_needless_interpolation", "<#if ${x} == 3></#if>", null},
        {"parse_needless_interpolation_assign", "<#assign x = ${y}>", null},
        {"parse_unknown_directive", "<#foo />", null},
        {"parse_unknown_closing", "</#foo>", null},
        {"parse_expected_close", "<#if x></#if", null},
        {"parse_unclosed_interpolation", "${x", null},
        {"parse_unclosed_string", "${ \"abc}", null},
        {"parse_unclosed_comment", "<#-- foo", null},
        {"parse_bad_close", "<#if x></#else>", null},
        {"parse_malformed_assign", "<#assign>", null},
        {"parse_malformed_list_close", "<#list [1] as x>${x}<#list>", null},
        {"parse_items_outside", "<#items as x></#items>", null},
        {"parse_macro_no_end", "<#macro m>body", null},
        {"parse_macro_return_value", "<#macro m><#return 1></#macro><@m/>", null},
        {"parse_break_outside", "<#break>", null},
        {"parse_setting_unknown", "<#setting foo=\"bar\">", null},
        {"parse_invalid_char", "${x + }", null},
        {"parse_bad_escape", "<#escape x as>${x}</#escape>", null},
        {"parse_nested_comment", "<#if x>${y}</#list>", null},
        {"parse_ftl_header_bad", "<#ftl strict_syntax=1>", null},
        {"parse_double_close", "<#if x></#if></#if>", null},
    };

    public static void main(String[] args) throws Exception {
        String outDir = args.length >= 1 ? args[0] : "expected_messages";
        File dir = new File(outDir);
        if (!dir.exists() && !dir.mkdirs()) {
            System.err.println("Cannot create output dir: " + outDir);
            System.exit(1);
        }

        // Suppress the JUL logger noise (timestamped "Error executing FreeMarker
        // template" lines) — we capture e.getMessage() directly anyway.
        freemarker.log.Logger.selectLoggerLibrary(freemarker.log.Logger.LIBRARY_NONE);

        int ok = 0, failed = 0;
        for (String[] sc : SCENARIOS) {
            String name = sc[0];
            String ftl = sc[1];
            String dataJson = sc[2];
            try {
                Result r = runScenario(name, ftl, dataJson);
                String msg = r.message;
                // Write baseline file
                try (Writer w = new OutputStreamWriter(new FileOutputStream(new File(dir, name + ".txt")), "UTF-8")) {
                    w.write(msg);
                }
                // JSON line to stdout
                System.out.println("{\"scenario\": " + jq(name)
                        + ", \"template\": " + jq(ftl)
                        + ", \"message\": " + jq(msg) + "}");
                ok++;
            } catch (Throwable t) {
                System.err.println("SCENARIO FAILED: " + name + ": " + t);
                failed++;
            }
        }
        System.err.println("ProbeErrors done: " + ok + " ok, " + failed + " failed (out dir: " + dir.getAbsolutePath() + ")");
    }

    static class Result {
        String message;
        Result(String message) { this.message = message; }
    }

    static Result runScenario(String name, String ftl, String dataJson) throws Exception {
        File work = new File(System.getProperty("java.io.tmpdir"), "fm_errors_" + name);
        if (work.exists()) deleteRecursively(work);
        work.mkdirs();
        File tplFile = new File(work, name + ".ftl");
        try (Writer w = new OutputStreamWriter(new FileOutputStream(tplFile), "UTF-8")) {
            w.write(ftl);
        }
        if ("include_parse_error".equals(name)) {
            try (Writer w = new OutputStreamWriter(new FileOutputStream(new File(work, "broken.ftl")), "UTF-8")) {
                w.write("<#if x>");
            }
        }
        if ("missing_in_include_body".equals(name)) {
            try (Writer w = new OutputStreamWriter(new FileOutputStream(new File(work, "sub.ftl")), "UTF-8")) {
                w.write("${missing}");
            }
        }

        Configuration cfg = new Configuration(Configuration.VERSION_2_3_34);
        cfg.setDirectoryForTemplateLoading(work);
        cfg.setObjectWrapper(new DefaultObjectWrapper(Configuration.VERSION_2_3_34));
        cfg.setDefaultEncoding("UTF-8");
        cfg.setNumberFormat("computer");
        cfg.setLogTemplateExceptions(false);
        // Default handler is RETHROW_HANDLER; production parity target.

        Map<String, Object> root = new HashMap<>();
        if (dataJson != null) {
            JsonParser p = new JsonParser(dataJson);
            Object v = p.parseValue();
            if (v instanceof Map) {
                @SuppressWarnings("unchecked")
                Map<String, Object> m = (Map<String, Object>) v;
                root = m;
            } else {
                root.put("data", v);
            }
        }

        Template template;
        try {
            template = cfg.getTemplate(name + ".ftl");
        } catch (freemarker.core.ParseException e) {
            // Parse-time failure (ParseException extends IOException, NOT
            // TemplateException, in FreeMarker)
            return new Result(e.getMessage());
        } catch (IOException e) {
            throw new RuntimeException("Cannot load template: " + e, e);
        }

        StringWriter out = new StringWriter();
        try {
            template.process(root, out);
            throw new RuntimeException("Template did NOT fail! Output: " + out);
        } catch (TemplateException e) {
            return new Result(e.getMessage());
        }
    }

    // -----------------------------------------------------------------------
    // JSON string quoting (for the JSONL output)
    // -----------------------------------------------------------------------

    static String jq(String s) {
        StringBuilder sb = new StringBuilder(s.length() + 16);
        sb.append('"');
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"': sb.append("\\\""); break;
                case '\\': sb.append("\\\\"); break;
                case '\n': sb.append("\\n"); break;
                case '\r': sb.append("\\r"); break;
                case '\t': sb.append("\\t"); break;
                case '\b': sb.append("\\b"); break;
                case '\f': sb.append("\\f"); break;
                default:
                    if (c < 0x20) {
                        sb.append(String.format("\\u%04x", (int) c));
                    } else {
                        sb.append(c);
                    }
            }
        }
        sb.append('"');
        return sb.toString();
    }

    static void deleteRecursively(File f) {
        File[] kids = f.listFiles();
        if (kids != null) {
            for (File k : kids) deleteRecursively(k);
        }
        f.delete();
    }

    // -----------------------------------------------------------------------
    // Minimal JSON parser (data models for scenarios)
    // -----------------------------------------------------------------------

    static class JsonParser {
        private final String text;
        private int pos;

        JsonParser(String text) { this.text = text; this.pos = 0; }

        void skipWs() {
            while (pos < text.length() && Character.isWhitespace(text.charAt(pos))) pos++;
        }
        char peek() {
            skipWs();
            if (pos >= text.length()) throw new RuntimeException("Unexpected end of JSON");
            return text.charAt(pos);
        }
        char next() {
            skipWs();
            if (pos >= text.length()) throw new RuntimeException("Unexpected end of JSON");
            return text.charAt(pos++);
        }

        Object parseValue() {
            char c = peek();
            switch (c) {
                case '{': return parseObject();
                case '[': return parseArray();
                case '"': return parseString();
                case 't': case 'f': return parseBoolean();
                case 'n': return null;
                default: return parseNumber();
            }
        }

        Map<String, Object> parseObject() {
            next();
            Map<String, Object> map = new LinkedHashMap<>();
            if (peek() == '}') { next(); return map; }
            while (true) {
                String key = parseString();
                if (next() != ':') throw new RuntimeException("Expected ':' at " + pos);
                Object value = parseValue();
                map.put(key, value);
                char c = next();
                if (c == '}') break;
                if (c != ',') throw new RuntimeException("Expected ',' or '}' at " + pos);
            }
            return map;
        }

        List<Object> parseArray() {
            next();
            List<Object> list = new ArrayList<>();
            if (peek() == ']') { next(); return list; }
            while (true) {
                list.add(parseValue());
                char c = next();
                if (c == ']') break;
                if (c != ',') throw new RuntimeException("Expected ',' or ']' at " + pos);
            }
            return list;
        }

        String parseString() {
            if (next() != '"') throw new RuntimeException("Expected '\"' at " + pos);
            StringBuilder sb = new StringBuilder();
            while (pos < text.length()) {
                char c = text.charAt(pos++);
                if (c == '"') return sb.toString();
                if (c == '\\') {
                    char esc = text.charAt(pos++);
                    switch (esc) {
                        case '"': sb.append('"'); break;
                        case '\\': sb.append('\\'); break;
                        case '/': sb.append('/'); break;
                        case 'b': sb.append('\b'); break;
                        case 'f': sb.append('\f'); break;
                        case 'n': sb.append('\n'); break;
                        case 'r': sb.append('\r'); break;
                        case 't': sb.append('\t'); break;
                        case 'u':
                            sb.append((char) Integer.parseInt(text.substring(pos, pos + 4), 16));
                            pos += 4;
                            break;
                        default: sb.append(esc);
                    }
                } else {
                    sb.append(c);
                }
            }
            throw new RuntimeException("Unterminated string");
        }

        Object parseNumber() {
            int start = pos;
            if (pos < text.length() && text.charAt(pos) == '-') pos++;
            while (pos < text.length() && Character.isDigit(text.charAt(pos))) pos++;
            boolean isFloat = false;
            if (pos < text.length() && text.charAt(pos) == '.') {
                isFloat = true;
                pos++;
                while (pos < text.length() && Character.isDigit(text.charAt(pos))) pos++;
            }
            if (pos < text.length() && (text.charAt(pos) == 'e' || text.charAt(pos) == 'E')) {
                isFloat = true;
                pos++;
                if (pos < text.length() && (text.charAt(pos) == '+' || text.charAt(pos) == '-')) pos++;
                while (pos < text.length() && Character.isDigit(text.charAt(pos))) pos++;
            }
            String num = text.substring(start, pos).trim();
            if (num.isEmpty()) throw new RuntimeException("Expected number at " + start);
            if (isFloat) return Double.parseDouble(num);
            try {
                long v = Long.parseLong(num);
                if (v >= Integer.MIN_VALUE && v <= Integer.MAX_VALUE) return (int) v;
                return v;
            } catch (NumberFormatException e) {
                return Double.parseDouble(num);
            }
        }

        Boolean parseBoolean() {
            if (text.startsWith("true", pos)) { pos += 4; return Boolean.TRUE; }
            if (text.startsWith("false", pos)) { pos += 5; return Boolean.FALSE; }
            throw new RuntimeException("Expected boolean at " + pos);
        }
    }
}
