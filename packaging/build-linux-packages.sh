#!/usr/bin/env bash
set -euo pipefail

readonly APP_ID="io.github.nothinglinux.nothinglinux"
readonly PACKAGE_NAME="nothing-linux"
readonly APPIMAGE_UPDATE_INFORMATION="gh-releases-zsync|Dospacite|NothingLinux|latest|nothing-linux_*_amd64.AppImage.zsync"
readonly LINUXDEPLOY_VERSION="1-alpha-20251107-1"
readonly GTK_PLUGIN_COMMIT="3b67a1d1c1b0c8268f57f2bce40fe2d33d409cea"

if [[ $# -ne 1 ]]; then
    printf 'usage: %s <version>\n' "$0" >&2
    exit 2
fi

readonly VERSION="$1"
readonly WORKSPACE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly BINARY="$WORKSPACE/target/release/nothing-linux"
readonly DIST="$WORKSPACE/dist"
readonly WORK_DIR="$(mktemp -d)"
readonly PACKAGE_ROOT="$WORK_DIR/package-root"
readonly TOOLS_DIR="$WORK_DIR/tools"

cd "$WORKSPACE"

cleanup() {
    rm -rf "$WORK_DIR"
}
trap cleanup EXIT

for command in curl fpm zsyncmake; do
    command -v "$command" >/dev/null || {
        printf 'required command not found: %s\n' "$command" >&2
        exit 1
    }
done

if [[ ! -x "$BINARY" ]]; then
    printf 'release binary not found: %s\n' "$BINARY" >&2
    printf 'run cargo build --release -p nothing-linux first\n' >&2
    exit 1
fi

case "$(uname -m)" in
    x86_64)
        readonly DEB_ARCH="amd64"
        readonly RPM_ARCH="x86_64"
        readonly LINUXDEPLOY_ARCH="x86_64"
        ;;
    *)
        printf 'unsupported packaging architecture: %s\n' "$(uname -m)" >&2
        exit 1
        ;;
esac

rm -rf "$DIST"
mkdir -p \
    "$DIST" \
    "$PACKAGE_ROOT/usr/bin" \
    "$PACKAGE_ROOT/usr/share/applications" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/scalable/apps" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/48x48/apps" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/64x64/apps" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/128x128/apps" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/256x256/apps" \
    "$PACKAGE_ROOT/usr/share/metainfo" \
    "$TOOLS_DIR"

install -m 0755 "$BINARY" "$PACKAGE_ROOT/usr/bin/nothing-linux"
install -m 0644 \
    "$WORKSPACE/data/$APP_ID.desktop" \
    "$PACKAGE_ROOT/usr/share/applications/$APP_ID.desktop"
install -m 0644 \
    "$WORKSPACE/data/$APP_ID.metainfo.xml" \
    "$PACKAGE_ROOT/usr/share/metainfo/$APP_ID.metainfo.xml"
install -m 0644 \
    "$WORKSPACE/data/icons/$APP_ID.svg" \
    "$PACKAGE_ROOT/usr/share/icons/hicolor/scalable/apps/$APP_ID.svg"
for icon_size in 48 64 128 256; do
    install -m 0644 \
        "$WORKSPACE/data/icons/hicolor/${icon_size}x${icon_size}/apps/$APP_ID.png" \
        "$PACKAGE_ROOT/usr/share/icons/hicolor/${icon_size}x${icon_size}/apps/$APP_ID.png"
done

common_fpm_args=(
    --input-type dir
    --chdir "$PACKAGE_ROOT"
    --name "$PACKAGE_NAME"
    --version "$VERSION"
    --iteration 1
    --license GPL-3.0-or-later
    --url https://github.com/Dospacite/NothingLinux
    --maintainer "Nothing Linux contributors"
    --vendor "Nothing Linux contributors"
    --category sound
    --description "Local desktop controller for Nothing Ear (2024) earbuds"
)

fpm "${common_fpm_args[@]}" \
    --output-type deb \
    --architecture "$DEB_ARCH" \
    --depends 'libgtk-4-1 (>= 4.14)' \
    --depends 'libadwaita-1-0 (>= 1.5)' \
    --depends libdbus-1-3 \
    --depends libbluetooth3 \
    --package "$DIST/${PACKAGE_NAME}_${VERSION}_${DEB_ARCH}.deb" \
    .

readonly LINUXDEPLOY="$TOOLS_DIR/linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage"
readonly GTK_PLUGIN="$TOOLS_DIR/linuxdeploy-plugin-gtk.sh"
curl --fail --location --silent --show-error \
    "https://github.com/linuxdeploy/linuxdeploy/releases/download/${LINUXDEPLOY_VERSION}/linuxdeploy-${LINUXDEPLOY_ARCH}.AppImage" \
    --output "$LINUXDEPLOY"
curl --fail --location --silent --show-error \
    "https://raw.githubusercontent.com/linuxdeploy/linuxdeploy-plugin-gtk/${GTK_PLUGIN_COMMIT}/linuxdeploy-plugin-gtk.sh" \
    --output "$GTK_PLUGIN"
chmod +x "$LINUXDEPLOY" "$GTK_PLUGIN"

export APPIMAGE_EXTRACT_AND_RUN=1
export DEPLOY_GTK_VERSION=4
export LINUXDEPLOY_PLUGIN_GTK="$GTK_PLUGIN"
export LDAI_OUTPUT="$DIST/${PACKAGE_NAME}_${VERSION}_${DEB_ARCH}.AppImage"
# Keep the update channel with the release assets themselves. linuxdeploy
# passes this to appimagetool, which embeds it in the AppImage and writes the
# matching zsync control file next to the generated artifact.
export LDAI_UPDATE_INFORMATION="$APPIMAGE_UPDATE_INFORMATION"
readonly OUTPUT="$LDAI_OUTPUT"

"$LINUXDEPLOY" \
    --appdir "$PACKAGE_ROOT" \
    --executable "$PACKAGE_ROOT/usr/bin/nothing-linux" \
    --desktop-file "$PACKAGE_ROOT/usr/share/applications/$APP_ID.desktop" \
    --icon-file "$PACKAGE_ROOT/usr/share/icons/hicolor/256x256/apps/$APP_ID.png" \
    --plugin gtk

# Gear Lever reads .DirIcon before themed icon locations. linuxdeploy creates
# that link from the desktop icon name, which can select the scalable SVG even
# when the PNG was supplied above. Keep the root link explicitly PNG so the
# copied desktop-integration icon has a reliable file type and extension.
ln -sfn "usr/share/icons/hicolor/256x256/apps/$APP_ID.png" "$PACKAGE_ROOT/.DirIcon"

# QuickShell's StatusNotifier implementation uses IconThemePath as a direct
# base path. Provide an extensionless alias matching the standard IconName so
# it can load the icon without relying on the host icon cache.
ln -sfn "usr/share/icons/hicolor/256x256/apps/$APP_ID.png" "$PACKAGE_ROOT/$APP_ID"

# Nothing Linux does not use GTK's optional GStreamer media backend. The GTK
# plugin can bundle a host-built copy whose GLib ABI does not match the bundled
# runtime, producing a startup loader error even though no media is used.
find "$PACKAGE_ROOT/usr/lib/gtk-4.0" \
    -path '*/media/libmedia-gstreamer.so' \
    -type f \
    -delete

"$LINUXDEPLOY" \
    --appdir "$PACKAGE_ROOT" \
    --output appimage

chmod +x "$OUTPUT"
readonly GENERATED_ZSYNC="$WORKSPACE/$(basename "$OUTPUT").zsync"
if [[ -s "$GENERATED_ZSYNC" ]]; then
    mv "$GENERATED_ZSYNC" "$OUTPUT.zsync"
fi
[[ -s "$OUTPUT.zsync" ]] || {
    printf 'AppImage update metadata was not generated: %s\n' "$OUTPUT.zsync" >&2
    exit 1
}

fpm "${common_fpm_args[@]}" \
    --output-type rpm \
    --architecture "$RPM_ARCH" \
    --depends 'gtk4 >= 4.14' \
    --depends 'libadwaita >= 1.5' \
    --depends dbus-libs \
    --depends bluez-libs \
    --package "$DIST/${PACKAGE_NAME}-${VERSION}-1.${RPM_ARCH}.rpm" \
    .

printf 'Created release packages in %s:\n' "$DIST"
find "$DIST" -maxdepth 1 -type f -printf '  %f\n' | sort
