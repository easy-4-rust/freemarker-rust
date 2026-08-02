#!/bin/bash
# Compile ProbeRender.java against the FreeMarker jar
#
# Usage: ./probe_compile.sh
#
# Requires: javac (JDK 8+) and the FreeMarker 2.3.34 jar.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SRC_FILE="$SCRIPT_DIR/ProbeRender.java"
CLASS_DIR="$SCRIPT_DIR/classes"

# Locate the FreeMarker jar (same logic as probe.sh)
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
    if [ -f /usr/share/java/freemarker.jar ]; then
        echo /usr/share/java/freemarker.jar
        return 0
    fi
    return 1
}

JAR=$(find_jar)
if [ -z "$JAR" ]; then
    echo "ERROR: Cannot find FreeMarker jar for compilation." >&2
    echo "Download freemarker-2.3.34.jar from Maven Central and place it in ~/.m2/repository/org/freemarker/freemarker/2.3.34/" >&2
    exit 1
fi

echo "Compiling ProbeRender.java with $JAR ..."
mkdir -p "$CLASS_DIR"

javac -cp "$JAR" -d "$CLASS_DIR" "$SRC_FILE"

echo "Compilation successful. Classes written to $CLASS_DIR/"
