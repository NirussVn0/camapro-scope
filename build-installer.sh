#!/usr/bin/env bash
# Build camapro-scope portable tarball (Arch/CachyOS-friendly installer).
# ponytail: assumes system webkit2gtk/gstreamer present (docs/PLATFORMS.md
# reference-distro assumption). Upgrade path: CI AppImage once linuxdeploy
# network/fuse issues resolved.
set -eu
cd "$(dirname "$0")"
BIN=desktop/src-tauri/target/release/camapro-scope
[ -x "$BIN" ] || { echo "build first: cd desktop && pnpm tauri build --no-bundle"; exit 1; }
STAGE=$(mktemp -d)
mkdir -p "$STAGE/bin" "$STAGE/share/applications" "$STAGE/share/icons/hicolor/256x256/apps"
cp "$BIN" "$STAGE/bin/camapro-scope"
cp desktop/src-tauri/icons/128x128.png "$STAGE/share/icons/hicolor/256x256/apps/camapro-scope.png"
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
tar czf camapro-scope-0.1.0-linux-x86_64.tar.gz -C "$STAGE" .
rm -rf "$STAGE"
echo "BUILT $(du -h camapro-scope-0.1.0-linux-x86_64.tar.gz)"
