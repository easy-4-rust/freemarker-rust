#!/bin/bash
# Java FreeMarker template renderer for L3 dual-engine comparison
#
# Usage: ./probe.sh <template.ftl> [data.json]
#
# Outputs the rendered result to stdout. Errors go to stderr.
# Returns 0 on success, 1 on usage/config error, 2 on render error.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Locate the FreeMarker 2.3.34 jar
find_jar() {
    # 1. Check Maven local repository (most common)
    local m2_jar="$HOME/.m2/repository/org/freemarker/freemarker/2.3.34/freemarker-2.3.34.jar"
    if [ -f "$m2_jar" ]; then
        echo "$m2_jar"
        return 0
    fi

    # 2. Check for other FreeMarker versions in Maven cache (any 2.3.x)
    local any_jar
    any_jar=$(find "$HOME/.m2/repository/org/freemarker" -name "freemarker-2.3.*.jar" -not -name "*-sources.jar" 2>/dev/null | sort -V | tail -1)
    if [ -n "$any_jar" ]; then
        echo "$any_jar"
        return 0
    fi

    # 3. Check for system-installed
    if [ -f /usr/share/java/freemarker.jar ]; then
        echo /usr/share/java/freemarker.jar
        return 0
    fi

    return 1
}

JAR=$(find_jar)
if [ -z "$JAR" ]; then
    echo "ERROR: Cannot find FreeMarker jar. Expected at:" >&2
    echo "  \$HOME/.m2/repository/org/freemarker/freemarker/2.3.34/freemarker-2.3.34.jar" >&2
    echo "" >&2
    echo "Download it from: https://mvnrepository.com/artifact/org.freemarker/freemarker/2.3.34" >&2
    echo "Or run: mvn dependency:copy -Dartifact=org.freemarker:freemarker:2.3.34 -DoutputDirectory=/tmp" >&2
    exit 1
fi

# Check that ProbeRender.class exists, compile if needed
CLASS_DIR="$SCRIPT_DIR/classes"
CLASS_FILE="$CLASS_DIR/ProbeRender.class"

if [ ! -f "$CLASS_FILE" ]; then
    echo "INFO: ProbeRender.class not found, compiling..." >&2
    "$SCRIPT_DIR/probe_compile.sh" || exit 1
fi

# Run Java with the FreeMarker jar and compiled classes
exec java -cp "$JAR:$CLASS_DIR" ProbeRender "$@"
