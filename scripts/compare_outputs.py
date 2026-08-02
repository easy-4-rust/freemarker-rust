#!/usr/bin/env python3
"""Compare Rust freemarker engine output against Java FreeMarker reference output.

L3 dual-engine comparison harness. Verifies that the Rust engine produces
output identical to Java FreeMarker 2.3.34.

Usage:
  # Single template comparison (live dual-engine)
  python3 compare_outputs.py --template path/to/template.ftl --data path/to/data.json
  python3 compare_outputs.py --template path/to/template.ftl --data path/to/data.json --java-only

  # Suite mode: compare Rust output against expected (Java-generated) files
  python3 compare_outputs.py --suite golden
  python3 compare_outputs.py --suite golden --limit 10
  python3 compare_outputs.py --suite golden --case "variables"

  # Suite mode: live dual-engine for cases with JSON data files
  python3 compare_outputs.py --suite live --data-dir suite_data/

  # Build the Rust probe binary (do this first)
  python3 compare_outputs.py --build

Output:
  PASS   <case-name>
  FAIL   <case-name>  <diff-preview>
  SKIP   <case-name>  (<reason>)
  ERROR  <case-name>  (<error-message>)
"""

import argparse
import difflib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths (relative to this script)
# ---------------------------------------------------------------------------
SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_ROOT = SCRIPT_DIR.parent
SUITE_DIR = PROJECT_ROOT / "freemarker-test" / "tests" / "suite"
MANIFEST_PATH = SUITE_DIR / "manifest.json"
JAVA_PROBE = SCRIPT_DIR / "java_probe" / "probe.sh"

# ---------------------------------------------------------------------------
# Rust probe
# ---------------------------------------------------------------------------

def build_rust_probe():
    """Build the Rust probe binary."""
    print("Building Rust probe binary...")
    result = subprocess.run(
        ["cargo", "build", "--release", "-p", "freemarker-test", "--bin", "probe"],
        cwd=PROJECT_ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print("ERROR: Failed to build Rust probe:")
        print(result.stderr)
        sys.exit(1)
    # Also build in debug if release fails (fallback)
    print("  Rust probe built successfully.")


def render_rust(template_path: str, data_json_path: str) -> bytes:
    """Render a template with the Rust engine, return output bytes.

    Uses cargo run --bin probe (debug build for speed during development).
    If the binary doesn't exist, tries release build, then builds debug.
    """
    bin_path = PROJECT_ROOT / "target" / "release" / "probe"
    if not bin_path.exists():
        bin_path = PROJECT_ROOT / "target" / "debug" / "probe"

    if not bin_path.exists():
        # Build the binary
        print("Rust probe binary not found, building...")
        subprocess.run(
            ["cargo", "build", "-p", "freemarker-test", "--bin", "probe"],
            cwd=PROJECT_ROOT,
            capture_output=True,
        )
        bin_path = PROJECT_ROOT / "target" / "debug" / "probe"

    if not bin_path.exists():
        raise RuntimeError(
            "Rust probe binary not found. Run 'python3 compare_outputs.py --build' first."
        )

    result = subprocess.run(
        [str(bin_path), template_path, data_json_path],
        capture_output=True,
    )
    if result.returncode == 2:
        # Render error (non-fatal for comparison)
        return b"__RUST_RENDER_ERROR__: " + result.stderr
    elif result.returncode != 0:
        raise RuntimeError(f"Rust probe failed (exit {result.returncode}): {result.stderr.decode()}")
    return result.stdout


def render_rust_from_model(template_path: str, data_model: dict) -> bytes:
    """Render with Rust engine using an in-memory data model dict."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        json.dump(data_model, f)
        tmp_path = f.name
    try:
        return render_rust(template_path, tmp_path)
    finally:
        os.unlink(tmp_path)


# ---------------------------------------------------------------------------
# Java probe
# ---------------------------------------------------------------------------

def render_java(template_path: str, data_json_path: str) -> bytes:
    """Render a template with Java FreeMarker, return output bytes."""
    if not JAVA_PROBE.exists():
        raise RuntimeError(f"Java probe script not found: {JAVA_PROBE}")

    result = subprocess.run(
        ["bash", str(JAVA_PROBE), template_path, data_json_path],
        capture_output=True,
    )
    if result.returncode == 2:
        # Render error (non-fatal for comparison)
        return b"__JAVA_RENDER_ERROR__: " + result.stderr
    elif result.returncode != 0:
        raise RuntimeError(f"Java probe failed (exit {result.returncode}): {result.stderr.decode()}")
    return result.stdout


def render_java_from_model(template_path: str, data_model: dict) -> bytes:
    """Render with Java FreeMarker using an in-memory data model dict."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        json.dump(data_model, f)
        tmp_path = f.name
    try:
        return render_java(template_path, tmp_path)
    finally:
        os.unlink(tmp_path)


# ---------------------------------------------------------------------------
# Comparison
# ---------------------------------------------------------------------------

def normalize_newlines(s: bytes) -> bytes:
    """Normalize line endings: CRLF and CR -> LF."""
    return s.replace(b"\r\n", b"\n").replace(b"\r", b"\n")


def diff_preview(actual: bytes, expected: bytes) -> str:
    """Generate a compact diff preview."""
    actual_str = actual.decode("utf-8", errors="replace")
    expected_str = expected.decode("utf-8", errors="replace")
    diff_lines = list(
        difflib.unified_diff(
            expected_str.splitlines(keepends=True),
            actual_str.splitlines(keepends=True),
            fromfile="expected",
            tofile="actual",
            lineterm="",
        )
    )
    if not diff_lines:
        return "no difference (encoding issue?)"
    # Return first 20 lines of diff
    preview = diff_lines[:20]
    if len(diff_lines) > 20:
        preview.append(f"... ({len(diff_lines) - 20} more lines)")
    return "\n".join(preview)


def strip_license_comment(text: bytes) -> bytes:
    """Strip /* ... */ license comment from expected file output."""
    s = text.lstrip()
    if s.startswith(b"/*"):
        end = s.find(b"*/")
        if end != -1:
            rest = s[end + 2:]
            # Strip one leading newline/CRLF
            if rest.startswith(b"\r\n"):
                rest = rest[2:]
            elif rest.startswith(b"\n"):
                rest = rest[1:]
            return rest
    return text


def compare_outputs(
    name: str,
    rust_output: bytes,
    java_output: bytes,
    expected_file: Path | None = None,
) -> dict:
    """Compare Rust and Java outputs, return result dict.

    Returns: {"status": "PASS"|"FAIL"|"ERROR", "name": ..., "detail": ...}
    """
    # Check for render errors
    if rust_output.startswith(b"__RUST_RENDER_ERROR__:"):
        return {
            "status": "ERROR",
            "name": name,
            "detail": f"Rust render error: {rust_output.decode(errors='replace')[23:].strip()}",
        }
    if java_output.startswith(b"__JAVA_RENDER_ERROR__:"):
        return {
            "status": "ERROR",
            "name": name,
            "detail": f"Java render error: {java_output.decode(errors='replace')[23:].strip()}",
        }

    rust_norm = normalize_newlines(rust_output)
    java_norm = normalize_newlines(java_output)

    # Handle trailing newline differences (Java FileTestCase.multilineAssertEquals)
    if rust_norm.endswith(b"\n") and not java_norm.endswith(b"\n"):
        java_norm += b"\n"
    elif not rust_norm.endswith(b"\n") and java_norm.endswith(b"\n"):
        java_norm = java_norm.rstrip(b"\n")

    if rust_norm == java_norm:
        return {"status": "PASS", "name": name, "detail": ""}

    # Also check against expected file if provided
    if expected_file and expected_file.exists():
        expected_raw = expected_file.read_bytes()
        expected_norm = normalize_newlines(strip_license_comment(expected_raw))
        if rust_norm.endswith(b"\n") and not expected_norm.endswith(b"\n"):
            expected_norm += b"\n"
        elif not rust_norm.endswith(b"\n") and expected_norm.endswith(b"\n"):
            expected_norm = expected_norm.rstrip(b"\n")
        if rust_norm == expected_norm:
            return {
                "status": "PASS",
                "name": name,
                "detail": "(matches expected, Java probe differs)",
            }

    detail = (
        f"Rust ({len(rust_norm)} bytes) != Java ({len(java_norm)} bytes)\n"
        + diff_preview(rust_norm, java_norm)
    )
    return {"status": "FAIL", "name": name, "detail": detail}


# ---------------------------------------------------------------------------
# Suite mode
# ---------------------------------------------------------------------------

def build_data_for_case(case: dict) -> dict | None:
    """Build a JSON-compatible data model dict for a test case.

    Returns None if the case needs complex Java-specific data models
    that cannot be represented as simple JSON (BeansWrapper, ?api, etc.).

    Many suite templates use only the common variables (message, testName)
    plus built-in assert/assertEquals directives for self-testing. These
    directives are test infrastructure and don't affect normal rendering,
    so we provide a basic model and let the template render as-is.
    """
    name = case.get("name", "")
    base = case.get("base", "")
    settings = case.get("settings", {})

    # Skip cases with Java-specific settings that affect rendering
    object_wrapper = settings.get("object_wrapper", "")
    if object_wrapper and "SimpleObjectWrapper" not in object_wrapper:
        return None  # BeansWrapper, DefaultObjectWrapper, etc.

    if "new_builtin_class_resolver" in settings:
        return None
    if settings.get("api_builtin_enabled") == "true":
        return None  # ?api builtin needs special wrapper
    if "classic_compatible" in settings:
        return None  # Classic compatibility mode changes rendering behavior

    # Common data present in all test cases (from TemplateTestCase.java:184-193)
    data: dict = {
        "message": "Hello, world!",
        "testName": name,
        "iciIntValue": 2003034,
    }

    # Per-base-name data models (mirroring build_data_model in common/mod.rs)
    if base in ("boolean",):
        data["boolean1"] = False
        data["boolean2"] = True
        data["boolean3"] = True
        data["boolean4"] = True
        data["boolean5"] = False
        data["list1"] = ["false", "0", False, True, True, True, False]
        data["list2"] = []
        data["hash1"] = {"temp": "Hello, world.", "boolean": False}
        data["hash2"] = {}
        return data

    if base in ("list", "list2", "list3", "list-bis", "listhash"):
        data["listables"] = {
            "list": [11, 22, 33],
            "linkedList": [11, 22, 33],
            "set": [11, 22, 33],
        }
        return data

    if base in ("number-format",):
        data["int"] = 1
        data["double"] = 1.0
        data["double2"] = 1.0
        data["double3"] = 1e-16
        data["double4"] = -1e-16
        data["bigDecimal"] = 1
        data["bigDecimal2"] = "1E-16"  # Will be a string in JSON
        return data

    if base in ("var-layers",):
        data["x"] = 4
        data["z"] = 4
        return data

    # Cases that work with just the common variables
    # (templates may use assert/assertEquals which will fail gracefully
    #  without the directive models, but main rendering should work)
    if base in (
        "variables", "iterators", "if", "comment", "default", "comparisons",
        "hashconcat", "precedence", "newlines1", "noparse",
        "strictinheader", "wstrip-in-header", "non-strict-syntax",
        "identifier-non-ascii", "macros2", "url", "charset-in-header",
        "arithmetic", "assignments", "compress", "then-builtin",
        "escapes", "hashliteral", "import", "include", "include2",
        "interpret", "localization", "nestedmacro", "number-format",
        "outputformat",
    ):
        return data

    # Complex cases that need special data models (methods, directives,
    # SQL dates, NaN/Infinity, multi-role models, classic-compatible, etc.)
    # These should use --suite golden instead.
    return None


def run_suite_against_expected(limit: int | None = None, filter_case: str | None = None) -> dict:
    """Run the Rust test suite and compare against expected (Java-generated) files.

    This runs `cargo test --test golden -- --nocapture` in the freemarker-test
    directory and parses the PASS/FAIL/SKIP lines from the golden_suite test output.
    """
    print("Running Rust golden suite tests (cargo test --test golden)...")
    print()

    test_dir = PROJECT_ROOT / "freemarker-test"

    result = subprocess.run(
        ["cargo", "test", "--test", "golden", "--", "--nocapture"],
        cwd=test_dir,
        capture_output=True,
        text=True,
    )

    # Merge stdout and stderr (cargo test output mixes both)
    output = result.stdout + "\n" + result.stderr

    stats = {"PASS": 0, "FAIL": 0, "SKIP": 0, "ERROR": 0, "total": 0}
    results = []

    for line in output.split("\n"):
        line_stripped = line.strip()

        # Match lines from golden_suite println! output
        if line_stripped.startswith("PASS   "):
            name = line_stripped[7:].strip()
            if filter_case and filter_case not in name:
                continue
            stats["PASS"] += 1
            stats["total"] += 1
            results.append({"status": "PASS", "name": name, "detail": ""})
            print(f"PASS   {name}")

        elif line_stripped.startswith("FAIL   "):
            rest = line_stripped[7:].strip()
            # Name is before "  " separator
            parts = rest.split("  ", 1)
            name = parts[0]
            detail = parts[1] if len(parts) > 1 else ""
            if filter_case and filter_case not in name:
                continue
            stats["FAIL"] += 1
            stats["total"] += 1
            results.append({"status": "FAIL", "name": name, "detail": detail})
            print(f"FAIL   {name}")
            if detail:
                print(f"       {detail[:200]}")

        elif line_stripped.startswith("SKIP   "):
            rest = line_stripped[7:].strip()
            # Name is before "  ("
            paren = rest.find("  (")
            if paren > 0:
                name = rest[:paren]
                reason = rest[paren + 3:].rstrip(")")
            else:
                name = rest
                reason = ""
            if filter_case and filter_case not in name:
                continue
            stats["SKIP"] += 1
            stats["total"] += 1
            results.append({"status": "SKIP", "name": name, "detail": reason})
            print(f"SKIP   {name}  ({reason})")

        if limit and stats["total"] >= limit:
            break

    # If we got 0 results, the test might have failed to run
    if stats["total"] == 0:
        print("WARNING: No suite results parsed. The golden test may have failed to compile or run.")
        print("Check the cargo test output above for errors.")

    return stats, results


def run_suite_live(limit: int | None = None, filter_case: str | None = None) -> dict:
    """Run live dual-engine comparison for suite cases that have JSON data.

    Iterates manifest.json, builds JSON data for simple cases, renders
    with both Rust and Java, and compares.
    """
    if not MANIFEST_PATH.exists():
        print(f"ERROR: Suite manifest not found: {MANIFEST_PATH}")
        sys.exit(1)

    manifest = json.loads(MANIFEST_PATH.read_text())
    cases = manifest.get("cases", [])

    stats = {"PASS": 0, "FAIL": 0, "SKIP": 0, "ERROR": 0, "total": 0}
    results = []

    for case in cases:
        name = case.get("name", "")
        base = case.get("base", "")
        template_name = case.get("template", f"{base}.ftl")

        if filter_case and filter_case not in name:
            continue

        template_path = SUITE_DIR / "cases" / base / template_name
        if not template_path.exists():
            stats["SKIP"] += 1
            stats["total"] += 1
            results.append({"status": "SKIP", "name": name, "detail": "template file not found"})
            print(f"SKIP   {name}  (template file not found)")
            if limit and stats["total"] >= limit:
                break
            continue

        # Build data model
        data_model = build_data_for_case(case)
        if data_model is None:
            stats["SKIP"] += 1
            stats["total"] += 1
            settings = case.get("settings", {})
            reason = settings.get("object_wrapper", "complex data model")
            results.append({"status": "SKIP", "name": name, "detail": f"complex/special model: {reason}"})
            print(f"SKIP   {name}  (complex/special model: {reason})")
            if limit and stats["total"] >= limit:
                break
            continue

        try:
            rust_out = render_rust_from_model(str(template_path), data_model)
            java_out = render_java_from_model(str(template_path), data_model)
        except Exception as e:
            stats["ERROR"] += 1
            stats["total"] += 1
            results.append({"status": "ERROR", "name": name, "detail": str(e)})
            print(f"ERROR  {name}  ({e})")
            if limit and stats["total"] >= limit:
                break
            continue

        result = compare_outputs(name, rust_out, java_out)
        result["status"] = result["status"]
        results.append(result)

        if result["status"] == "PASS":
            stats["PASS"] += 1
            print(f"PASS   {name}")
        elif result["status"] == "FAIL":
            stats["FAIL"] += 1
            print(f"FAIL   {name}")
            detail = result.get("detail", "")
            if detail:
                for dl in detail.split("\n")[:5]:
                    print(f"       {dl}")
        else:
            stats["ERROR"] += 1
            print(f"ERROR  {name}  {result.get('detail', '')}")
        stats["total"] += 1

        if limit and stats["total"] >= limit:
            break

    return stats, results


# ---------------------------------------------------------------------------
# Single template comparison
# ---------------------------------------------------------------------------

def run_single(
    template_path: str,
    data_path: str | None,
    java_only: bool = False,
) -> dict:
    """Run single template comparison between Rust and Java."""
    if not os.path.exists(template_path):
        print(f"ERROR: Template file not found: {template_path}")
        sys.exit(1)

    name = os.path.basename(template_path)

    if data_path and not os.path.exists(data_path):
        print(f"ERROR: Data file not found: {data_path}")
        sys.exit(1)

    # Create simple data if none provided
    if not data_path:
        with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
            json.dump({"message": "Hello, world!"}, f)
            data_path = f.name

    try:
        rust_out = render_rust(template_path, data_path) if not java_only else b""
        java_out = render_java(template_path, data_path)
    except Exception as e:
        print(f"ERROR: {e}")
        sys.exit(1)
    finally:
        if data_path and "tmp" in data_path and not os.path.exists(data_path):
            pass  # Already cleaned up
        elif data_path and "tmp" in data_path:
            os.unlink(data_path)

    if java_only:
        print(java_out.decode("utf-8", errors="replace"))
        return {"status": "OK", "name": name, "detail": ""}

    result = compare_outputs(name, rust_out, java_out)
    status = result["status"]

    if status == "PASS":
        print(f"PASS   {name}  (outputs match byte-for-byte)")
    else:
        print(f"FAIL   {name}")
        detail = result.get("detail", "")
        if detail:
            print(detail)

    return result


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="L3 Java dual-engine comparison harness for freemarker-rust",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--template",
        help="Path to FreeMarker template (.ftl) file",
    )
    parser.add_argument(
        "--data",
        help="Path to JSON data file for the template model",
    )
    parser.add_argument(
        "--java-only",
        action="store_true",
        help="Only render with Java (no comparison)",
    )
    parser.add_argument(
        "--suite",
        choices=["golden", "live"],
        help="Suite mode: 'golden' runs cargo test golden suite; 'live' does live dual-engine",
    )
    parser.add_argument(
        "--limit",
        type=int,
        help="Limit number of test cases (suite mode)",
    )
    parser.add_argument(
        "--case",
        help="Filter to specific test case name (suite mode)",
    )
    parser.add_argument(
        "--build",
        action="store_true",
        help="Build the Rust probe binary",
    )

    args = parser.parse_args()

    if args.build:
        build_rust_probe()
        # Also compile Java probe
        java_compile = SCRIPT_DIR / "java_probe" / "probe_compile.sh"
        if java_compile.exists():
            print("Compiling Java probe...")
            subprocess.run(["bash", str(java_compile)], check=False)
        print("Build complete.")
        return

    if args.suite:
        if args.suite == "golden":
            stats, results = run_suite_against_expected(
                limit=args.limit, filter_case=args.case
            )
        else:  # live
            stats, results = run_suite_live(
                limit=args.limit, filter_case=args.case
            )

        # Print summary
        print()
        print(f"{'='*60}")
        total = stats.get("total", 0)
        print(
            f"Suite results: "
            f"PASS={stats['PASS']} "
            f"FAIL={stats['FAIL']} "
            f"SKIP={stats['SKIP']} "
            f"ERROR={stats['ERROR']} "
            f"(total {total})"
        )
        print(f"{'='*60}")

        # Print failures in detail
        failures = [r for r in results if r["status"] in ("FAIL", "ERROR")]
        if failures:
            print(f"\nFailures/Errors ({len(failures)}):")
            for r in failures:
                print(f"  [{r['status']}] {r['name']}")
                detail = r.get("detail", "")
                if detail:
                    for line in detail.split("\n")[:10]:
                        print(f"    {line}")

        # Return exit code based on failures
        if stats.get("FAIL", 0) > 0 or stats.get("ERROR", 0) > 0:
            sys.exit(1)

    elif args.template:
        result = run_single(
            template_path=args.template,
            data_path=args.data,
            java_only=args.java_only,
        )
        if result["status"] == "FAIL":
            sys.exit(1)
    else:
        parser.print_help()
        print("\nNo action specified. Use --template, --suite, or --build.")


if __name__ == "__main__":
    main()
