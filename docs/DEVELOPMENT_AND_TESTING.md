# 🧪 Hướng dẫn Phát triển, Kiểm thử & Đóng gói (`docs/DEVELOPMENT_AND_TESTING.md`)

Tài liệu này hướng dẫn quy trình phát triển tính năng mới, chạy bộ kiểm thử tự động (`cargo test`) và đóng gói bộ cài `.dmg` cho các máy Mac khác.

---

## 1. Yêu cầu Môi trường

- **Hệ điều hành**: macOS 12.0+ (Hỗ trợ cả Apple Silicon `aarch64-apple-darwin` và Intel `x86_64-apple-darwin`).
- **Rust Toolchain**: `rustc` & `cargo` (Edition 2021, khuyến nghị bản stable mới nhất).
- **Xcode Command Line Tools**: Cung cấp `clang`, `codesign`, `sips`, `iconutil`, `hdiutil`.

---

## 2. Chạy Bộ Kiểm thử Tự động (`cargo test`)

Dự án bao gồm **26 bài kiểm thử tự động** trải đều trên tất cả 6 thư viện lõi, crate thực thi chính và bộ kiểm thử tích hợp:

```bash
# Chạy toàn bộ Unit Tests & Integration Tests trên toàn Workspace
cargo test --workspace
```

### Danh sách các bộ Test theo từng Crate:

1. **`core-monitor` (5 unit tests)** — [`crates/core-monitor/src/lib.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/crates/core-monitor/src/lib.rs):
   - `test_classify_sensor_category`: Kiểm tra phân loại cảm biến nhiệt độ (`CPU/SoC`, `GPU`, `Battery/PMU`, `Storage/Other`).
   - `test_summarize_sensor_readings_dedup_and_avg`: Kiểm tra khử trùng lặp cảm biến và tính nhiệt độ trung bình/cực đại.
   - `test_parse_pmset_battery_output`: Kiểm tra giải mã đầu ra `pmset -g batt` cho cả Pin sạc và nguồn AC.
   - `test_parse_ioreg_gpu_output`: Kiểm tra bóc tách thông số GPU Apple Silicon (`Device Utilization %`, `Renderer`, `Tiler`, `VRAM`).
   - `test_process_sorting_and_kill_guards`: Kiểm tra sắp xếp tiến trình (`ram`, `cpu`, `disk`, `name`) và chặn xóa nhầm PID hệ thống (`0`, `1`, hoặc chính ứng dụng).
2. **`core-display` (3 unit tests)** — [`crates/core-display/src/lib.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/crates/core-display/src/lib.rs):
   - `test_compute_gamma_channels_bounds_and_warmth`: Kiểm tra công thức nội suy kênh màu R/G/B khi giảm sáng sâu và tăng độ ấm màu.
   - `test_find_display_containing_point_multi_monitor`: Kiểm tra thuật toán xác định màn hình chứa con trỏ chuột trên thiết lập đa màn hình.
   - `test_classify_audio_device_types`: Kiểm tra nhận diện AirPods, Tai nghe, Loa tích hợp, HDMI/DisplayPort.
3. **`core-clipboard` (4 unit tests)** — [`crates/core-clipboard/src/lib.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/crates/core-clipboard/src/lib.rs):
   - `test_fnv1a_hash_deterministic`: Kiểm tra tính nhất quán của hàm băm FNV-1a 64-bit.
   - `test_base64_encode_and_decode_roundtrip` & `test_decode_base64_invalid_errors`: Kiểm tra bộ mã hóa/giải mã Base64 và Data URL PNG.
   - `test_clipboard_sqlite_crud_and_pagination`: Kiểm tra thêm mới, ghim, xóa và phân trang SQLite.
4. **`core-window` (2 unit tests)** — [`crates/core-window/src/lib.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/crates/core-window/src/lib.rs):
   - `test_compute_snap_rect_layouts`: Kiểm tra tọa độ hình học của các bố cục chia cửa sổ (`left_half`, `right_half`, `bottom_right`, `first_two_thirds`, `maximize`).
   - `test_window_shortcuts_conflict_detection`: Kiểm tra phát hiện trùng phím tắt khi cập nhật.
5. **`core-input` (3 unit tests)** — [`crates/core-input/src/lib.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/crates/core-input/src/lib.rs):
   - `test_canonicalize_and_shortcuts_match`: Kiểm tra chuẩn hóa thứ tự phím bổ trợ (`Ctrl+Option+Cmd+Shift+Key`).
   - `test_shortcut_to_carbon_keycode_and_modifiers`: Kiểm tra ánh xạ phím tắt sang mã phím Carbon (`RegisterEventHotKey`).
   - `test_keycode_and_flags_to_shortcut`: Kiểm tra dịch ngược sự kiện `CGEventTap` sang chuỗi phím tắt.
6. **`core-cleaner` (4 unit tests)** — [`crates/core-cleaner/src/lib.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/crates/core-cleaner/src/lib.rs):
   - `test_classify_file_kinds`: Kiểm tra phân loại định dạng tệp.
   - `test_extract_plist_key`: Kiểm tra đọc khóa XML trong `Info.plist`.
   - `test_internal_db_engine_vs_normal_sql_file`: Đảm bảo tệp `.sql` thông thường được quét trùng lặp trong khi tệp engine nội bộ (`.ibd`, `.frm`, `PG_VERSION`, `.db-wal`) được khóa bảo vệ.
   - `test_should_skip_duplicate_dir_rules`: Kiểm tra quy tắc loại trừ thư mục theo tên hoặc đường dẫn.
7. **`mac-omni-tool` Binary Crate (2 unit tests)** — [`src/ui_assets.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/src/ui_assets.rs) & [`src/rpc.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/src/rpc.rs):
   - `test_all_ui_assets_referenced_in_index_html_resolve`: Đảm bảo 100% các tệp `<link>` CSS và `<script>` JS khai báo trong `ui/index.html` đều được nhúng và phản hồi hợp lệ qua `resolve_ui_asset()`.
   - `test_deserialize_api_request_with_and_without_payload`: Kiểm tra giải mã gói tin JSON-RPC.
8. **Integration Tests (3 integration tests)** — [`tests/integration_test.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/tests/integration_test.rs):
   - Kiểm thử tích hợp xuyên suốt cả 6 crate trên môi trường macOS thực tế.

---

## 3. Quy trình Thêm một Màn hình (Screen) Mới

Nhờ kiến trúc đã được tách nhỏ, việc thêm một màn hình mới cực kỳ gọn gàng:

1. **Tạo file JS màn hình mới**: Tạo `ui/js/screens/<ten-man-hinh>.js` xuất hàm `render<TenManHinh>Screen(view)` (hoặc `create<TenManHinh>Screen()` nếu có bộ đếm thời gian).
2. **Đăng ký Asset**:
   - Thêm thẻ `<script src="/js/screens/<ten-man-hinh>.js"></script>` vào [`ui/index.html`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/index.html).
   - Khai báo `include_str!` tương ứng trong [`src/ui_assets.rs`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/src/ui_assets.rs).
3. **Gắn vào Sidebar Router**: Thêm mục `.nav-item` và nhánh điều hướng trong [`ui/js/app.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/app.js).
4. **Kiểm tra**: Chạy `cargo test --workspace` — bài test `test_all_ui_assets_referenced_in_index_html_resolve` sẽ tự động kiểm tra tính toàn vẹn của tài nguyên mới.

---

## 4. Đóng gói Bản cài đặt `.app` và `.dmg`

Sử dụng script [`build_installer_dmg.sh`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/build_installer_dmg.sh):

```bash
# Build cho máy hiện tại + tạo file dist/MacOmniTool-Installer.dmg + cài vào /Applications/MacOmniTool.app
./build_installer_dmg.sh

# Hoặc build bản Universal Binary chạy được trên cả Mac chip M (Apple Silicon) và Mac chip Intel:
./build_installer_dmg.sh --universal
```

Sau khi chạy xong:
- Ứng dụng được cập nhật tại `/Applications/MacOmniTool.app` (đã ký ad-hoc `codesign` với identifier cố định `com.khacthang.mac-omni-tool` để giữ nguyên quyền Accessibility / Screen Recording).
- Bộ cài đặt kéo-thả nằm tại `dist/MacOmniTool-Installer.dmg`.

