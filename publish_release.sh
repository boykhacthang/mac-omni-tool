#!/usr/bin/env zsh
set -e

cd "$(dirname "$0")"

if [[ -z "$1" ]]; then
  echo "Cách dùng: ./publish_release.sh <VERSION> [GHI_CHÚ_CẬP_NHẬT]"
  echo "Ví dụ:    ./publish_release.sh 2.1.0 \"Tối ưu hiệu năng và thêm tính năng mới\""
  exit 1
fi

RAW_VER="$1"
VERSION="${RAW_VER#v}"
NOTES_VI="${2:-Cập nhật tính năng mới và tối ưu hiệu năng cho Mac Omni Tool v${VERSION}.}"
NOTES_EN="${3:-${NOTES_VI}}"
PUB_DATE="$(date '+%Y-%m-%d')"

echo "🚀 Bắt đầu đóng gói và phát hành Mac Omni Tool v${VERSION}..."

# 1. Cập nhật version trong Cargo.toml và bundle_app.sh
sed -i '' -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"${VERSION}\"/" Cargo.toml
sed -i '' -E "s/<string>[0-9]+\.[0-9]+\.[0-9]+<\/string>/<string>${VERSION}<\/string>/g" bundle_app.sh

# 2. Chạy bộ kiểm thử và đóng gói MacOmniTool.app + .dmg + .zip
cargo test --workspace
./build_installer_dmg.sh

# 3. Lưu bộ cài vào thư mục releases/v<VERSION>/
REL_DIR="releases/v${VERSION}"
mkdir -p "$REL_DIR"
DMG_FILE="${REL_DIR}/MacOmniTool-v${VERSION}-Installer.dmg"
ZIP_FILE="${REL_DIR}/MacOmniTool-v${VERSION}-macOS.zip"

cp "target/release/MacOmniTool-Installer.dmg" "$DMG_FILE"
cp "target/release/MacOmniTool-macOS.zip" "$ZIP_FILE"

SHA256_ZIP="$(shasum -a 256 "$ZIP_FILE" | awk '{print $1}')"
shasum -a 256 releases/v*/* > releases/SHA256SUMS.txt

# 4. Cập nhật releases/latest.json để các máy Client tự động nhận diện bản mới
cat > releases/latest.json << EOF
{
  "version": "${VERSION}",
  "published_at": "${PUB_DATE}",
  "zip_url": "https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v${VERSION}/MacOmniTool-v${VERSION}-macOS.zip",
  "dmg_url": "https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v${VERSION}/MacOmniTool-v${VERSION}-Installer.dmg",
  "sha256_zip": "${SHA256_ZIP}",
  "notes_en": $(printf '%s' "$NOTES_EN" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))'),
  "notes_vi": $(printf '%s' "$NOTES_VI" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')
}
EOF

echo "✅ Đã tạo xong releases/latest.json (SHA256: ${SHA256_ZIP})"

# 5. Commit, tạo Git Tag và đẩy lên GitHub
git add Cargo.toml Cargo.lock bundle_app.sh releases/
git commit -m "release(v${VERSION}): ${NOTES_VI}" || true
git tag -f -a "v${VERSION}" -m "Mac Omni Tool v${VERSION} — ${NOTES_VI}"
git push origin main
git push origin "v${VERSION}" --force

echo "🎉 Đã phát hành v${VERSION} lên GitHub thành công!"
echo "   🔗 Release: https://github.com/boykhacthang/mac-omni-tool/releases/tag/v${VERSION}"
echo "   🔄 Toàn bộ máy Client sẽ tự động thấy thông báo cập nhật lên v${VERSION}!"
