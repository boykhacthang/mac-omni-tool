#!/usr/bin/env zsh
set -e

cd "$(dirname "$0")"
echo "🔨 Đang biên dịch bản Release tối ưu hóa (Apple Silicon arm64)..."
cargo build --release

# Dừng tiến trình đang chạy TRƯỚC KHI ghi đè binary để tránh treo CGEventTap / lỗi chữ ký kernel
killall mac-omni-tool 2>/dev/null || true
sleep 0.2

APP_DIR="target/release/MacOmniTool.app"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

rm -f "$APP_DIR/Contents/MacOS/mac-omni-tool"
cp "target/release/mac-omni-tool" "$APP_DIR/Contents/MacOS/mac-omni-tool"
chmod +x "$APP_DIR/Contents/MacOS/mac-omni-tool"

if [[ -f "assets/AppIcon.icns" ]]; then
    cp "assets/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"
fi

# Xóa các file binary rời cũ ngoài .app và bản trùng trong /Applications (nếu có) để Finder chỉ có duy nhất 1 app
rm -f "target/release/mac-omni-tool"
rm -f "target/debug/mac-omni-tool"
rm -rf "/Applications/MacOmniTool.app" 2>/dev/null || true
hdiutil detach "/Volumes/Mac Omni Tool Installer" -quiet 2>/dev/null || true

cat > "$APP_DIR/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Mac Omni Tool</string>
    <key>CFBundleDisplayName</key>
    <string>Mac Omni Tool</string>
    <key>CFBundleIdentifier</key>
    <string>com.khacthang.mac-omni-tool</string>
    <key>CFBundleVersion</key>
    <string>2.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>2.0.0</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © 2026 boykhacthang • MIT License • github.com/boykhacthang/mac-omni-tool</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>mac-omni-tool</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIconName</key>
    <string>AppIcon</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMultipleInstancesProhibited</key>
    <true/>
</dict>
</plist>
EOF

touch "$APP_DIR"
codesign --force --deep --sign - --identifier "com.khacthang.mac-omni-tool" "$APP_DIR" 2>/dev/null || true

if [[ "$1" == "--reset-tcc" ]]; then
    tccutil reset Accessibility com.khacthang.mac-omni-tool 2>/dev/null || true
    tccutil reset ScreenCapture com.khacthang.mac-omni-tool 2>/dev/null || true
fi

echo "✅ Đã đóng gói thành công (duy nhất 1 bản .app) tại: $(pwd)/$APP_DIR"
