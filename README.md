# KeyboardLangLock

A lightweight Windows 11 desktop widget that shows the active window's keyboard layout and lets you **lock it** — built to stop games that use `Alt+Shift` as a game action from accidentally triggering a system-wide language switch.

Built with [Tauri](https://tauri.app) (Rust + WebView2), so it stays light on RAM (~50 MB).

## Features

- Shows the layout of the **active window** (EN / UK / RU…), not the widget itself
- Click the language label to cycle to the next layout
- Lock the layout — snaps back if `Alt+Shift` changes it anyway (locked language turns red)
- Pin the widget's position (drag it when unpinned)
- Settings panel: **size, opacity, corner radius, background color, always-on-top**
- Autostart via the tray-icon menu
- Settings persist across restarts

## Install

Download the latest `.exe` from [Releases](../../releases) and run it.
Windows SmartScreen may warn on first run since the installer isn't code-signed — click *More info → Run anyway*.
The WebView2 runtime it needs ships with Windows 11 by default.

## Usage

| Action | Effect |
|---|---|
| Click the language label | Switch the active window's layout to the next one |
| Click the lock icon | Lock / unlock the current layout |
| Click the pin icon | Lock / unlock the widget's position |
| Click the gear icon | Open the settings panel |
| Tray icon → right-click | Settings / Autostart / Exit |

## Known limitation

In **exclusive fullscreen** games the widget isn't visible and layout enforcement may not apply. Use borderless/windowed mode.

## Build from source

Prerequisites:

- [Rust](https://rustup.rs) (stable, MSVC toolchain — the default on Windows)
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- [Node.js](https://nodejs.org)
- WebView2 runtime (preinstalled on Windows 11)

```bash
npm install
npm run dev      # run in development
npm run build    # produce the installer in src-tauri/target/release/bundle
```

## Release

Push a `v*` tag to trigger the CI build on GitHub:

```bash
git tag v1.0.0
git push origin v1.0.0
```

The workflow builds on `windows-latest` (MSVC + WebView2 already present) and attaches the `.exe` to the Release.

## License

[MIT](LICENSE)

## Author

[Mykola Moskal](https://www.linkedin.com/in/mykola-moskal-228749194)
