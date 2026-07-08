//! Win32 keyboard-layout access.
//!
//! Mirrors what the original AutoHotkey prototype did, but through the
//! `windows` crate instead of raw DllCall:
//!   - read the *foreground window's* layout (not this app's)
//!   - switch it via WM_INPUTLANGCHANGEREQUEST
//!   - map an HKL's low word (an LCID) to an ISO-639 abbreviation (EN/UK/…)

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardLayoutList, HKL};
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

/// All installed layouts, in the system's own order (may contain duplicates
/// for the same language — that mirrors the OS list and Alt+Shift cycling).
pub fn layout_list() -> Vec<usize> {
    unsafe {
        let count = GetKeyboardLayoutList(None);
        if count <= 0 {
            return Vec::new();
        }
        let mut list = vec![HKL::default(); count as usize];
        GetKeyboardLayoutList(Some(&mut list));
        list.iter().map(|h| h.0 as usize).collect()
    }
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

/// Cycle the foreground window to the next installed layout; returns the HKL
/// that was applied.
pub fn switch_next() -> usize {
    let list = layout_list();
    if list.is_empty() {
        return foreground_hkl();
    }
    let cur = foreground_hkl();
    let next = match list.iter().position(|&h| h == cur) {
        Some(i) if i + 1 < list.len() => list[i + 1],
        _ => list[0],
    };
    apply_hkl(next);
    next
}

// HWND is used only through the calls above; keep the import meaningful.
#[allow(dead_code)]
fn _assert_hwnd(_: HWND) {}
