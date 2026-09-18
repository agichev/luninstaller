#!/usr/bin/env bash
set -e

APP_NAME="luninstaller"
VERSION="0.1.0"
ARCH=$(dpkg --print-architecture)
PKG_DIR="pkg_deb/${APP_NAME}_${VERSION}_${ARCH}"

echo "=== 1. Building release binary ==="
cargo build --release

echo "=== 2. Preparing packaging structure ==="
rm -rf pkg_deb
mkdir -p "${PKG_DIR}/DEBIAN"
mkdir -p "${PKG_DIR}/usr/bin"
mkdir -p "${PKG_DIR}/usr/share/applications"
mkdir -p "${PKG_DIR}/usr/share/icons/hicolor/scalable/apps"

echo "=== 3. Copying binaries and resources ==="
install -m 755 "target/release/${APP_NAME}" "${PKG_DIR}/usr/bin/${APP_NAME}"
install -m 644 "resources/${APP_NAME}.desktop" "${PKG_DIR}/usr/share/applications/${APP_NAME}.desktop"
install -m 644 "resources/${APP_NAME}.svg" "${PKG_DIR}/usr/share/icons/hicolor/scalable/apps/${APP_NAME}.svg"

echo "=== 4. Creating DEBIAN/control ==="
cat << CONTROL_EOF > "${PKG_DIR}/DEBIAN/control"
Package: ${APP_NAME}
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${ARCH}
Depends: libgtk-3-0 (>= 3.24), policykit-1 | polkitd
Maintainer: Danil <agichev@gmail.com>
Description: Simple and modern GNOME application uninstaller
 A GTK application written in Rust that lists all installed user-facing
 applications from the GNOME application menu (Flatpak, Snap, APT/dpkg,
 and local desktop files) and allows uninstalling them in just a couple of clicks.
CONTROL_EOF

echo "=== 5. Building Debian package ==="
dpkg-deb --build --root-owner-group "${PKG_DIR}"
mv "${PKG_DIR}.deb" .

echo "=== Successfully built: ${APP_NAME}_${VERSION}_${ARCH}.deb ==="
