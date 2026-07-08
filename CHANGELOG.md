# Changelog

All notable changes to this project are documented here.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.0] - 2026-07-08

Initial release. Built with Tauri (Rust + WebView2).

### Added

- Keyboard layout widget for the active window (EN / UK / RU…).
- Click the language label to switch the active window's layout.
- Layout lock with snap-back if `Alt+Shift` changes it anyway.
- Position pin for the widget itself.
- Settings panel: size, opacity, corner radius, background color, always-on-top.
- Autostart via the tray-icon menu.
- Settings persisted across restarts.
