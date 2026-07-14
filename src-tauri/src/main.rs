// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hook;
mod keyboard;
mod toggle;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, State, WindowEvent, Wry};
use tauri_plugin_autostart::ManagerExt;

// ---------- Shared runtime state (used by the enforcement thread) ----------
struct Lock {
    layout_locked: bool,
    target: usize,
    last_enforce: Instant,
    // Last foreground window that wasn't one of our own (widget/settings).
    // Enforcement and lock-target capture act on this, not a fresh
    // GetForegroundWindow() call, so opening the settings panel (which can
    // steal focus) doesn't corrupt which app we're tracking.
    last_external_hwnd: isize,
    // Our own window handles, excluded when picking "the target app".
    own_hwnds: Vec<isize>,
    // The widget window specifically. If it ever becomes the foreground
    // window (WebView2 grabs focus on click), we bounce focus straight back
    // to last_external_hwnd so the game keeps the keyboard.
    widget_hwnd: isize,
    // Mirror of the corresponding Settings field, kept here so the
    // enforcement loop can sync the blocking layers without re-reading
    // the settings file. Drives both the keyboard hook and the
    // registry-level hotkey disable.
    block_hotkeys: bool,
}

impl Default for Lock {
    fn default() -> Self {
        Lock {
            layout_locked: false,
            target: 0,
            last_enforce: Instant::now(),
            last_external_hwnd: 0,
            own_hwnds: Vec::new(),
            widget_hwnd: 0,
            block_hotkeys: false,
        }
    }
}

struct AppState {
    lock: Mutex<Lock>,
    // The tray's "Autostart" check item, so the settings panel and the tray
    // menu stay in sync when either one toggles autostart.
    autostart_item: Mutex<Option<CheckMenuItem<Wry>>>,
}

// ---------- Persisted settings ----------
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
struct Settings {
    scale: f64,
    opacity: u32,     // 20..100 (percent, applied to card background)
    radius: u32,      // corner radius in px
    bg_color: String, // hex without '#'
    always_on_top: bool,
    pos_x: i32,
    pos_y: i32,
    layout_locked: bool,
    pos_locked: bool,
    show_lock: bool,
    show_pin: bool,
    show_settings: bool,
    theme: String,
    custom_css: String,
    // Experimental single "hard block" switch. While locked it engages
    // BOTH blocking layers: the keyboard hook (breaks Alt+Shift /
    // Ctrl+Shift before the switch fires) and the registry-level disable
    // (HKCU\Keyboard Layout\Toggle + SPI_SETLANGTOGGLE — the reliable one
    // that games respect). Off by default because it writes to the
    // registry; guarded by a backup file and a RunOnce restore entry.
    // Win+Space and the taskbar indicator keep working either way.
    block_hotkeys: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            scale: 1.0,
            opacity: 100,
            radius: 16,
            bg_color: "1E1E1E".into(),
            always_on_top: true,
            pos_x: 1000,
            pos_y: 400,
            layout_locked: false,
            pos_locked: false,
            show_lock: true,
            show_pin: true,
            show_settings: true,
            theme: "default".into(),
            custom_css: String::new(),
            block_hotkeys: false,
        }
    }
}

fn settings_path(app: &AppHandle) -> std::path::PathBuf {
    let dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    let _ = std::fs::create_dir_all(&dir);
    dir.join("settings.json")
}

fn load_settings(app: &AppHandle) -> Settings {
    match std::fs::read_to_string(settings_path(app)) {
        Ok(txt) => serde_json::from_str(&txt).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

/// Current external (non-own) foreground HWND + its keyboard layout.
/// Prefers the WinEvent-tracked foreground (updated system-wide, skips our
/// own process) so we always follow the real active window even when the
/// widget briefly grabs focus. Falls back to a fresh GetForegroundWindow(),
/// then to the last known external window.
fn current_external(s: &mut Lock) -> (isize, usize) {
    let tracked = hook::last_foreground();
    let raw = keyboard::foreground_hwnd_raw();
    if tracked != 0 && !s.own_hwnds.contains(&tracked) {
        s.last_external_hwnd = tracked;
    } else if raw != 0 && !s.own_hwnds.contains(&raw) {
        s.last_external_hwnd = raw;
    }
    let hwnd = s.last_external_hwnd;
    (hwnd, keyboard::hwnd_hkl(hwnd))
}

// ---------- Commands ----------
#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    load_settings(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<'_, Arc<AppState>>, settings: Settings) {
    {
        let mut s = state.lock.lock().unwrap();
        s.block_hotkeys = settings.block_hotkeys;
    }
    if let Ok(txt) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(settings_path(&app), txt);
    }
}

/// Reset appearance/behavior to defaults, but keep the widget's current
/// position and lock state — a "reset settings" click shouldn't yank the
/// widget across the screen or drop an active lock mid-session.
#[tauri::command]
fn reset_settings(app: AppHandle, state: State<'_, Arc<AppState>>) -> Settings {
    let current = load_settings(&app);
    let d = Settings::default();
    let merged = Settings {
        pos_x: current.pos_x,
        pos_y: current.pos_y,
        layout_locked: current.layout_locked,
        pos_locked: current.pos_locked,
        ..d
    };
    {
        let mut s = state.lock.lock().unwrap();
        s.block_hotkeys = merged.block_hotkeys;
    }
    if let Ok(txt) = serde_json::to_string_pretty(&merged) {
        let _ = std::fs::write(settings_path(&app), txt);
    }
    merged
}

#[tauri::command]
fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(app: AppHandle, state: State<'_, Arc<AppState>>, enabled: bool) {
    let mgr = app.autolaunch();
    if enabled {
        let _ = mgr.enable();
    } else {
        let _ = mgr.disable();
    }
    // Keep the tray menu's check item in sync with the settings panel.
    if let Some(item) = state.autostart_item.lock().unwrap().as_ref() {
        let _ = item.set_checked(enabled);
    }
}

#[tauri::command]
fn get_status(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    let mut s = state.lock.lock().unwrap();
    let hkl = if s.layout_locked {
        s.target
    } else {
        current_external(&mut s).1
    };
    json!({ "lang": keyboard::lang_of_hkl(hkl), "locked": s.layout_locked })
}

#[tauri::command]
fn toggle_lock(state: State<'_, Arc<AppState>>) -> bool {
    let mut s = state.lock.lock().unwrap();
    s.layout_locked = !s.layout_locked;
    if s.layout_locked {
        let (_, hkl) = current_external(&mut s);
        s.target = hkl;
    }
    let hard_block = s.layout_locked && s.block_hotkeys;
    hook::BLOCK_ACTIVE.store(hard_block, Ordering::Relaxed);
    toggle::set_blocked(hard_block);
    s.layout_locked
}

#[tauri::command]
fn set_always_on_top(app: AppHandle, value: bool) {
    if let Some(w) = app.get_webview_window("widget") {
        let _ = w.set_always_on_top(value);
    }
}

#[tauri::command]
fn resize_widget(app: AppHandle, width: f64, height: f64) {
    if let Some(w) = app.get_webview_window("widget") {
        let _ = w.set_size(LogicalSize::new(width, height));
    }
}

#[tauri::command]
fn open_settings(app: AppHandle) {
    show_settings_window(&app);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn show_settings_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

// Native "already running" notice, shown when a second instance is launched.
#[cfg(windows)]
fn notify_already_running() {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};
    let text: Vec<u16> = "WindowsLanguageWidget is already running.\0"
        .encode_utf16()
        .collect();
    let title: Vec<u16> = "WindowsLanguageWidget\0".encode_utf16().collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

#[cfg(not(windows))]
fn notify_already_running() {}

// Subclass proc that refuses activation on mouse clicks. Combined with
// WS_EX_NOACTIVATE this guarantees clicking anywhere on the widget (including
// the WebView2 client area, which otherwise grabs focus) never pulls the
// foreground away from the game — so the layout indicator keeps tracking the
// real active window in real time.
#[cfg(windows)]
unsafe extern "system" fn noactivate_subclass(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
    _uid: usize,
    _data: usize,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{MA_NOACTIVATE, WM_MOUSEACTIVATE};
    if msg == WM_MOUSEACTIVATE {
        return LRESULT(MA_NOACTIVATE as isize);
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

// Keep the widget from stealing focus when clicked (WS_EX_NOACTIVATE +
// WM_MOUSEACTIVATE subclass), so GetForegroundWindow keeps pointing at the
// game/app, not our widget.
#[cfg(windows)]
fn make_noactivate(app: &AppHandle) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::SetWindowSubclass;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    if let Some(w) = app.get_webview_window("widget") {
        if let Ok(h) = w.hwnd() {
            let hwnd = HWND(h.0 as *mut _);
            unsafe {
                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
                let _ = SetWindowSubclass(hwnd, Some(noactivate_subclass), 1, 0);
            }
        }
    }
}

fn main() {
    // Capture the active window before any of our own windows exist, so we
    // know the real foreground app even if the widget grabs focus on its
    // first paint. The bounce/tracking below relies on this being seeded.
    let initial_fg = keyboard::foreground_hwnd_raw();
    if initial_fg != 0 {
        hook::LAST_FOREGROUND.store(initial_fg, Ordering::Relaxed);
    }

    let state = Arc::new(AppState {
        lock: Mutex::new(Lock::default()),
        autostart_item: Mutex::new(None),
    });

    tauri::Builder::default()
        // Single-instance guard MUST be the first plugin registered. When a
        // second copy is launched it hands off to the running one (which
        // surfaces the widget + a notice) and the new process exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("widget") {
                let _ = w.show();
                let _ = w.set_focus();
            }
            notify_already_running();
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            reset_settings,
            get_autostart,
            set_autostart,
            get_status,
            toggle_lock,
            set_always_on_top,
            resize_widget,
            open_settings,
            quit_app,
        ])
        .on_window_event(|window, event| {
            // Keep the settings window alive across closes so it can reopen.
            if window.label() == "settings" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();

            // Record our own window handles so the enforcement loop never
            // mistakes the widget or settings panel for "the target app".
            {
                let mut own = Vec::new();
                let mut widget_hwnd = 0isize;
                if let Some(w) = app.get_webview_window("widget") {
                    if let Ok(h) = w.hwnd() {
                        widget_hwnd = h.0 as isize;
                        own.push(widget_hwnd);
                    }
                }
                if let Some(w) = app.get_webview_window("settings") {
                    if let Ok(h) = w.hwnd() {
                        own.push(h.0 as isize);
                    }
                }
                let mut s = state.lock.lock().unwrap();
                s.own_hwnds = own;
                s.widget_hwnd = widget_hwnd;
            }

            // Restore persisted lock state + position/always-on-top.
            let settings = load_settings(&handle);
            {
                let mut s = state.lock.lock().unwrap();
                s.layout_locked = settings.layout_locked;
                s.block_hotkeys = settings.block_hotkeys;
                if s.layout_locked {
                    let (_, hkl) = current_external(&mut s);
                    s.target = hkl;
                }
            }

            // Install the language-hotkey-breaking keyboard hook. It stays
            // installed for the app's lifetime; BLOCK_ACTIVE (synced by the
            // enforcement loop below) decides whether it interferes.
            hook::install();

            // Registry-level hotkey disabling (the reliable layer): restore
            // leftovers from a crashed run, then let the enforcement loop
            // keep it in sync with the lock state.
            toggle::init(
                handle
                    .path()
                    .app_config_dir()
                    .unwrap_or_else(|_| std::env::temp_dir())
                    .join("hotkey_backup.json"),
            );
            if let Some(w) = app.get_webview_window("widget") {
                let _ = w.set_always_on_top(settings.always_on_top);
                let _ = w.set_position(tauri::LogicalPosition::new(
                    settings.pos_x as f64,
                    settings.pos_y as f64,
                ));
            }

            #[cfg(windows)]
            make_noactivate(&handle);

            // Tray icon + menu.
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let autostart_i = CheckMenuItem::with_id(
                app,
                "autostart",
                "Autostart",
                true,
                app.autolaunch().is_enabled().unwrap_or(false),
                None::<&str>,
            )?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &autostart_i, &sep, &quit_i])?;

            // Share the check item with the settings panel (via AppState) and
            // with the tray click handler, so all three stay in sync.
            *state.autostart_item.lock().unwrap() = Some(autostart_i.clone());
            let autostart_tray = autostart_i.clone();

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("WindowsLanguageWidget")
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "settings" => show_settings_window(app),
                    "quit" => app.exit(0),
                    "autostart" => {
                        let mgr = app.autolaunch();
                        let enabled = !mgr.is_enabled().unwrap_or(false);
                        if enabled {
                            let _ = mgr.enable();
                        } else {
                            let _ = mgr.disable();
                        }
                        let _ = autostart_tray.set_checked(enabled);
                        // Sync the settings panel's checkbox if it's open.
                        let _ = app.emit_to("settings", "autostart-changed", enabled);
                    }
                    _ => {}
                })
                .build(app)?;

            // Background thread: enforce the lock and push layout updates.
            // Enforcement itself always acts on the real observed layout of
            // the tracked external window (never a blindly-trusted
            // "target"), so a stuck/ignored request is retried correctly.
            // The UI display, however, shows the stable locked-target label
            // while locked instead of the raw observed value, so a brief
            // Alt+Shift-then-snap-back doesn't flicker the widget.
            // Throttles re-enforcement to 100ms to avoid hammering apps that
            // ignore/fight the request while still correcting fast enough
            // to feel instant.
            let st = state.clone();
            std::thread::spawn(move || {
                let mut last: Option<(String, bool)> = None;
                loop {
                    let (hwnd, cur_hkl, locked, target, should_enforce) = {
                        let mut s = st.lock.lock().unwrap();
                        // If our widget stole the foreground (WebView2 does this
                        // on click, even with WS_EX_NOACTIVATE), hand it right
                        // back to the last real app so the game keeps the
                        // keyboard and the indicator keeps tracking it live.
                        let fg = keyboard::foreground_hwnd_raw();
                        if fg == s.widget_hwnd
                            && s.last_external_hwnd != 0
                            && s.last_external_hwnd != s.widget_hwnd
                        {
                            keyboard::set_foreground(s.last_external_hwnd);
                        }
                        let (hwnd, cur_hkl) = current_external(&mut s);
                        let can_enforce = s.last_enforce.elapsed() > Duration::from_millis(100);
                        let should = can_enforce && s.layout_locked && s.target != 0;
                        // Keep both blocking layers in sync with lock state.
                        let hard_block = s.layout_locked && s.block_hotkeys;
                        hook::BLOCK_ACTIVE.store(hard_block, Ordering::Relaxed);
                        toggle::set_blocked(hard_block);
                        (hwnd, cur_hkl, s.layout_locked, s.target, should)
                    };

                    if should_enforce
                        && cur_hkl != 0
                        && (cur_hkl & 0xFFFF) != (target & 0xFFFF)
                    {
                        keyboard::apply_hkl_to(hwnd, target);
                        let mut s = st.lock.lock().unwrap();
                        s.last_enforce = Instant::now();
                    }

                    // While locked, display the stable target instead of the
                    // raw observed value so a corrected Alt+Shift blip
                    // doesn't flicker the widget.
                    let display_hkl = if locked { target } else { cur_hkl };
                    let cur = (keyboard::lang_of_hkl(display_hkl), locked);
                    if last.as_ref() != Some(&cur) {
                        let _ = handle.emit_to(
                            "widget",
                            "layout",
                            json!({ "lang": cur.0, "locked": cur.1 }),
                        );
                        last = Some(cur);
                    }
                    std::thread::sleep(Duration::from_millis(80));
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, event| {
            // Whatever way the app exits, put the user's language hotkeys
            // back (no-op if they weren't disabled).
            if let tauri::RunEvent::Exit = event {
                toggle::set_blocked(false);
            }
        });
}
