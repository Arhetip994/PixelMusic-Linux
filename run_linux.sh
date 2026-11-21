#!/usr/bin/env bash
# Runs PixelMusic from source on Linux: cargo run --release with Qt 6
# Arguments are passed to the app: ./run_linux.sh --new-instance --light
set -euo pipefail

# Ensure Cargo/Rust is in PATH
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "Error: cargo not found. Install Rust: https://rustup.rs" >&2
    exit 1
fi

# Locate Qt 6 qmake
QMAKE="${PIXELMUSIC_QMAKE:-${QMAKE:-}}"
if [ -z "$QMAKE" ]; then
    for q in "$(command -v qmake6 || true)" \
             "$(command -v qmake || true)" \
             /usr/lib/qt6/bin/qmake6 \
             /usr/lib64/qt6/bin/qmake6 \
             /usr/lib/qt6/bin/qmake \
             "$HOME/Qt/6.12.0/gcc_64/bin/qmake6"; do
        if [ -n "$q" ] && [ -x "$q" ]; then
            ver="$("$q" -query QT_VERSION 2>/dev/null || echo 0)"
            major="${ver%%.*}"
            if [ "$major" = "6" ]; then
                QMAKE="$q"
                break
            fi
        fi
    done
fi

if [ -z "$QMAKE" ] || [ ! -x "$QMAKE" ]; then
    echo "Error: Qt 6 (qmake6) not found. Set PIXELMUSIC_QMAKE=/path/to/qmake6 or install qt6-base." >&2
    exit 1
fi

QTDIR="$("$QMAKE" -query QT_INSTALL_PREFIX 2>/dev/null || true)"
QT_LIBS="$("$QMAKE" -query QT_INSTALL_LIBS 2>/dev/null || true)"

export QMAKE
if [ -n "$QT_LIBS" ] && [ -d "$QT_LIBS" ]; then
    export LD_LIBRARY_PATH="$QT_LIBS${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

PROFILE="--release"
APP_ARGS=()
while [ $# -gt 0 ]; do
    case "$1" in
        --debug)
            PROFILE=""
            shift
            ;;
        --release)
            PROFILE="--release"
            shift
            ;;
        *)
            APP_ARGS+=("$1")
            shift
            ;;
    esac
done

exec cargo run --locked ${PROFILE:+$PROFILE} -p pixelmusic -- "${APP_ARGS[@]}"
