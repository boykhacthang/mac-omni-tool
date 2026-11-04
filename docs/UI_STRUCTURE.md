# 🎨 Cấu trúc Giao diện Modular (`docs/UI_STRUCTURE.md`)

Giao diện của **Mac Omni Tool** được thiết kế theo kiến trúc **Modular Screen & Component** thuần chuẩn ES6+ (Zero-Framework Overhead), tách biệt rõ ràng giữa **Stylesheets**, **Core Helpers**, **Reusable Components**, **Dashboard Screens** và **Floating Overlays**.

---

## 1. Sơ đồ Tổ chức Thư mục `ui/`

```text
ui/
├── index.html                        # Entrypoint HTML duy nhất nạp các module theo thứ tự
├── styles/
│   ├── base.css                      # Hệ thống Design Tokens (Dark/Light), Sidebar, Grid, Card, Form
│   ├── screens.css                   # Style riêng cho các màn hình & Bảng tiến trình Responsive
│   └── overlays.css                  # Style cho Menu Bar Popover, Clipboard Drawer, Alt-Tab, Screenshot Overlay
└── js/
    ├── locales/
    │   ├── en.js                     # Từ điển ngôn ngữ Tiếng Anh (Mặc định: window.OMNI_LOCALES.en)
    │   └── vi.js                     # Từ điển ngôn ngữ Tiếng Việt (window.OMNI_LOCALES.vi)
    ├── core/
    │   ├── api.js                    # Giao tiếp RPC api(), quản lý Theme, Toast, Formatters, Phím tắt
    │   └── i18n.js                   # Engine đa ngôn ngữ (t, tr, applyAppLang, đồng bộ đa cửa sổ)
    ├── components/
    │   ├── clipboard-card.js         # Component thẻ Clipboard dùng chung cho cả Tab & Drawer
    │   └── path-cell.js              # Component đường dẫn rút gọn + Tooltip + Copy/Finder
    ├── screens/
    │   ├── display-audio.js          # Màn hình 1: Màn hình & Âm thanh
    │   ├── hardware-monitor.js       # Màn hình 2: Phần cứng & Tiến trình (DOM Row Recycling)
    │   ├── clipboard-tab.js          # Màn hình 3: Lịch sử Clipboard & Ghim
    │   ├── window-tiling.js          # Màn hình 4: Chia cửa sổ & Phím tắt
    │   ├── mouse.js                  # Màn hình 5: Chuột rời, Cuộn ngược, Cuộn mượt 120Hz & Nút bấm theo App
    │   ├── system-cleaner.js         # Màn hình 6: Dọn rác, Tệp trùng, Gỡ App & Tệp lớn
    │   └── settings.js               # Màn hình 7: Cài đặt tổng hợp & Quyền hệ thống
    ├── overlays/
    │   ├── menubar-popover.js        # Chế độ ?mode=mini_widget (Menu Bar Popover)
    │   ├── clipboard-drawer.js       # Chế độ ?mode=clipboard_drawer (Popup Cmd+Shift+V)
    │   ├── alttab-overlay.js         # Chế độ ?mode=alttab (Alt-Tab Live Preview)
    │   └── screenshot-overlay.js     # Chế độ ?mode=screenshot_overlay&slot=N (Screenshot & Annotate)
    └── app.js                        # Khung Dashboard chính & Bộ định tuyến theo APP_MODE
```

---

## 2. Cơ chế Định tuyến Đa Cửa sổ (`APP_MODE`)

Ứng dụng khởi tạo nhiều cửa sổ `tao::window::Window` khác nhau, mỗi cửa sổ nạp cùng một `omni://localhost/index.html` với tham số truy vấn `?mode=...` riêng. Tệp [`ui/js/app.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/app.js) đọc `APP_MODE` (từ [`ui/js/core/api.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/core/api.js)) và chỉ kích hoạt đúng module của cửa sổ đó:

| Giá trị `mode` | Hàm khởi tạo | File nguồn | Mô tả cửa sổ |
| :--- | :--- | :--- | :--- |
| `main` *(mặc định)* | `initMainDashboard()` | [`ui/js/app.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/app.js) | Bảng điều khiển trung tâm với thanh menu trái và 7 màn hình chức năng. |
| `mini_widget` | `initMiniLeftWidget()` | [`ui/js/overlays/menubar-popover.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/overlays/menubar-popover.js) | Popup thả xuống dưới biểu tượng `❖` trên thanh Menu Bar. |
| `clipboard_drawer` | `initClipboardDrawer()` | [`ui/js/overlays/clipboard-drawer.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/overlays/clipboard-drawer.js) | Ngăn kéo Clipboard trượt ở cạnh trái màn hình khi nhấn `Cmd+Shift+V`. |
| `alttab` | `initAltTabOverlay()` | [`ui/js/overlays/alttab-overlay.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/overlays/alttab-overlay.js) | Hộp thoại chuyển đổi cửa sổ Live Preview ở giữa màn hình khi nhấn `Cmd+Tab`. |
| `screenshot_overlay` | `initScreenshotOverlay()` | [`ui/js/overlays/screenshot-overlay.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/overlays/screenshot-overlay.js) | Lớp phủ chụp màn hình & vẽ chú thích trên từng màn hình (`slot=0..3`). |

---

## 3. Các Component Tái sử dụng (`ui/js/components/`)

### 3.1. `clipboard-card.js`
- Được dùng chung bởi cả **Màn hình Clipboard** ([`ui/js/screens/clipboard-tab.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/screens/clipboard-tab.js)) và **Popup Clipboard cạnh trái** ([`ui/js/overlays/clipboard-drawer.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/overlays/clipboard-drawer.js)).
- Cung cấp:
  - `getSavedClipPageSize()` & `setSavedClipPageSize(val)`: Đồng bộ số mục mỗi trang (`50 / 100 / 200 / 500 / 1000`) qua `localStorage`.
  - `renderFullClipboardCardsHtml(items, selectedIdx, actionLabel, indexOffset)`: Hiển thị danh sách thẻ Clipboard với đầy đủ nội dung văn bản hoặc ảnh PNG kèm nút Dán/Copy, Ghim (`📌`) và Xóa (`✕`).

### 3.2. `path-cell.js`
- Được dùng chung bởi toàn bộ các bảng trong **Màn hình Dọn dẹp** ([`ui/js/screens/system-cleaner.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/screens/system-cleaner.js)): Thùng rác, Bộ nhớ đệm, Tệp trùng lặp, Gỡ ứng dụng và Tệp lớn.
- Cung cấp:
  - `shortenPath(fullPath)`: Rút gọn đường dẫn dài (ví dụ `/Users/khacthang/Library/Caches/Google/Chrome` -> `~/…/Google/Chrome`).
  - `renderPathCell(fullPath, includeFinderBtn)`: Hiển thị viên thuốc đường dẫn kèm Tooltip đầy đủ khi rê chuột, nút Copy đường dẫn (`📋`) và nút Mở trong Finder (`📂`).
  - `bindPathActions(container)`: Gắn sự kiện click cho các nút Copy và Finder bên trong một vùng chứa.

---

## 4. Thiết kế Responsive & Quản lý Vòng đời Màn hình (`ui/js/screens/`)

1. **Vòng đời Polling Sạch sẽ (`stopPolling`)**:
   - Hai màn hình có tác vụ lặp thời gian thực là `createHardwareMonitorScreen` ([`hardware-monitor.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/screens/hardware-monitor.js)) và `createSystemCleanerScreen` ([`system-cleaner.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/screens/system-cleaner.js)) đều trả về đối tượng `{ render, stopPolling }`.
   - Khi người dùng chuyển sang tab khác trên Sidebar, [`ui/js/app.js`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/js/app.js) tự động gọi `stopPolling()` để hủy ngay `setInterval` / `setTimeout` của màn hình cũ.
2. **Bảng Tiến trình Co giãn Thông minh (`.proc-table` trong [`ui/styles/screens.css`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/ui/styles/screens.css))**:
   - Sử dụng `table-layout: fixed` với độ rộng cột chuẩn xác.
   - Khi thu nhỏ cửa sổ hoặc chia nửa màn hình:
     - `< 980px`: Tự động ẩn cột `Đọc / Ghi Đĩa` (`.col-io`).
     - `< 780px`: Tự động ẩn thêm cột `#` (`.col-idx`) và `PID` (`.col-pid`) để nhường không gian cho tên tiến trình, thanh RAM, `% CPU` và nút `Force Quit`.
     - `< 580px`: Tự động thu gọn Sidebar trái thành thanh biểu tượng gọn (`54px`).

