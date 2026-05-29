# TypeLink Project Guide

Use your phone as a voice input source for your desktop computer. The desktop app is a Tauri application that hosts a local HTTP + WebSocket server.

## Commands

- Run development app: `cd desktop && npm start` (runs `tauri dev` without external dev server dependencies)
- Build production release: `cd desktop && npm run build` (runs `tauri build`)
- Check Rust backend compilation: `cd desktop/src-tauri && cargo check`
- Build Rust backend: `cd desktop/src-tauri && cargo build`

## Codebase Architecture

- **Frontend**: Located in [desktop/public](file:///Users/zhangjian/typethin/desktop/public).
  - [index.html](file:///Users/zhangjian/typethin/desktop/public/index.html): Mobile interface for voice input.
  - [qr.html](file:///Users/zhangjian/typethin/desktop/public/qr.html): Desktop app view showing QR code to scan.
- **Backend (Tauri)**: Located in [desktop/src-tauri](file:///Users/zhangjian/typethin/desktop/src-tauri).
  - [lib.rs](file:///Users/zhangjian/typethin/desktop/src-tauri/src/lib.rs): Core logic. Starts an Axum HTTP + WebSocket server on port 9527 to serve the public pages and receive input.
  - Keyboard simulation is done via the `enigo` crate (clipboard paste).

## Key Development Rules & Technical Decisions

- **Main Thread Requirement**: macOS UI/input source APIs (used by `enigo`) require execution on the main thread. Always dispatch clipboard pasting and keyboard simulation to the main thread via `app_handle.run_on_main_thread(move || { ... })`.
- **System Tray Icon**:
  - The tray icon is dynamically drawn with pixel grids as "TL".
  - Created via `create_tray_icon(r, g, b) -> tauri::image::Image<'static>`.
  - Colors indicate status: Gray `#808080` (disconnected) and Green `#16a34a` (connected).
- **Tauri Config**:
  - `devUrl` is removed from `tauri.conf.json` so that `tauri dev` compiles and starts the backend immediately without waiting for an external dev server.
- **Windows Adapter Names**:
  - `if_addrs` returns GUIDs (e.g., `{CD9E4015-...}`) on Windows. We must query the Windows registry key `SYSTEM\CurrentControlSet\Control\Network\{4D36E972-E325-11CE-BFC1-08002BE10318}\<GUID>\Connection` -> `Name` to translate GUIDs to user-friendly names before matching/scoring.
  - To avoid compilation warnings (`unused_mut`) on non-Windows platforms, use variable shadowing rather than mutable bindings for this translation.
