#!/usr/bin/env bash
# Build camapro-scope portable tarball installer + Android APK, with optional GitHub release.
# Usage:
#   ./build-installer.sh                  # Builds both Linux desktop and Android APK
#   ./build-installer.sh --desktop-only   # Builds only Linux desktop package
#   ./build-installer.sh --apk-only       # Builds only Android APK
#   ./build-installer.sh --release        # Builds both and creates GitHub release via gh
#   ./build-installer.sh --release --tag v0.2.0

set -euo pipefail
cd "$(dirname "$0")"
ROOT_DIR="$(pwd)"

DO_DESKTOP=true
DO_APK=true
DO_REBUILD=true
DO_RELEASE=false
TAG=""
TITLE=""
NOTES=""
NOTES_FILE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --desktop-only|--desktop)
      DO_DESKTOP=true
      DO_APK=false
      shift
      ;;
    --apk-only|--apk)
      DO_DESKTOP=false
      DO_APK=true
      shift
      ;;
    --skip-desktop)
      DO_DESKTOP=false
      shift
      ;;
    --skip-apk)
      DO_APK=false
      shift
      ;;
    --no-rebuild)
      DO_REBUILD=false
      shift
      ;;
    --release)
      DO_RELEASE=true
      shift
      ;;
    --tag)
      TAG="$2"
      shift 2
      ;;
    --title)
      TITLE="$2"
      shift 2
      ;;
    --notes)
      NOTES="$2"
      shift 2
      ;;
    --notes-file)
      NOTES_FILE="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: $0 [options]"
      echo ""
      echo "Options:"
      echo "  --desktop-only     Build only Linux desktop installer"
      echo "  --apk-only         Build only Android APK"
      echo "  --skip-desktop     Skip desktop build"
      echo "  --skip-apk         Skip APK build"
      echo "  --no-rebuild       Package existing build outputs without re-compiling"
      echo "  --release          Publish build artifacts as a GitHub release (via gh CLI)"
      echo "  --tag <tag>        Specify release tag (default: v<version>)"
      echo "  --title <title>    Specify release title"
      echo "  --notes <text>     Specify release notes"
      echo "  --notes-file <file> Specify file containing release notes"
      echo "  -h, --help         Show this help message"
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

# Read product version
VERSION="${VERSION:-}"
if [ -z "$VERSION" ]; then
  if [ -f "desktop/src-tauri/tauri.conf.json" ]; then
    VERSION=$(grep -m1 '"version":' desktop/src-tauri/tauri.conf.json | tr -d ' ",' | cut -d: -f2)
  fi
fi
VERSION="${VERSION:-0.1.0}"
echo "=========================================="
echo "  Camapro Scope Build & Packaging (v${VERSION})"
echo "=========================================="

if [ "$DO_REBUILD" = true ]; then
  rm -rf "$ROOT_DIR/dist"
fi
mkdir -p "$ROOT_DIR/dist"

detect_jdk() {
  if [ -n "${JAVA_HOME:-}" ] && [ -x "${JAVA_HOME}/bin/javac" ]; then
    return 0
  fi
  # Search known JDK locations
  local candidates=(
    "/home/nirussvn0/Android/jdk/jdk-21.0.12.1+1"
    "${HOME}/Android/jdk/jdk-21.0.12.1+1"
    "/opt/android-studio/jbr"
    "/usr/lib/jvm/java-21-openjdk"
    "/usr/lib/jvm/default"
  )
  for cand in "${candidates[@]}"; do
    if [ -x "$cand/bin/javac" ]; then
      export JAVA_HOME="$cand"
      export PATH="$JAVA_HOME/bin:$PATH"
      return 0
    fi
  done
  # Wildcard search for user android jdk
  for cand in "${HOME}/Android/jdk"/*; do
    if [ -x "$cand/bin/javac" ]; then
      export JAVA_HOME="$cand"
      export PATH="$JAVA_HOME/bin:$PATH"
      return 0
    fi
  done
  return 1
}

detect_android_sdk() {
  if [ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME" ]; then
    return 0
  fi
  local candidates=(
    "${HOME}/Android/Sdk"
    "/opt/android-sdk"
  )
  for cand in "${candidates[@]}"; do
    if [ -d "$cand" ]; then
      export ANDROID_HOME="$cand"
      export ANDROID_SDK_ROOT="$cand"
      return 0
    fi
  done
  return 1
}

build_desktop() {
  echo ""
  echo "==> [1/2] Building Linux desktop..."
  BIN="desktop/src-tauri/target/release/camapro-scope"

  if [ "$DO_REBUILD" = true ] || [ ! -x "$BIN" ]; then
    echo "--> Compiling release binary (pnpm tauri build --no-bundle)..."
    (cd "$ROOT_DIR/desktop" && pnpm tauri build --no-bundle)
  else
    echo "--> Using existing binary: $BIN"
  fi

  [ -x "$BIN" ] || { echo "ERROR: Binary $BIN not found or not executable" >&2; exit 1; }

  echo "--> Creating portable installer stage..."
  STAGE=$(mktemp -d)
  mkdir -p "$STAGE/bin" "$STAGE/share/applications" "$STAGE/share/icons/hicolor/256x256/apps"
  cp "$BIN" "$STAGE/bin/camapro-scope"
  cp "$ROOT_DIR/desktop/src-tauri/icons/128x128.png" "$STAGE/share/icons/hicolor/256x256/apps/camapro-scope.png"

  cat > "$STAGE/share/applications/camapro-scope.desktop" <<'EOF'
[Desktop Entry]
Name=Camapro Scope
Exec=camapro-scope
Icon=camapro-scope
Type=Application
Categories=Video;AudioVideo;
EOF

  cat > "$STAGE/install.sh" <<'EOF'
#!/usr/bin/env bash
# Install into ~/.local (bin + desktop entry + icon). Uninstall: delete the 3 files listed.
set -eu
cd "$(dirname "$0")"
PREFIX="${1:-$HOME/.local}"
mkdir -p "$PREFIX/bin" "$PREFIX/share/applications" "$PREFIX/share/icons/hicolor/256x256/apps"
cp bin/camapro-scope "$PREFIX/bin/"
cp share/applications/camapro-scope.desktop "$PREFIX/share/applications/"
cp share/icons/hicolor/256x256/apps/camapro-scope.png "$PREFIX/share/icons/hicolor/256x256/apps/"
echo "Installed: $PREFIX/bin/camapro-scope"
echo "Remove:    rm $PREFIX/bin/camapro-scope $PREFIX/share/applications/camapro-scope.desktop $PREFIX/share/icons/hicolor/256x256/apps/camapro-scope.png"
EOF
  chmod +x "$STAGE/install.sh"

  TARBALL="$ROOT_DIR/dist/camapro-scope-${VERSION}-linux-x86_64.tar.gz"
  tar czf "$TARBALL" -C "$STAGE" .
  rm -rf "$STAGE"

  # Also copy to root for backwards compatibility
  cp "$TARBALL" "$ROOT_DIR/camapro-scope-${VERSION}-linux-x86_64.tar.gz"
  echo "✓ Built Linux tarball: $TARBALL ($(du -h "$TARBALL" | cut -f1))"
}

build_apk() {
  echo ""
  echo "==> [2/2] Building Android APK..."
  if ! detect_jdk; then
    echo "ERROR: Could not locate a working JDK with javac. Please set JAVA_HOME." >&2
    exit 1
  fi
  if ! detect_android_sdk; then
    echo "ERROR: Could not locate Android SDK. Please set ANDROID_HOME." >&2
    exit 1
  fi

  echo "--> Using JAVA_HOME: $JAVA_HOME"
  echo "--> Using ANDROID_HOME: $ANDROID_HOME"

  if [ "$DO_REBUILD" = true ] || [ ! -f "android/app/build/outputs/apk/debug/app-debug.apk" ]; then
    echo "--> Running gradle assembleDebug..."
    (cd "$ROOT_DIR/android" && ./gradlew assembleDebug)
  else
    echo "--> Using existing APK from android/app/build/outputs/apk/debug/app-debug.apk"
  fi

  APK_SRC="$ROOT_DIR/android/app/build/outputs/apk/debug/app-debug.apk"
  [ -f "$APK_SRC" ] || { echo "ERROR: APK not found at $APK_SRC" >&2; exit 1; }

  cp "$APK_SRC" "$ROOT_DIR/dist/app-debug.apk"
  cp "$APK_SRC" "$ROOT_DIR/dist/camapro-scope-${VERSION}-android-debug.apk"
  cp "$APK_SRC" "$ROOT_DIR/app-debug.apk"

  echo "✓ Built Android APK: $ROOT_DIR/dist/app-debug.apk ($(du -h "$ROOT_DIR/dist/app-debug.apk" | cut -f1))"
}

if [ "$DO_DESKTOP" = true ]; then
  build_desktop
fi

if [ "$DO_APK" = true ]; then
  build_apk
fi

# Generate Checksums
echo ""
echo "==> Generating SHA256 checksums..."
cd "$ROOT_DIR/dist"
rm -f SHA256SUMS.txt
sha256sum camapro-scope-*.tar.gz *.apk > SHA256SUMS.txt
cat SHA256SUMS.txt
cd "$ROOT_DIR"

echo ""
echo "=========================================="
echo "  Build completed successfully!"
echo "  Artifacts available in: $ROOT_DIR/dist/"
ls -lh "$ROOT_DIR/dist"
echo "=========================================="

if [ "$DO_RELEASE" = true ]; then
  echo ""
  echo "==> Creating GitHub release..."
  if ! command -v gh >/dev/null 2>&1; then
    echo "ERROR: 'gh' CLI is required for --release but not found in PATH." >&2
    exit 1
  fi

  RELEASE_TAG="${TAG:-v${VERSION}}"
  RELEASE_TITLE="${TITLE:-Camapro Scope ${VERSION}}"

  RELEASE_FILES=(
    "$ROOT_DIR/dist/camapro-scope-${VERSION}-linux-x86_64.tar.gz"
    "$ROOT_DIR/dist/app-debug.apk"
    "$ROOT_DIR/dist/SHA256SUMS.txt"
  )

  # Check if release exists
  if gh release view "$RELEASE_TAG" >/dev/null 2>&1; then
    echo "--> Release $RELEASE_TAG already exists. Uploading/updating assets..."
    gh release upload "$RELEASE_TAG" "${RELEASE_FILES[@]}" --clobber
  else
    echo "--> Creating new release $RELEASE_TAG..."
    CMD=(gh release create "$RELEASE_TAG" "${RELEASE_FILES[@]}" --title "$RELEASE_TITLE")
    if [ -n "$NOTES_FILE" ] && [ -f "$NOTES_FILE" ]; then
      CMD+=(--notes-file "$NOTES_FILE")
    elif [ -n "$NOTES" ]; then
      CMD+=(--notes "$NOTES")
    else
      CMD+=(--generate-notes)
    fi
    "${CMD[@]}"
  fi

  echo "✓ GitHub release published: $(gh release view "$RELEASE_TAG" --json url -q .url)"
fi
