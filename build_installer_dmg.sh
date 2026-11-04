#!/usr/bin/env zsh
set -e

cd "$(dirname "$0")"

# 1. Build và đóng gói MacOmniTool.app mới nhất
./bundle_app.sh

APP_PATH="target/release/MacOmniTool.app"
DMG_STAGING="target/release/dmg_staging"
DMG_OUT="target/release/MacOmniTool-Installer.dmg"
ZIP_OUT="target/release/MacOmniTool-macOS.zip"

rm -rf "$DMG_STAGING" "$DMG_OUT" "$ZIP_OUT"
mkdir -p "$DMG_STAGING"

# 2. Copy MacOmniTool.app và tạo lối tắt kéo-thả vào /Applications
cp -R "$APP_PATH" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

# 3. Tạo script hỗ trợ gỡ Gatekeeper 1-click trên máy Mac mới (vì chưa đăng ký Apple Developer 99$/năm)
cat > "$DMG_STAGING/Mo-Khoa-App-Tren-May-Moi.command" << 'EOF'
#!/usr/bin/env zsh
echo "🔓 Đang gỡ cờ kiểm dịch (Quarantine) cho MacOmniTool.app..."
if [[ -d "/Applications/MacOmniTool.app" ]]; then
    xattr -cr "/Applications/MacOmniTool.app"
    echo "✅ Đã mở khóa thành công /Applications/MacOmniTool.app! Đang khởi chạy..."
    open "/Applications/MacOmniTool.app"
else
    echo "⚠️ Vui lòng kéo MacOmniTool.app vào thư mục Applications trước rồi chạy lại file này."
fi
EOF
chmod +x "$DMG_STAGING/Mo-Khoa-App-Tren-May-Moi.command"

# 4. Đóng gói thành file bộ cài .dmg chuẩn macOS
echo "📦 Đang tạo bộ cài .dmg..."
hdiutil create \
    -volname "Mac Omni Tool Installer" \
    -srcfolder "$DMG_STAGING" \
    -ov \
    -format UDZO \
    "$DMG_OUT" >/dev/null

rm -rf "$DMG_STAGING"

# 5. Tạo thêm bản nén .zip gọn nhẹ để gửi qua Zalo/Telegram/AirDrop
ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$ZIP_OUT"

echo "✅ Đã tạo xong bộ cài cho máy Mac khác:"
echo "   💿 Bộ cài DMG: $(pwd)/$DMG_OUT"
echo "   🗜️ Bản nén ZIP: $(pwd)/$ZIP_OUT"

