# 🛠️ Mac Omni Tool — All-in-One Native macOS Utility (Free Community Edition)

> 🌐 **Language / Ngôn ngữ**: **[🇬🇧 English](README.md)** | [🇻🇳 Tiếng Việt](README.vi.md)

[![Edition](https://img.shields.io/badge/Edition-Free_Community-10b981?style=flat-square)](#)
[![Version](https://img.shields.io/badge/Version-2.3.0-3b82f6?style=flat-square)](#)
[![License: MIT](https://img.shields.io/badge/License-MIT-f59e0b?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/macOS-Apple_Silicon_%26_Intel-000000?style=flat-square&logo=apple)](#)
[![Built with Rust](https://img.shields.io/badge/Built_with-Rust-dea584?style=flat-square&logo=rust)](#)
[![Author](https://img.shields.io/badge/Author-boykhacthang-8b5cf6?style=flat-square)](https://github.com/boykhacthang/mac-omni-tool)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-Support_me_on_Ko--fi-72a4f2?style=flat-square&logo=ko-fi&logoColor=white)](https://ko-fi.com/I1R72A1475)

**Mac Omni Tool** is a **100% Free & Open-Source Community Edition** All-in-One system utility for macOS (Apple Silicon M1/M2/M3/M4 & Intel), authored by **[boykhacthang](https://github.com/boykhacthang/mac-omni-tool)**. Built natively in **Rust** with direct FFI bindings to Apple frameworks (`IOKit`, `CoreGraphics`, `DisplayServices`, `CoreAudio`, `Carbon`, `ApplicationServices`, `AppKit`) and a lightweight `tao` + `wry` multi-window architecture, it replaces 7+ paid macOS utilities in a single self-contained `.app` bundle.

---

## 📥 Direct Download — Pre-built macOS Installers (No Build Required)

You can download the pre-built **`.dmg` Installer** or **`.zip` App Bundle** directly without installing Rust or building from source:

| Version | Status | 💿 macOS `.dmg` Installer | 🗜️ macOS `.zip` Portable | 📋 Release Page |
| :--- | :--- | :--- | :--- | :--- |
| **v2.3.0** | 🌟 **Latest Stable** | [**⬇️ Download `MacOmniTool-v2.3.0-Installer.dmg`**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v2.3.0/MacOmniTool-v2.3.0-Installer.dmg) | [**⬇️ Download `MacOmniTool-v2.3.0-macOS.zip`**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v2.3.0/MacOmniTool-v2.3.0-macOS.zip) | [GitHub Release v2.3.0](https://github.com/boykhacthang/mac-omni-tool/releases/tag/v2.3.0) |
| **v2.2.1** | 📦 **Previous Stable** | [**⬇️ Download `MacOmniTool-v2.2.1-Installer.dmg` (3.2 MB)**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v2.2.1/MacOmniTool-v2.2.1-Installer.dmg) | [**⬇️ Download `MacOmniTool-v2.2.1-macOS.zip` (2.7 MB)**](https://github.com/boykhacthang/mac-omni-tool/raw/main/releases/v2.2.1/MacOmniTool-v2.2.1-macOS.zip) | [GitHub Release v2.2.1](https://github.com/boykhacthang/mac-omni-tool/releases/tag/v2.2.1) |

### ⚡ Quick Installation Steps (30 Seconds)
1. Download **`MacOmniTool-v2.3.0-Installer.dmg`** above and double-click to open it.
2. Drag **`MacOmniTool.app`** into the **`Applications`** folder.
3. **First Launch on a New Mac (Gatekeeper Bypass)**: Because this is a free community open-source app without a paid \$99/year Apple Developer notarization certificate, double-click **`Mo-Khoa-App-Tren-May-Moi.command`** inside the `.dmg` (or run the command below in Terminal once):
   ```bash
   xattr -cr /Applications/MacOmniTool.app && open /Applications/MacOmniTool.app
   ```

### 🔔 User-Controlled Update Notifications & Confirmation
- **For Users (Notify Only — Never Auto-Updates Without Confirmation)**:
  - `Mac Omni Tool` checks [`releases/latest.json`](https://raw.githubusercontent.com/boykhacthang/mac-omni-tool/main/releases/latest.json) on startup (or manually via **` -> Mac Omni Tool -> Check for Updates…`** or **`Settings -> Check for Updates`**) and **only displays a non-intrusive notification banner** when a new version is available.
  - The app **never updates itself automatically in the background**. You can review the release notes and choose to **Update Now** (with an explicit confirmation step that verifies the **SHA-256** checksum, replaces `/Applications/MacOmniTool.app`, and restarts), **Download `.dmg` Manually**, **Remind Me Later**, or **Skip Version**.
- **For Maintainers (1-Command Release Publishing)**:
  - Whenever you release a new version (e.g., `2.1.0`), simply run:
    ```bash
    ./publish_release.sh 2.1.0 "Added new features and bug fixes"
    ```
  - This single command automatically updates `Cargo.toml`, compiles the release `.app`, packages the `.dmg` & `.zip` into `releases/v2.2.0/`, computes SHA-256 hashes, updates `releases/latest.json`, commits, tags `v2.2.0`, and pushes everything to GitHub so all client machines receive the update notification immediately.

---

## 🚀 Feature Evolution Across 2 Versions (`v1.0` & `v2.0`)

### 🌟 Version 1.0 — Core All-in-One Foundation
Version 1.0 established the core native macOS utility suite:
1. **☀️ Display & Color Control (`core-display`)**:
   - Hardware brightness control (`DisplayServices` / `IOKit`) for internal and external monitors.
   - Software Gamma LUT extra dimming (20%–100%) and blue-light warmth filter for night eye protection.
   - Instant one-click switching for **macOS System Light/Dark Mode** and **App Light/Dark Theme**.
2. **🔊 Fast Audio & AirPods Switcher (`CoreAudio`)**:
   - Master volume control, quick volume presets (`25% / 50% / 75% / 100%`), mute toggle, and instant switching between **AirPods, MacBook Speakers, Bluetooth/Wired Headphones, USB DAC, and Display Audio**, plus Input Microphone selection.
3. **🌡️ Hardware & Sensor Monitor (`core-monitor`)**:
   - Direct Apple Silicon & Intel thermal sensors (`IOHIDEventSystemClient`: CPU SoC, GPU, Battery/PMU), CPU/RAM/SSD usage, live network RX/TX bandwidth, and native macOS Menu Bar live status item (`❖ 45°C • 8.2%`) with a quick dropdown Popover.
4. **📋 1,000-Item Clipboard Manager (`core-clipboard`)**:
   - Persistent SQLite WAL history (`~/Library/Application Support/MacOmniTool/`) storing up to **1,000 items** (full text + PNG images), item pinning, search & category filters, configurable pagination (`50 / 100 / 200 / 500 / 1000` per page), and a quick left-edge drawer (`Cmd+Shift+V`).
5. **📐 Window Tiling & Windows-Style Alt-Tab (`core-window`)**:
   - **13 customizable window snapping layouts** (Halves, Quarters, 2/3 & 1/3 Splits, Center 75%, Maximize, Move to Next Display) with conflict detection.
   - **Windows-style Alt-Tab (`Cmd+Tab`)** overlay showing live window thumbnails (`CGWindowListCreateImage`).
6. **🧹 System Cleaner, Trash & App Uninstaller (`core-cleaner`)**:
   - Scans & cleans user/system caches (`~/Library/Caches`, Logs, Xcode DerivedData, Cargo/npm caches), manages & empties macOS Trash (`~/.Trash`), uninstalls `.app` bundles along with `~/Library` leftovers, and scans large files.
7. **🖱️ Mouse Chord & Finder Cut/Paste (`core-input`)**:
   - Simultaneous **Left + Right Mouse Click** (or `F3` key) to trigger Mission Control, App Exposé, Show Desktop, or Launchpad.
   - True **Finder Cut & Paste (`Cmd+X` -> `Cmd+V`)** to move files directly in macOS Finder.

---

### 🔥 Version 2.0 — Modular Architecture, External Mouse & 120Hz Smooth Scroll, Multi-Monitor Screenshot, i18n & Zero-Leak Engine
Version 2.0 is a complete architectural and feature upgrade engineered for production stability and community extensibility:
1. **🌐 Multi-Language Support (`i18n` Dropdown — English Default & Vietnamese)**:
   - Instant `<select>` language dropdown in the Sidebar, Displays screen, Settings screen, and Menu Bar Popover.
   - Defaults to **English (`en`)** with full **Vietnamese (`vi`)** support, synchronized in real time across all 5 window types (`Main`, `Menu Bar Popover`, `Clipboard Drawer`, `Alt-Tab`, `Screenshot Overlays`).
   - Decoupled locale dictionaries (`ui/js/locales/en.js`, `ui/js/locales/vi.js`) so contributors can add new languages by dropping in a single file.
2. **📸 3-Mode Multi-Monitor Screenshot & Annotation**:
   - **Mode 1 (`Cmd+Shift+0`) — Dual-Monitor Region Capture**: Displays interactive selection and annotation overlays across all connected monitors simultaneously.
   - **Mode 2 (`Cmd+Shift+9`) — Cursor-Aware Fullscreen + Annotate**: Automatically detects the monitor under the mouse cursor (or click another monitor to switch) and opens the full annotation suite (**Freehand Pen, Line, Arrow, Rectangle, Circle, Text, Pixelate Blur, Move, Colors, Stroke Width, Undo `Cmd+Z`, Save `Cmd+S`, Copy `Cmd+C`**).
   - **Mode 3 (`Cmd+Shift+8`) — Cursor-Aware Fullscreen + Instant Copy**: Captures the monitor under the mouse cursor and copies it directly to the Clipboard.
3. **🖱️ External Mouse Customization, Reverse Scroll & 120Hz Smooth Scroll (`core-input`)**:
   - **120Hz ProMotion Smooth Scroll**: Interpolates discrete external mouse wheel notches into fluid pixel-level gliding without affecting MacBook Trackpad gestures.
   - **Independent Reverse Mouse Scroll**: Reverse external mouse wheel scroll direction (standard Windows direction) while keeping Natural Scrolling intact on your MacBook Trackpad.
   - **Per-App Mouse Button Profiles (Mouse 3 / 4 / 5)**: Keep default Forward/Back web page navigation in browsers (`Chrome`, `Safari`, `Arc`, `Brave`, `Edge`, `Firefox`) while automatically switching between open macOS Desktops (`Spaces`) in other applications.
3. **🧠 Zero-RAM-Leak Real-Time Hardware & Responsive Process Table**:
   - **Apple Silicon GPU Telemetry (`IOAccelerator`)**: Reads live GPU Device/Renderer/Tiler utilization %, GPU core count, Metal family, and Unified Memory (VRAM In-Use / Allocated / 75% Max ceiling), plus per-core CPU mini load bars.
   - **Zero-Memory-Leak Polling**: Uses in-place DOM row recycling (`syncProcessRowsDom` + `removeChild`), event delegation, and `retain()` PID pruning so running the monitor 24/7 never increases WebKit or Rust RAM usage.
   - **Responsive Process Table**: Automatically adapts column visibility (`< 980px`, `< 780px`, `< 580px`) so the table never breaks when tiled to half/quarter screen.
4. **👯 3-Stage Parallel Duplicate File Scanner (`rayon`) with Database Protection**:
   - Ultra-fast 3-stage pipeline (`Byte Size Grouping` -> `8KB Edge Hash` -> `Full 128-bit Content Hash`).
   - **Smart Database Protection**: Automatically locks and skips internal database engine storage files (`.ibd`, `.frm`, `.myd`, `PG_VERSION`, `.db-wal`, `WiredTiger`) while still scanning standard `.sql` dump/script files, plus customizable folder exclusions and 1-click **"⚡ Auto-Select Copies (Keep 1 Original)"**.
5. **⚡ Native Carbon Hotkeys & Seamless macOS Window Polish**:
   - Hybrid **Carbon `RegisterEventHotKey`** + `CGEventTap` engine ensures window tiling shortcuts (`Ctrl+Option+C`, arrows, etc.) work reliably even when secure inputs or other apps are active, plus direct Cocoa `NSWindow:setFrame:display:animate:` self-snapping.
   - Automatically hides the macOS Dock icon (`NSApplicationActivationPolicyAccessory`) when minimized to the Menu Bar, and renders crisp rounded transparent corners on the Menu Bar Popover.
6. **🏗️ Professional Modular Codebase & Automated Test Suite**:
   - Refactored into **6 domain Rust crates** (`core-monitor`, `core-display`, `core-clipboard`, `core-window`, `core-input`, `core-cleaner`), **8 root binary modules** (`src/`), **19 modular UI files** (`ui/`), and **27 automated Unit & Integration tests**.

---

## 🏗️ Modular Project Structure

```text
mac-omni-tool/
├── Cargo.toml                      # Rust Workspace configuration (v2.2.0, MIT License)
├── LICENSE                         # MIT Open-Source License (Copyright (c) 2026 khacthang)
├── .github/FUNDING.yml             # GitHub Sponsors & Ko-fi configuration
├── build_installer_dmg.sh          # Release bundler for MacOmniTool.app, .dmg & .zip
├── README.md                       # Project documentation (English - Default)
├── README.vi.md                    # Project documentation (Vietnamese)
├── docs/                           # Technical documentation suite
│   ├── ARCHITECTURE.md             # System architecture & Native macOS FFI
│   ├── UI_STRUCTURE.md             # Modular Frontend, i18n & DOM recycling guide
│   ├── API_REFERENCE.md            # JSON-RPC specification (omni://localhost/api)
│   └── DEVELOPMENT_AND_TESTING.md  # Build, testing & packaging guide
├── src/                            # Root binary crate (Application Shell & Event Loop)
│   ├── main.rs                     # Bootstrap & tao EventLoop coordinator
│   ├── events.rs                   # AppUserEvent definitions & screenshot session state
│   ├── macos_window.rs             # NSWindow styling, Dock icon policy & Cocoa snapping
│   ├── menubar.rs                  # Native NSStatusBar item (❖) on the macOS Menu Bar
│   ├── single_instance.rs          # flock single-instance lock & wakeup IPC
│   ├── ui_assets.rs                # Compile-time embedded HTML/CSS/JS/Locale assets
│   ├── rpc.rs                      # JSON-RPC dispatcher (POST omni://localhost/api)
│   └── webview.rs                  # wry WebView builder & custom protocol handler
├── ui/                             # Modular Frontend (Screens, Overlays, Locales & Components)
│   ├── index.html                  # Clean HTML entry loading CSS, Locales & JS modules
│   ├── styles/
│   │   ├── base.css                # Light/Dark theme tokens, Layout, Cards, Buttons, Switches
│   │   ├── screens.css             # Screen-specific styles & responsive process table
│   │   └── overlays.css            # Styles for Menu Bar Popover, Clipboard Drawer, Alt-Tab, Screenshot Overlay
│   └── js/
│       ├── app.js                  # Main Dashboard shell & multi-window mode router
│       ├── locales/
│       │   ├── en.js               # English locale dictionary (Default)
│       │   └── vi.js               # Vietnamese locale dictionary
│       ├── core/
│       │   ├── api.js              # RPC client api(), theme sync, toast & formatters
│       │   └── i18n.js             # Multi-language engine & <select> language switcher
│       ├── components/
│       │   ├── clipboard-card.js   # Reusable Clipboard card list (Tab & Drawer)
│       │   └── path-cell.js        # Reusable compact path pill with tooltip, Copy & Finder reveal
│       ├── screens/
│       │   ├── display-audio.js    # Screen 1: Displays & Audio
│       │   ├── hardware-monitor.js # Screen 2: Hardware & Processes (Zero-RAM-Leak DOM recycling)
│       │   ├── clipboard-tab.js    # Screen 3: Clipboard Manager
│       │   ├── window-tiling.js    # Screen 4: Window Tiling & Shortcuts
│       │   ├── mouse.js            # Screen 5: External Mouse, Reverse Scroll, Smooth Scroll & Per-App Buttons
│       │   ├── system-cleaner.js   # Screen 6: System Cleaner, Duplicates, Uninstaller & Large Files
│       │   └── settings.js         # Screen 7: Unified Settings, TCC Permissions & About Author
│       └── overlays/
│           ├── menubar-popover.js  # Window 0: Lightweight Menu Bar Popover
│           ├── clipboard-drawer.js # Window 2: Left-side Clipboard Drawer (Cmd+Shift+V)
│           ├── alttab-overlay.js   # Window 3: Alt-Tab Live Preview Overlay (Cmd+Tab)
│           └── screenshot-overlay.js # Window 4: Multi-Monitor Screenshot & Annotation Overlay
├── crates/                         # 6 Domain Workspace Library Crates
│   ├── core-monitor/               # IOHID thermal sensors, IOAccelerator GPU, CPU/RAM/Processes
│   ├── core-display/               # DisplayServices brightness, Gamma LUT, CoreAudio, Screenshots
│   ├── core-clipboard/             # 1,000-item SQLite WAL store, PNG storage, NSPasteboard watcher
│   ├── core-window/                # AXUIElement Window Snapping, CGWindowList Alt-Tab, TCC Perms
│   ├── core-input/                 # Carbon RegisterEventHotKey, CGEventTap, Finder Cut/Paste
│   └── core-cleaner/               # Junk scanner, Trash, 3-stage rayon Duplicate scanner, Uninstaller
└── tests/
    └── integration_test.rs         # Workspace-wide integration test suite
```

---

## 🛠️ Quick Start, Testing & Packaging

```bash
# 1. Run all 27 unit & integration tests across the workspace
cargo test --workspace

# 2. Run in development mode
cargo run

# 3. Build optimized release MacOmniTool.app and MacOmniTool-Installer.dmg
./build_installer_dmg.sh
```

---

## 📚 Technical Documentation

- [System Architecture & Native macOS FFI (`docs/ARCHITECTURE.md`)](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/docs/ARCHITECTURE.md)
- [Modular UI Structure, i18n & DOM Memory Management (`docs/UI_STRUCTURE.md`)](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/docs/UI_STRUCTURE.md)
- [JSON-RPC API Reference (`docs/API_REFERENCE.md`)](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/docs/API_REFERENCE.md)
- [Development, Testing & Packaging Guide (`docs/DEVELOPMENT_AND_TESTING.md`)](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/docs/DEVELOPMENT_AND_TESTING.md)

---

## 👨‍💻 About the Author & Free Community Edition

- **Author**: **boykhacthang** ([@boykhacthang](https://github.com/boykhacthang/mac-omni-tool))
- **Repository**: [https://github.com/boykhacthang/mac-omni-tool](https://github.com/boykhacthang/mac-omni-tool) (`git@github.com:boykhacthang/mac-omni-tool.git`)
- **Edition**: **Free Community Edition** — 100% free, open-source, no ads, no telemetry, and no subscriptions.
- **Contributions**: Pull requests, bug reports, and new locale files (`ui/js/locales/<lang>.js`) are warmly welcome!

---

## ☕ Support me on Ko-fi

If **Mac Omni Tool** saves you time and replaces multiple paid utilities on your Mac, consider supporting the author on **Ko-fi** so new features and macOS updates keep coming:

[![Support me on Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I1R72A1475)

- ☕ **Ko-fi**: [https://ko-fi.com/I1R72A1475](https://ko-fi.com/I1R72A1475)
- ⭐ **Star this repository on GitHub** to help more macOS users discover the project!


---

## 📄 License

Distributed under the **MIT License** — Free Community Edition. See [`LICENSE`](file:///Users/khacthang/Documents/project/Rust-learn/mac-omni-tool/LICENSE) for full details.
Copyright © 2026 **boykhacthang**.
