import freemarker.template.*;
import java.io.*;
import java.util.*;

/**
 * Java FreeMarker template renderer for L3 dual-engine comparison.
 *
 * Usage: java ProbeRender <template.ftl> [data.json]
 *
 * Renders a FreeMarker template with optional JSON data model.
 * Output is written to stdout. Errors go to stderr.
 *
 * The JSON data file should contain a single JSON object whose keys
 * become the template root variables. Supported value types:
 *   - string, number, boolean, null
 *   - array (becomes FreeMarker sequence)
 *   - object (becomes FreeMarker hash)
 */
public class ProbeRender {
    public static void main(String[] args) throws Exception {
        if (args.length < 1) {
            System.err.println("Usage: ProbeRender <template.ftl> [data.json]");
            System.err.println("  Renders a FreeMarker template with optional JSON data.");
            System.err.println("  Output is written to stdout.");
            System.exit(1);
        }

        File templateFile = new File(args[0]);
        if (!templateFile.exists() || !templateFile.isFile()) {
            System.err.println("Error: template file not found: " + args[0]);
            System.exit(1);
        }

        // Configure FreeMarker 2.3.34
        Configuration cfg = new Configuration(Configuration.VERSION_2_3_34);
        cfg.setDirectoryForTemplateLoading(templateFile.getParentFile());
        cfg.setObjectWrapper(new DefaultObjectWrapper(Configuration.VERSION_2_3_34));
        cfg.setDefaultEncoding("UTF-8");
        cfg.setNumberFormat("computer");

        // Build data model
        Map<String, Object> root;
        if (args.length >= 2) {
            File dataFile = new File(args[1]);
            if (!dataFile.exists()) {
                System.err.println("Error: data file not found: " + args[1]);
                System.exit(1);
            }
            String jsonText = readFile(dataFile);
            root = parseJSON(jsonText);
        } else {
            root = new HashMap<>();
        }

        // Load and process template
        Template template = cfg.getTemplate(templateFile.getName());
        StringWriter out = new StringWriter();
        try {
            template.process(root, out);
        } catch (TemplateException e) {
            // Print the error without the FTL stack trace prefix for cleaner output
            System.err.println("Error rendering template: " + e.getMessageWithoutStackTop());
            System.exit(2);
        }
        System.out.print(out.toString());
    }

    private static String readFile(File file) throws IOException {
        StringBuilder sb = new StringBuilder();
        try (BufferedReader reader = new BufferedReader(
                new InputStreamReader(new FileInputStream(file), "UTF-8"))) {
            String line;
            while ((line = reader.readLine()) != null) {
                sb.append(line).append('\n');
            }
        }
        return sb.toString();
    }

    // -----------------------------------------------------------------------
    // Minimal recursive-descent JSON parser
    // -----------------------------------------------------------------------

    private static Map<String, Object> parseJSON(String text) {
        JsonParser p = new JsonParser(text);
        Object value = p.parseValue();
        if (value instanceof Map) {
            @SuppressWarnings("unchecked")
            Map<String, Object> map = (Map<String, Object>) value;
            return map;
        }
        // Wrap non-object root in a map under key "data"
        Map<String, Object> wrapper = new HashMap<>();
        wrapper.put("data", value);
        return wrapper;
    }

    static class JsonParser {
        private final String text;
        private int pos;

        JsonParser(String text) {
            this.text = text;
            this.pos = 0;
        }

        void skipWhitespace() {
            while (pos < text.length() && Character.isWhitespace(text.charAt(pos))) {
                pos++;
            }
        }

        char peek() {
            skipWhitespace();
            if (pos >= text.length()) {
                throw new RuntimeException("Unexpected end of JSON input");
            }
            return text.charAt(pos);
        }

        char next() {
            skipWhitespace();
            if (pos >= text.length()) {
                throw new RuntimeException("Unexpected end of JSON input");
            }
            return text.charAt(pos++);
        }

        Object parseValue() {
            char c = peek();
            switch (c) {
                case '{': return parseObject();
                case '[': return parseArray();
                case '"': return parseString();
                case 't': case 'f': return parseBoolean();
                case 'n': return parseNull();
                default: return parseNumber();
            }
        }

        Map<String, Object> parseObject() {
            next(); // consume '{'
            Map<String, Object> map = new LinkedHashMap<>();
            if (peek() == '}') {
                next();
                return map;
            }
            while (true) {
                String key = parseString();
                if (next() != ':') {
                    throw new RuntimeException("Expected ':' after object key at position " + pos);
                }
                Object value = parseValue();
                map.put(key, value);
                char c = next();
                if (c == '}') break;
                if (c != ',') {
                    throw new RuntimeException("Expected ',' or '}' in object at position " + pos);
                }
            }
            return map;
        }

        List<Object> parseArray() {
            next(); // consume '['
            List<Object> list = new ArrayList<>();
            if (peek() == ']') {
                next();
                return list;
            }
            while (true) {
                list.add(parseValue());
                char c = next();
                if (c == ']') break;
                if (c != ',') {
                    throw new RuntimeException("Expected ',' or ']' in array at position " + pos);
                }
            }
            return list;
        }

        String parseString() {
            if (next() != '"') {
                throw new RuntimeException("Expected '\"' at position " + pos);
            }
            StringBuilder sb = new StringBuilder();
            while (pos < text.length()) {
                char c = text.charAt(pos++);
                if (c == '"') return sb.toString();
                if (c == '\\') {
                    if (pos >= text.length()) {
                        throw new RuntimeException("Unexpected end of string escape");
                    }
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
                            if (pos + 4 > text.length()) {
                                throw new RuntimeException("Unexpected end of unicode escape");
                            }
                            String hex = text.substring(pos, pos + 4);
                            pos += 4;
                            sb.append((char) Integer.parseInt(hex, 16));
                            break;
                        default:
                            sb.append(esc);
                            break;
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
            String numStr = text.substring(start, pos).trim();
            if (numStr.isEmpty()) {
                throw new RuntimeException("Expected number at position " + start);
            }
            if (isFloat) {
                return Double.parseDouble(numStr);
            }
            // Try int first, then long
            try {
                long v = Long.parseLong(numStr);
                if (v >= Integer.MIN_VALUE && v <= Integer.MAX_VALUE) {
                    return (int) v;
                }
                return v;
            } catch (NumberFormatException e) {
                return Double.parseDouble(numStr);
            }
        }

        Boolean parseBoolean() {
            if (text.startsWith("true", pos)) {
                pos += 4;
                return Boolean.TRUE;
            }
            if (text.startsWith("false", pos)) {
                pos += 5;
                return Boolean.FALSE;
            }
            throw new RuntimeException("Expected 'true' or 'false' at position " + pos);
        }

        Object parseNull() {
            if (text.startsWith("null", pos)) {
                pos += 4;
                return null;
            }
            throw new RuntimeException("Expected 'null' at position " + pos);
        }
    }
}
