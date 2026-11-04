#!/bin/bash
# Bouwt een macOS .app-bundel (en .dmg) van de Speeltoestel Calculator.
# Dubbelklikken in Finder opent dan alleen de GUI, zonder terminal.
set -euo pipefail

cd "$(dirname "$0")"

APP_NAME="Speeltoestel Calculator"
BIN_NAME="speeltoestel-calculator"
VERSION="1.0.0"
BUNDLE_ID="nl.bobkosse.speeltoestel-calculator"

echo "==> cargo build --release"
cargo build --release

APP="target/release/${APP_NAME}.app"
DMG="target/release/Speeltoestel-Calculator-${VERSION}.dmg"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "target/release/${BIN_NAME}" "$APP/Contents/MacOS/${APP_NAME}"
chmod +x "$APP/Contents/MacOS/${APP_NAME}"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>${APP_NAME}</string>
	<key>CFBundleDisplayName</key>
	<string>${APP_NAME}</string>
	<key>CFBundleIdentifier</key>
	<string>${BUNDLE_ID}</string>
	<key>CFBundleVersion</key>
	<string>${VERSION}</string>
	<key>CFBundleShortVersionString</key>
	<string>${VERSION}</string>
	<key>CFBundleExecutable</key>
	<string>${APP_NAME}</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>NSHighResolutionCapable</key>
	<true/>
	<key>LSMinimumSystemVersion</key>
	<string>10.13</string>
</dict>
</plist>
PLIST

echo -n "APPL????" > "$APP/Contents/PkgInfo"

# Optioneel app-icoon: leg app/assets/AppIcon.iconset (PNG's 16-512px) neer,
# dan wordt met iconutil automatisch AppIcon.icns gemaakt.
if command -v iconutil >/dev/null 2>&1 && [ -d "app/assets/AppIcon.iconset" ]; then
  echo "==> icoon genereren"
  iconutil -c icns "app/assets/AppIcon.iconset" -o "$APP/Contents/Resources/AppIcon.icns"
fi

echo "==> .app-bundel klaar: ${APP}"

if command -v hdiutil >/dev/null 2>&1; then
  echo "==> .dmg maken"
  rm -f "$DMG"
  hdiutil create -volname "${APP_NAME}" -srcfolder "$APP" -ov "$DMG"
  echo "==> .dmg klaar: ${DMG}"
fi

echo ""
echo "Klaar. Dubbelklik ${APP} in Finder — er verschijnt geen terminal."
echo "Let op: bij de eerste keer openen kan macOS Gatekeeper melden dat de app"
echo "van een onbekende ontwikkelaar komt. Rechtermuisknop → Open lost dat op."
