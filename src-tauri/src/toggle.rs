//! Temporarily disables Windows' language-switch hotkeys (Alt+Shift /
//! Ctrl+Shift / grave) while the layout lock is on.
//!
//! Unlike reverting the layout after the fact (games see the switch event
//! and rebind their keys before we can undo it) or breaking the key combo
//! with injected dummy keys (Win11 commits the toggle too early for that),
//! this removes the hotkey itself: `HKCU\Keyboard Layout\Toggle` is set to
//! "disabled" and `SystemParametersInfoW(SPI_SETLANGTOGGLE)` makes the
//! session re-read it immediately — no relogin needed. While locked, the
//! system simply has no keyboard shortcut for switching layouts, so there
//! is no event for the game to notice. Win+Space and the taskbar language
//! indicator are unaffected.
//!
//! The user's original values are restored on unlock and on app exit. As a
//! crash guard they're also written to a backup file first; if the app
//! died while hotkeys were disabled, the next launch restores them.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, SPI_SETLANGTOGGLE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
};
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const TOGGLE_KEY: &str = "Keyboard Layout\\Toggle";
// "1" = Alt+Shift, "2" = Ctrl+Shift, "3" = disabled, "4" = grave accent.
// "Hotkey" is the legacy combined value some builds still honor.
const VALUE_NAMES: [&str; 3] = ["Language Hotkey", "Layout Hotkey", "Hotkey"];
const DISABLED: &str = "3";

// Last-resort safety net: while the hotkeys are disabled, a RunOnce entry
// holds the exact command that puts the user's values back. If this app
// dies (crash, power loss) and is never launched again, Windows itself
// runs the restore at the next logon. Removed again on a clean restore.
const RUNONCE_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce";
const RUNONCE_NAME: &str = "WindowsLanguageWidget_RestoreHotkeys";

static BACKUP_PATH: OnceLock<PathBuf> = OnceLock::new();
// Some(original values) while the hotkeys are disabled; None otherwise.
// A value of None inside the vec means "didn't exist before" (delete it
// on restore instead of writing).
static SAVED: Mutex<Option<Vec<(String, Option<String>)>>> = Mutex::new(None);

pub fn init(backup_path: PathBuf) {
    // Crash recovery: a leftover backup means a previous run died while
    // the hotkeys were disabled — put the user's values back.
    if let Ok(txt) = std::fs::read_to_string(&backup_path) {
        if let Ok(saved) = serde_json::from_str::<Vec<(String, Option<String>)>>(&txt) {
            write_values(&saved);
            notify_system();
        }
        let _ = std::fs::remove_file(&backup_path);
        clear_runonce();
    }
    let _ = BACKUP_PATH.set(backup_path);
}

fn open_key() -> Option<RegKey> {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(TOGGLE_KEY, KEY_READ | KEY_WRITE)
        .ok()
}

fn read_values() -> Vec<(String, Option<String>)> {
    let key = open_key();
    VALUE_NAMES
        .iter()
        .map(|name| {
            let v = key
                .as_ref()
                .and_then(|k| k.get_value::<String, _>(name).ok());
            (name.to_string(), v)
        })
        .collect()
}

fn write_values(vals: &[(String, Option<String>)]) {
    if let Some(key) = open_key() {
        for (name, v) in vals {
            match v {
                Some(v) => {
                    let _ = key.set_value(name, v);
                }
                None => {
                    let _ = key.delete_value(name);
                }
            }
        }
    }
}

fn runonce_command(original: &[(String, Option<String>)]) -> String {
    let parts: Vec<String> = original
        .iter()
        .map(|(name, v)| match v {
            Some(v) => format!(
                "reg add \"HKCU\\{}\" /v \"{}\" /t REG_SZ /d \"{}\" /f",
                TOGGLE_KEY, name, v
            ),
            None => format!("reg delete \"HKCU\\{}\" /v \"{}\" /f", TOGGLE_KEY, name),
        })
        .collect();
    format!("cmd.exe /c {}", parts.join(" & "))
}

fn set_runonce(original: &[(String, Option<String>)]) {
    if let Ok((key, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUNONCE_KEY) {
        let _ = key.set_value(RUNONCE_NAME, &runonce_command(original));
    }
}

fn clear_runonce() {
    if let Ok(key) =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUNONCE_KEY, KEY_WRITE)
    {
        let _ = key.delete_value(RUNONCE_NAME);
    }
}

fn notify_system() {
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_SETLANGTOGGLE,
            0,
            None,
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
}

/// Idempotently disable/re-enable the system language hotkeys.
pub fn set_blocked(blocked: bool) {
    let mut saved = SAVED.lock().unwrap();
    if blocked && saved.is_none() {
        let original = read_values();
        if let Some(p) = BACKUP_PATH.get() {
            if let Ok(txt) = serde_json::to_string(&original) {
                let _ = std::fs::write(p, txt);
            }
        }
        set_runonce(&original);
        let disabled = VALUE_NAMES
            .iter()
            .map(|n| (n.to_string(), Some(DISABLED.to_string())))
            .collect::<Vec<_>>();
        write_values(&disabled);
        notify_system();
        *saved = Some(original);
    } else if !blocked {
        if let Some(original) = saved.take() {
            write_values(&original);
            notify_system();
            if let Some(p) = BACKUP_PATH.get() {
                let _ = std::fs::remove_file(p);
            }
            clear_runonce();
        }
    }
}
