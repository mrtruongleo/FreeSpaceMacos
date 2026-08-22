# 🧹 FreeSpace for macOS

A modern, high-performance storage cleanup and disk analyzer app for macOS built with **Tauri 2 (Rust + TypeScript/HTML5/CSS)**.

![FreeSpace App](app-icon-1024.png)

---

## ✨ Features

- **🛡️ Safe to Clean (Auto-selected)**:
  - User Caches (`~/Library/Caches`)
  - Application Logs (`~/Library/Logs`)
  - Crash Reports (`~/Library/Application Support/CrashReporter`)
  - Trash Bin (`~/.Trash`)
  - Safari Browser Cache
- **⚠️ Needs Review (Explicit User Confirmation)**:
  - Xcode DerivedData & Archives
  - iOS Simulator Devices
  - Developer Caches (`npm`, `yarn`, `CocoaPods`, `Gradle`, `pip`, `Homebrew`)
  - Docker Containers & Volumes Data
  - VS Code Editor Cache
  - Downloads Folder
- **🚀 Performance & Safety**:
  - Direct, blazing-fast file system removal without repetitive macOS authorization prompts.
  - Interactive disk usage gauge & real-time space calculation.
  - Confirmation dialog with detailed breakdown when selecting items that require review.
  - Toast notification system for instant feedback.
- **🎨 Modern macOS UI**:
  - Dark mode glassmorphism design with vibrancy styling.
  - Custom macOS squircle app icon.

---

## 🛠️ Tech Stack

- **Framework**: [Tauri 2](https://v2.tauri.app/)
- **Backend**: Rust (`walkdir`, `dirs`, `libc`, `serde`)
- **Frontend**: TypeScript, Vanilla CSS (Glassmorphism & animations), HTML5
- **Bundler**: Vite

---

## 🚀 Getting Started

### Prerequisites
- macOS (Apple Silicon or Intel)
- Node.js (v18+) & npm
- Rust & Cargo (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)

### Development
```bash
# Install dependencies
npm install

# Run dev server with hot reload
npm run tauri dev
```

### Build Production Bundle
```bash
# Build .app and .dmg bundles
npm run tauri build
```

The compiled application will be located at:
- `src-tauri/target/release/bundle/macos/FreeSpace.app`
- `src-tauri/target/release/bundle/dmg/FreeSpace_0.1.0_aarch64.dmg`

---

## 📄 License
MIT
