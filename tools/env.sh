#!/bin/sh
# Cross-platform KiCad environment for the build scripts (sourced, not run).
# Works on Linux, macOS and Windows (Git Bash / MSYS2). Every value can be
# overridden from the environment:
#   KICAD_CLI       - kicad-cli executable
#   KPY             - Python with the `pcbnew` module (KiCad's bundled Python)
#   KICAD_FP        - KiCad standard footprints dir
#   FREEROUTING_JAR - freerouting-2.5.0.jar (only for routing)

case "$(uname -s)" in
  Darwin)
    _app=${KICAD_APP:-/Applications/KiCad/KiCad.app}
    : "${KICAD_CLI:=$_app/Contents/MacOS/kicad-cli}"
    : "${KPY:=$_app/Contents/Frameworks/Python.framework/Versions/Current/bin/python3}"
    : "${KICAD_FP:=$_app/Contents/SharedSupport/footprints}"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    # newest installed version under C:\Program Files\KiCad\<ver>
    _kd=${KICAD_DIR:-$(ls -d "/c/Program Files/KiCad/"*/ 2>/dev/null | sort -V | tail -1)}
    _kd=${_kd%/}
    : "${KICAD_CLI:=$_kd/bin/kicad-cli.exe}"
    : "${KPY:=$_kd/bin/python.exe}"
    : "${KICAD_FP:=$_kd/share/kicad/footprints}"
    export PYTHONUTF8=1           # Cyrillic file names / console output
    ;;
  *)
    : "${KICAD_CLI:=kicad-cli}"
    : "${KPY:=python3}"
    : "${KICAD_FP:=/usr/share/kicad/footprints}"
    ;;
esac
export KICAD_CLI KPY KICAD_FP

# fall back to PATH if the default location is missing
command -v "$KICAD_CLI" >/dev/null 2>&1 || KICAD_CLI=kicad-cli
command -v "$KPY" >/dev/null 2>&1 || KPY=python3
"$KPY" -c "import pcbnew" 2>/dev/null || \
  echo "warning: '$KPY' has no pcbnew module - set KPY to KiCad's Python" >&2
