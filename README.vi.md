# 🛠️ Mac Omni Tool — Bộ Công Cụ Hệ Thống Đa Năng Cho macOS (Bản Cộng Đồng Miễn Phí)

> 🌐 **Language / Ngôn ngữ**: [🇬🇧 English](README.md) | **[🇻🇳 Tiếng Việt](README.vi.md)**

[![Edition](https://img.shields.io/badge/Phiên_bản-Free_Community-10b981?style=flat-square)](#)
[![Version](https://img.shields.io/badge/Version-2.0.0-3b82f6?style=flat-square)](#)
[![License: MIT](https://img.shields.io/badge/Giấy_phép-MIT-f59e0b?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/macOS-Apple_Silicon_%26_Intel-000000?style=flat-square&logo=apple)](#)
[![Built with Rust](https://img.shields.io/badge/Ngôn_ngữ-Rust-dea584?style=flat-square&logo=rust)](#)
[![Author](https://img.shields.io/badge/Tác_giả-boykhacthang-8b5cf6?style=flat-square)](https://github.com/boykhacthang/mac-omni-tool)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support_me_on_Ko--fi-72a4f2?style=flat-square&logo=ko-fi&logoColor=white)](https://ko-fi.com/I1R72A1475)

**Mac Omni Tool** là bộ công cụ hệ thống đa năng (**All-in-One — Phiên bản Cộng đồng Miễn phí & Mã nguồn mở 100%**) dành cho macOS (Apple Silicon M1/M2/M3/M4 & Intel), được phát triển bởi **[boykhacthang](https://github.com/boykhacthang/mac-omni-tool)**. Ứng dụng được viết bằng **Rust** kết hợp trực tiếp với các Native Frameworks của Apple (`IOKit`, `CoreGraphics`, `DisplayServices`, `CoreAudio`, `Carbon`, `ApplicationServices`, `AppKit`) cùng giao diện đa cửa sổ `tao` + `wry` siêu nhẹ, giúp thay thế hơn 7 phần mềm trả phí trên Mac chỉ trong một gói `.app` duy nhất.

---

## 📥 Tải Về Bộ Cài Đặt Sẵn (`.dmg` / `.zip` — Không Cần Build Lại)

Người dùng chỉ cần tải trực tiếp bộ cài **`.dmg`** hoặc **`.zip`** đã được đóng gói sẵn bên dưới về dùng ngay mà không cần cài Rust hay biên dịch lại mã nguồn:

| Phiên bản | Trạng thái | 💿 Bộ cài `.dmg` chuẩn macOS | 🗜️ Bản nén `.zip` gọn nhẹ | 📋 Trang Release |
| :--- | :--- | :--- | :--- | :--- |
| **v2.0.0** | 🌟 **Mới nhất (Khuyên dùng)** | [**⬇️ Tải `MacOmniTool-v2.0.0-Installer.dmg` (3.1 MB)**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v2.0.0/MacOmniTool-v2.0.0-Installer.dmg) | [**⬇️ Tải `MacOmniTool-v2.0.0-macOS.zip` (2.6 MB)**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v2.0.0/MacOmniTool-v2.0.0-macOS.zip) | [GitHub Release v2.0.0](https://github.com/boykhacthang/mac-omni-tool/releases/tag/v2.0.0) |
| **v1.0.0** | 📦 Bản nền tảng | [**⬇️ Tải `MacOmniTool-v1.0.0-Installer.dmg` (3.1 MB)**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v1.0.0/MacOmniTool-v1.0.0-Installer.dmg) | [**⬇️ Tải `MacOmniTool-v1.0.0-macOS.zip` (2.6 MB)**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v1.0.0/MacOmniTool-v1.0.0-macOS.zip) | [GitHub Release v1.0.0](https://github.com/boykhacthang/mac-omni-tool/releases/tag/v1.0.0) |

### ⚡ Hướng dẫn Cài đặt Nhanh (30 Giây)
1. Tải file **`MacOmniTool-v2.0.0-Installer.dmg`** ở bảng trên và mở lên.
2. Kéo **`MacOmniTool.app`** thả vào thư mục **`Applications`**.
3. **Mở khóa lần đầu trên máy Mac mới**: Vì đây là bản cộng đồng miễn phí không thu phí chứng chỉ Apple Developer (\$99/năm), ở lần mở đầu tiên bạn chỉ cần bấm đúp vào file **`Mo-Khoa-App-Tren-May-Moi.command`** có sẵn trong `.dmg` (hoặc chạy lệnh sau trong Terminal):
   ```bash
   xattr -cr /Applications/MacOmniTool.app && open /Applications/MacOmniTool.app
   ```

### 🔄 Tự Động Kiểm Tra & Cập Nhật 1-Click (Auto-Updater)
- **Dành cho Người dùng (Client)**:
  - `Mac Omni Tool` tự động kiểm tra bản mới từ [`releases/latest.json`](https://raw.githubusercontent.com/boykhacthang/mac-omni-tool/main/releases/latest.json) mỗi khi mở app (hoặc kiểm tra thủ công tại menu **` -> Mac Omni Tool -> Check for Updates…`** hoặc **`Cài đặt -> Kiểm tra bản cập nhật`**).
  - Khi có phiên bản mới, chỉ cần bấm nút **"⚡ Tự tải & Cập nhật ngay (1-Click)"**, ứng dụng sẽ tự động tải bản `.zip` mới, kiểm tra mã băm an toàn **SHA-256**, gỡ khóa Gatekeeper (`xattr -cr`), ghi đè `/Applications/MacOmniTool.app` và tự khởi động lại sang bản mới.
- **Dành cho Nhà phát triển (Phát hành bản mới chỉ với 1 lệnh)**:
  - Khi muốn tung ra bản cập nhật mới (ví dụ `2.1.0`), chỉ cần chạy duy nhất 1 lệnh:
    ```bash
    ./publish_release.sh 2.1.0 "Mô tả các tính năng mới của bản 2.1.0"
    ```
  - Script sẽ tự động nâng version trong `Cargo.toml`, build bản Release tối ưu, đóng gói `.dmg` & `.zip` vào `releases/v2.1.0/`, tính mã SHA-256, cập nhật `releases/latest.json`, tạo Git Tag `v2.1.0` và đẩy thẳng lên GitHub để mọi máy client nhận thông báo cập nhật ngay lập tức.

---

## 🚀 Các Tính Năng Qua 2 Phiên Bản (`v1.0` & `v2.0`)

### 🌟 Phiên bản 1.0 — Nền tảng All-in-One Cốt lõi
Phiên bản 1.0 xây dựng trọn bộ công cụ điều khiển hệ thống macOS:
1. **☀️ Điều khiển Màn hình & Bảo vệ Mắt (`core-display`)**:
   - Điều chỉnh độ sáng phần cứng (`DisplayServices` / `IOKit`) cho cả màn hình MacBook và màn hình rời.
   - Giảm sáng sâu bằng phần mềm (Gamma LUT 20%–100%) và lọc ánh sáng xanh (Warmth) bảo vệ mắt ban đêm.
   - Chuyển đổi nhanh giao diện **macOS Sáng / Tối** và **Giao diện App Sáng / Tối**.
2. **🔊 Chuyển nhanh Thiết bị Âm thanh & AirPods (`CoreAudio`)**:
   - Thanh trượt âm lượng, nút mức âm lượng nhanh (`25% / 50% / 75% / 100%`), tắt tiếng và chuyển đổi tức thì giữa **AirPods, Loa ngoài MacBook, Tai nghe Bluetooth/Có dây, USB DAC, Loa màn hình HDMI/DisplayPort**, kèm chọn Micro đầu vào.
3. **🌡️ Giám sát Phần cứng & Nhiệt độ (`core-monitor`)**:
   - Đọc trực tiếp cảm biến nhiệt độ (`IOHIDEventSystemClient`: CPU SoC, GPU, Pin/PMU), mức dùng CPU/RAM/SSD, băng thông mạng Live I/O và hiển thị thông số trực tiếp trên thanh Menu Bar (`❖ 45°C • 8.2%`) kèm Popup điều khiển nhanh.
4. **📋 Quản lý Clipboard 1,000 Mục (`core-clipboard`)**:
   - Lưu trữ tối đa **1,000 mục** (Văn bản đầy đủ & Hình ảnh PNG) trong SQLite WAL (`~/Library/Application Support/MacOmniTool/`), ghim mục quan trọng, tìm kiếm, phân trang (`50 / 100 / 200 / 500 / 1000` mục/trang) và ngăn kéo trượt cạnh trái (`Cmd+Shift+V`).
5. **📐 Chia Cửa sổ & Chuyển đổi Cửa sổ Alt-Tab (`core-window`)**:
   - **13 bố cục chia cửa sổ tùy chỉnh phím tắt** (Chia 1/2, Góc 1/4, Tỷ lệ 2/3 & 1/3, Căn giữa 75%, Toàn màn hình, Chuyển sang màn hình kế tiếp).
   - **Alt-Tab kiểu Windows (`Cmd+Tab`)** với ảnh chụp Live Preview thời gian thực của từng cửa sổ.
6. **🧹 Dọn rác Hệ thống, Thùng rác & Gỡ Ứng dụng (`core-cleaner`)**:
   - Quét và dọn bộ nhớ đệm (`~/Library/Caches`, Logs, Xcode DerivedData, Cargo/npm caches), xem chi tiết và làm trống Thùng rác (`~/.Trash`), gỡ sạch ứng dụng (`.app` kèm tệp phụ trong `~/Library`) và tìm tệp dung lượng lớn.
7. **🖱️ Tự động hóa Chuột & Cắt/Dán Finder (`core-input`)**:
   - Nhấn đồng thời **Chuột Trái + Chuột Phải** (hoặc phím `F3`) để mở Mission Control, App Exposé, Hiện Desktop hoặc Launchpad.
   - Bật **Cắt & Dán tệp (`Cmd+X` -> `Cmd+V`)** trực tiếp trong macOS Finder.

---

### 🔥 Phiên bản 2.0 — Kiến trúc Modular, Chuột Rời & Cuộn Mượt 120Hz, Chụp màn hình Đa màn hình, Đa ngôn ngữ & Chống Rò rỉ RAM
Phiên bản 2.0 tái cấu trúc toàn diện theo chuẩn Rust chuyên nghiệp và bổ sung loạt tính năng cao cấp:
1. **🌐 Đa ngôn ngữ với Hộp chọn `<select>` (`i18n` — Mặc định Tiếng Anh & Tiếng Việt)**:
   - Chọn ngôn ngữ qua hộp thả xuống (`<select>`) đồng bộ tại Sidebar, màn hình Màn hình, màn hình Cài đặt và Menu Bar Popover.
   - Mặc định hiển thị **Tiếng Anh (`en`)**, hỗ trợ chuyển sang **Tiếng Việt (`vi`)** tức thì trên toàn bộ 5 loại cửa sổ (`Main Dashboard`, `Menu Bar Popover`, `Clipboard Drawer`, `Alt-Tab`, `Screenshot Overlays`).
   - Tách riêng từ điển dịch thuật tại `ui/js/locales/en.js` và `ui/js/locales/vi.js`, cho phép cộng đồng thêm ngôn ngữ mới cực kỳ dễ dàng.
2. **📸 Chụp màn hình & Vẽ Chú thích 3 Chế độ trên Đa màn hình**:
   - **Chế độ 1 (`Cmd+Shift+0`) — Chụp 1 vùng trên cả 2 màn hình**: Hiển thị lớp phủ chọn vùng & vẽ chú thích đồng thời trên tất cả màn hình đang kết nối.
   - **Chế độ 2 (`Cmd+Shift+9`) — Toàn màn hình theo con trỏ chuột + Vẽ chú thích**: Tự động nhận diện màn hình nơi con trỏ chuột đang đứng (hoặc click sang màn hình kia để đổi) kèm thanh công cụ vẽ đầy đủ (**Bút đỏ, Đường thẳng, Mũi tên, Khung vuông, Tròn, Viết chữ, Làm mờ Pixelate, Di chuyển, Chọn màu, Độ dày nét, Hoàn tác `Cmd+Z`, Lưu `.png` `Cmd+S`, Copy `Cmd+C`**).
   - **Chế độ 3 (`Cmd+Shift+8`) — Toàn màn hình theo con trỏ chuột + Copy tức thì**: Chụp ngay màn hình dưới con trỏ chuột và đưa thẳng vào Clipboard.
3. **🖱️ Tùy chỉnh Chuột Rời, Cuộn Chuột Ngược & Cuộn Mượt 120Hz (`core-input`)**:
   - **Cuộn chuột mượt mà 120Hz (ProMotion Smooth Scroll)**: Nội suy từng khấc con lăn vật lý thành chuyển động lướt êm ái, tự động bỏ qua cử chỉ Trackpad MacBook.
   - **Bật/Tắt Cuộn Chuột Ngược độc lập (Reverse Mouse Scroll)**: Đảo chiều con lăn chuột rời chuẩn Windows mà vẫn giữ nguyên vuốt tự nhiên trên Trackpad.
   - **Gán nút bấm chuột theo từng App (Per-App Profiles cho Mouse 3 / 4 / 5)**: Tự động dùng mặc định Tiến/Lùi trang web khi mở trình duyệt (`Chrome`, `Safari`, `Arc`, `Brave`, `Edge`, `Firefox`), và tự động chuyển qua lại giữa các Desktop (`Spaces`) đang mở khi dùng ở các ứng dụng khác.
3. **🧠 Giám sát Phần cứng Apple Silicon GPU & Bảng Tiến trình Zero-RAM-Leak**:
   - **Thông số GPU Chuyên sâu (`IOAccelerator`)**: Đọc trực tiếp % tải GPU (Device / Renderer / Tiler), số nhân GPU, chuẩn Metal, và bộ nhớ Unified VRAM (Đang dùng / Đã cấp / Trần tối đa 75% RAM), cùng biểu đồ xung nhịp từng nhân CPU (Per-Core).
   - **Cơ chế Chống tràn RAM tuyệt đối**: Tái sử dụng dòng DOM tại chỗ (`syncProcessRowsDom` + `removeChild`), ủy quyền sự kiện (Event Delegation) và dọn dẹp tiến trình đã tắt (`retain`), giúp chạy liên tục 24/7 mà không tăng RAM.
   - **Bảng Tiến trình Tự động Co giãn (Responsive)**: Tự động ẩn/hiện cột thông minh (`< 980px`, `< 780px`, `< 580px`) để không bao giờ vỡ bảng khi thu nhỏ hoặc chia đôi cửa sổ.
4. **👯 Quét Tệp Trùng lặp Đa luồng 3 Bước (`rayon`) & Bảo vệ Database**:
   - Thuật toán 3 bước siêu tốc (`Nhóm kích thước` -> `Băm nhanh 8KB đầu/cuối` -> `Băm 100% nội dung 128-bit`).
   - **Bảo vệ Database Nội bộ**: Tự động khóa và bỏ qua các tệp lưu trữ bên trong engine SQL/NoSQL (`.ibd`, `.frm`, `.myd`, `PG_VERSION`, `.db-wal`, `WiredTiger`) trong khi vẫn quét bình thường các file `.sql` dump bên ngoài; hỗ trợ thêm thư mục loại trừ và nút **"⚡ Chọn tự động bản sao (Giữ 1 bản gốc)"**.
5. **⚡ Phím tắt Carbon Native & Trải nghiệm macOS Mượt mà**:
   - Kết hợp **Carbon `RegisterEventHotKey`** và `CGEventTap` giúp các phím tắt chia cửa sổ (`Ctrl+Option+C`, mũi tên...) luôn phản hồi tức thì, kèm cơ chế tự căn chỉnh cửa sổ qua Cocoa `NSWindow`.
   - Tự động ẩn biểu tượng trên thanh Dock khi thu gọn lên Menu Bar (`🔽 Ẩn App`), và xử lý bo góc trong suốt hoàn hảo cho Menu Bar Popover.
6. **🏗️ Tái cấu trúc Modular & Bộ Kiểm thử Tự động**:
   - Tách thành **6 thư viện Rust độc lập** (`crates/*`), **8 module điều phối** (`src/*`), **19 file giao diện modular** (`ui/*`) và **27 bài kiểm thử Unit & Integration Tests**.

---

## 🏗️ Cấu trúc Thư mục Dự án

```text
mac-omni-tool/
├── Cargo.toml                      # Cấu hình Cargo Workspace (v2.0.0, MIT License)
├── LICENSE                         # Giấy phép Mã nguồn mở MIT (Copyright (c) 2026 khacthang)
├── .github/FUNDING.yml             # Cấu hình nút Sponsor / Ko-fi trên GitHub
├── build_installer_dmg.sh          # Script đóng gói MacOmniTool.app, .dmg & .zip
├── README.md                       # Tài liệu dự án (Tiếng Anh - Mặc định)
├── README.vi.md                    # Tài liệu dự án (Tiếng Việt)
├── docs/                           # Bộ tài liệu kỹ thuật chuyên sâu
│   ├── ARCHITECTURE.md             # Kiến trúc hệ thống & Native FFI
│   ├── UI_STRUCTURE.md             # Kiến trúc Frontend Modular, i18n & Component
│   ├── API_REFERENCE.md            # Tài liệu JSON-RPC (omni://localhost/api)
│   └── DEVELOPMENT_AND_TESTING.md  # Hướng dẫn phát triển, kiểm thử & đóng gói
├── src/                            # Binary Crate chính (Application Shell & Event Loop)
├── ui/                             # Giao diện tách theo Màn hình, Locale & Component
│   ├── index.html                  # Khung HTML chính
│   ├── styles/                     # base.css, screens.css, overlays.css
│   └── js/
│       ├── locales/                # en.js (Mặc định), vi.js (Tiếng Việt)
│       ├── core/                   # api.js, i18n.js (Engine đa ngôn ngữ & <select>)
│       ├── components/             # clipboard-card.js, path-cell.js
│       ├── screens/                # 6 màn hình chức năng độc lập
│       └── overlays/               # 4 cửa sổ Overlay độc lập
├── crates/                         # 6 Thư viện Lõi (core-monitor, core-display, core-clipboard, core-window, core-input, core-cleaner)
└── tests/
    └── integration_test.rs         # Bộ kiểm thử tích hợp toàn hệ thống
```

---

## 🛠️ Chạy thử, Kiểm thử & Đóng gói Nhanh

```bash
# 1. Chạy toàn bộ 27 bài kiểm thử Unit & Integration Tests
cargo test --workspace

# 2. Chạy ứng dụng ở chế độ phát triển
cargo run

# 3. Đóng gói bản cài đặt MacOmniTool.app và file MacOmniTool-Installer.dmg
./build_installer_dmg.sh
```

---

## 👨‍💻 Về Tác Giả & Phiên Bản Cộng Đồng Miễn Phí

- **Tác giả**: **boykhacthang** ([@boykhacthang](https://github.com/boykhacthang/mac-omni-tool))
- **Mã nguồn GitHub**: [https://github.com/boykhacthang/mac-omni-tool](https://github.com/boykhacthang/mac-omni-tool) (`git@github.com:boykhacthang/mac-omni-tool.git`)
- **Phiên bản**: **Free Community Edition (Bản Cộng đồng Miễn phí)** — Mã nguồn mở 100%, không quảng cáo, không thu thập dữ liệu, hoàn toàn miễn phí cho cộng đồng người dùng Mac.
- **Đóng góp**: Mọi đóng góp tính năng mới, báo lỗi hoặc bổ sung file dịch ngôn ngữ mới (`ui/js/locales/<lang>.js`) đều được chào đón!

---

## ☕ Ủng Hộ Tác Giả (Support me on Ko-fi)

Nếu **Mac Omni Tool** giúp công việc hàng ngày trên máy Mac của bạn tiện lợi hơn, bạn có thể ủng hộ tác giả **boykhacthang** qua **Ko-fi** để tiếp thêm động lực phát triển thêm nhiều tính năng mới:

[![Support me on Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I1R72A1475)

- ☕ **Ko-fi**: [https://ko-fi.com/I1R72A1475](https://ko-fi.com/I1R72A1475)
- ⭐ **Tặng 1 Star trên GitHub** để lan tỏa dự án tới cộng đồng macOS!

```html
<script type='text/javascript' src='https://storage.ko-fi.com/cdn/widget/Widget_2.js'></script><script type='text/javascript'>kofiwidget2.init('Support me on Ko-fi', '#72a4f2', 'I1R72A1475');kofiwidget2.draw();</script>
```

---

## 📄 Giấy Phép (License)

Dự án được phát hành dưới giấy phép nguồn mở **MIT License** — Free Community Edition. Xem chi tiết tại file [`LICENSE`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/LICENSE).
Copyright © 2026 **boykhacthang**.
