#!/bin/bash
# Java FreeMarker error message probe for the M5 error-alignment milestone.
#
# Usage: ./probe_errors.sh [output-dir]
#
# Renders ~45 failing templates with FreeMarker 2.3.34 and writes one
# baseline file per scenario into <output-dir>/<scenario>.txt
# (default output-dir: ../freemarker/src/error/expected_messages/).
# Prints one JSON line per scenario to stdout.
#
# The .txt baseline contains the FULL Java error message
# (TemplateException.getMessage(), including the FTL stack trace section).
#
# Returns 0 on success, 1 on usage/config error.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Locate the FreeMarker 2.3.34 jar
find_jar() {
    local m2_jar="$HOME/.m2/repository/org/freemarker/freemarker/2.3.34/freemarker-2.3.34.jar"
    if [ -f "$m2_jar" ]; then
        echo "$m2_jar"
        return 0
    fi
    local any_jar
    any_jar=$(find "$HOME/.m2/repository/org/freemarker" -name "freemarker-2.3.*.jar" -not -name "*-sources.jar" 2>/dev/null | sort -V | tail -1)
    if [ -n "$any_jar" ]; then
        echo "$any_jar"
        return 0
    fi
    return 1
}

JAR=$(find_jar)
if [ -z "$JAR" ]; then
    echo "ERROR: Cannot find FreeMarker jar. Expected at:" >&2
    echo "  \$HOME/.m2/repository/org/freemarker/freemarker/2.3.34/freemarker-2.3.34.jar" >&2
    exit 1
fi

OUT_DIR="${1:-$SCRIPT_DIR/../../freemarker/src/error/expected_messages}"
mkdir -p "$OUT_DIR"

CLASS_DIR="$SCRIPT_DIR/classes"
CLASS_FILE="$CLASS_DIR/ProbeErrors.class"

if [ ! -f "$CLASS_FILE" ]; then
    echo "INFO: ProbeErrors.class not found, compiling..." >&2
    mkdir -p "$CLASS_DIR"
    javac -cp "$JAR" -d "$CLASS_DIR" "$SCRIPT_DIR/ProbeErrors.java"
fi

exec java -cp "$JAR:$CLASS_DIR" ProbeErrors "$OUT_DIR"
