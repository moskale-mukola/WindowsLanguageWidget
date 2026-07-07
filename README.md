# KeyboardLangLock

A Windows 11 desktop widget that shows the active window's keyboard layout and lets you **lock it** — built to stop games that use `Alt+Shift` as a game action from accidentally triggering a system-wide language switch.

![screenshot](docs/screenshot.png)

## Features

- Shows the layout of the **active window** (EN / UK / RU…), not the widget itself
- Click the language label to cycle to the next layout
- 🔓/🔒 — lock the layout (snaps back if `Alt+Shift` changes it anyway; locked language turns red)
- 📍/📌 — lock the widget's position (drag by the background while unlocked)
- Autostart via the tray menu
- Position and state persisted to `LangWidget.ini`

## Install

**Prebuilt `.exe`** — download the latest `LangWidget.exe` from [Releases](../../releases) and run it.
Windows SmartScreen may warn on first run since the binary isn't code-signed — click *More info → Run anyway*, or build it yourself (see below).

**From source** — install [AutoHotkey v2](https://www.autohotkey.com/download/ahk-v2.exe), then run [`src/LangWidget.ahk`](src/LangWidget.ahk).

## Usage

| Action | Effect |
|---|---|
| Click language label | Switch active window's layout to the next one |
| Click 🔓 | Lock the current layout |
| Click 🔒 | Unlock |
| Click 📍 | Lock widget position |
| Click 📌 | Unlock (drag by background) |
| Tray icon → right-click | Autostart / Reload / Exit |

## Known limitation

In **exclusive fullscreen** games the widget isn't visible and the layout enforcement may not apply. Use borderless/windowed mode.

## Building the .exe yourself

1. Install [AutoHotkey v2](https://www.autohotkey.com/download/ahk-v2.exe) — this bundles the `Ahk2Exe` compiler (default: `C:\Program Files\AutoHotkey\Compiler\Ahk2exe.exe`).
2. Open `Ahk2Exe`, set `Source` to `src\LangWidget.ahk`, pick a `Destination`, click `Convert`.

The `.exe` isn't committed to git (see [`.gitignore`](.gitignore)) — it's published via GitHub Releases.

## Releasing

Pushing a `v*` tag triggers [`.github/workflows/build.yml`](.github/workflows/build.yml), which compiles the `.exe` and attaches it to a GitHub Release:

```bash
git tag v1.0.0
git push origin v1.0.0
```

## Roadmap

- App icon (`.ico`) embedded in the `.exe`
- Tray/INI settings for colors, size, which languages show full names
- "Hard lock" option: unbind the system `Alt+Shift` hotkey instead of snapping back
- Alternate skins for different docks

## License

[MIT](LICENSE)

## Author

[Mykola Moskal](https://www.linkedin.com/in/mykola-moskal-228749194)
