// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod keyboard;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, State, WindowEvent};
use tauri_plugin_autostart::ManagerExt;

// ---------- Shared runtime state (used by the enforcement thread) ----------
struct Lock {
    layout_locked: bool,
    target: usize,
    last_enforce: Instant,
}

impl Default for Lock {
    fn default() -> Self {
        Lock {
            layout_locked: false,
            target: 0,
            last_enforce: Instant::now(),
        }
    }
}

struct AppState {
    lock: Mutex<Lock>,
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

// ---------- Commands ----------
#[tauri::command]
fn get_settings(app: AppHandle) -> Settings {
    load_settings(&app)
}

#[tauri::command]
fn save_settings(app: AppHandle, settings: Settings) {
    if let Ok(txt) = serde_json::to_string_pretty(&settings) {
        let _ = std::fs::write(settings_path(&app), txt);
    }
}

#[tauri::command]
fn get_status(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    let s = state.lock.lock().unwrap();
    let hkl = if s.layout_locked {
        s.target
    } else {
        keyboard::foreground_hkl()
    };
    json!({ "lang": keyboard::lang_of_hkl(hkl), "locked": s.layout_locked })
}


#[tauri::command]
fn toggle_lock(state: State<'_, Arc<AppState>>) -> bool {
    let mut s = state.lock.lock().unwrap();
    s.layout_locked = !s.layout_locked;
    if s.layout_locked {
        s.target = keyboard::foreground_hkl();
    }
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

// Keep the widget from stealing focus when clicked (WS_EX_NOACTIVATE), so
// GetForegroundWindow keeps pointing at the game/app, not our widget.
#[cfg(windows)]
fn make_noactivate(app: &AppHandle) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    };
    if let Some(w) = app.get_webview_window("widget") {
        if let Ok(h) = w.hwnd() {
            let hwnd = HWND(h.0 as *mut _);
            unsafe {
                let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | WS_EX_NOACTIVATE.0 as isize);
            }
        }
    }
}

fn main() {
    let state = Arc::new(AppState {
        lock: Mutex::new(Lock::default()),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state.clone())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
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

            // Restore persisted lock state + position/always-on-top.
            let settings = load_settings(&handle);
            {
                let mut s = state.lock.lock().unwrap();
                s.layout_locked = settings.layout_locked;
                if s.layout_locked {
                    s.target = keyboard::foreground_hkl();
                }
            }
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

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("KeyboardLangLock")
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "settings" => show_settings_window(app),
                    "quit" => app.exit(0),
                    "autostart" => {
                        let mgr = app.autolaunch();
                        if mgr.is_enabled().unwrap_or(false) {
                            let _ = mgr.disable();
                        } else {
                            let _ = mgr.enable();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            // Background thread: enforce the lock and push layout updates.
            // Only emits when the (lang, locked) pair actually changes.
            // Throttles lock enforcement to 500ms to prevent infinite loops with
            // programs that also react to PostMessageW.
            let st = state.clone();
            std::thread::spawn(move || {
                let mut last: Option<(String, bool)> = None;
                loop {
                    let (locked, target, should_enforce) = {
                        let s = st.lock.lock().unwrap();
                        let can_enforce = s.last_enforce.elapsed() > Duration::from_millis(500);
                        (s.layout_locked, s.target, can_enforce && s.layout_locked && s.target != 0)
                    };
                    if should_enforce {
                        let cur = keyboard::foreground_hkl();
                        if cur != 0 && (cur & 0xFFFF) != (target & 0xFFFF) {
                            keyboard::apply_hkl(target);
                            let mut s = st.lock.lock().unwrap();
                            s.last_enforce = Instant::now();
                        }
                    }
                    let hkl = if locked {
                        target
                    } else {
                        keyboard::foreground_hkl()
                    };
                    let cur = (keyboard::lang_of_hkl(hkl), locked);
                    if last.as_ref() != Some(&cur) {
                        let _ = handle.emit_to(
                            "widget",
                            "layout",
                            json!({ "lang": cur.0, "locked": cur.1 }),
                        );
                        last = Some(cur);
                    }
                    std::thread::sleep(Duration::from_millis(150));
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
