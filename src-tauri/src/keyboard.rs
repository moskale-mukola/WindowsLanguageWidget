//! Win32 keyboard-layout access.
//!
//! Mirrors what the original AutoHotkey prototype did, but through the
//! `windows` crate instead of raw DllCall:
//!   - read the *foreground window's* layout (not this app's)
//!   - switch it via WM_INPUTLANGCHANGEREQUEST
//!   - map an HKL's low word (an LCID) to an ISO-639 abbreviation (EN/UK/…)

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
};

/// HKL of the foreground window, as a plain integer (0 if none).
pub fn foreground_hkl() -> usize {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return 0;
        }
        let tid = GetWindowThreadProcessId(hwnd, None);
        GetKeyboardLayout(tid).0 as usize
    }
}

/// Uppercase ISO-639 abbreviation for an HKL value, e.g. "EN" / "UK".
pub fn lang_of_hkl(hkl: usize) -> String {
    if hkl == 0 {
        return "--".to_string();
    }
    let lcid = (hkl & 0xFFFF) as u32;
    let mut buf = [0u16; 16];
    let n = unsafe { GetLocaleInfoW(lcid, LOCALE_SISO639LANGNAME, Some(&mut buf)) };
    if n <= 0 {
        return format!("{:04X}", lcid);
    }
    String::from_utf16_lossy(&buf[..(n as usize).saturating_sub(1)]).to_uppercase()
}

/// Request the foreground window to switch to the given layout.
pub fn apply_hkl(hkl: usize) {
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            let _ = PostMessageW(
                hwnd,
                WM_INPUTLANGCHANGEREQUEST,
                WPARAM(0),
                LPARAM(hkl as isize),
            );
        }
    }
}

// HWND is used only through the calls above; keep the import meaningful.
#[allow(dead_code)]
fn _assert_hwnd(_: HWND) {}
