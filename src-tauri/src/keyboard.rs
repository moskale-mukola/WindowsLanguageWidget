//! Win32 keyboard-layout access.
//!
//! Mirrors what the original AutoHotkey prototype did, but through the
//! `windows` crate instead of raw DllCall:
//!   - read the *foreground window's* layout (not this app's)
//!   - switch it via WM_INPUTLANGCHANGEREQUEST
//!   - map an HKL's low word (an LCID) to an ISO-639 abbreviation (EN/UK/…)
//!
//! Two things matter for reliability against real games:
//!   - the top-level foreground window is not always the one that owns
//!     keyboard focus (it may be a child control) — GetGUIThreadInfo's
//!     hwndFocus is the window that actually needs the message.
//!   - our own windows (widget/settings) can become foreground (e.g. when
//!     the settings panel is clicked); callers should exclude those HWNDs
//!     before treating GetForegroundWindow()'s result as "the target app".

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, PostMessageW,
    SetForegroundWindow, GUITHREADINFO, WM_INPUTLANGCHANGEREQUEST,
};

/// Raw foreground window handle, as an integer (0 if none). Callers should
/// check this against their own windows before treating it as "the app".
pub fn foreground_hwnd_raw() -> isize {
    unsafe { GetForegroundWindow().0 as isize }
}

/// Hand the foreground back to a given window. Used to bounce focus off our
/// own widget the instant WebView2 grabs it on a click, so the game/app stays
/// the foreground window and keeps receiving the keyboard.
pub fn set_foreground(hwnd_val: isize) {
    if hwnd_val == 0 {
        return;
    }
    unsafe {
        let _ = SetForegroundWindow(HWND(hwnd_val as *mut _));
    }
}

/// HKL of the given window's thread (0 if the handle is null).
pub fn hwnd_hkl(hwnd_val: isize) -> usize {
    if hwnd_val == 0 {
        return 0;
    }
    unsafe {
        let hwnd = HWND(hwnd_val as *mut _);
        let tid = GetWindowThreadProcessId(hwnd, None);
        GetKeyboardLayout(tid).0 as usize
    }
}

/// The window that actually owns keyboard focus within `hwnd`'s thread (this
/// may be a child control, not `hwnd` itself). Falls back to `hwnd` if the
/// thread info can't be read.
fn focus_hwnd_for(hwnd: HWND) -> HWND {
    unsafe {
        let tid = GetWindowThreadProcessId(hwnd, None);
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        if GetGUIThreadInfo(tid, &mut info).is_ok() && !info.hwndFocus.0.is_null() {
            info.hwndFocus
        } else {
            hwnd
        }
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

/// Request the given window to switch to the given layout. Posts to the
/// actual focused control within that window's thread, not just the
/// top-level window, since many apps only react on the control that has
/// keyboard focus.
pub fn apply_hkl_to(hwnd_val: isize, hkl: usize) {
    if hwnd_val == 0 {
        return;
    }
    unsafe {
        let hwnd = HWND(hwnd_val as *mut _);
        let target = focus_hwnd_for(hwnd);
        let _ = PostMessageW(
            target,
            WM_INPUTLANGCHANGEREQUEST,
            WPARAM(0),
            LPARAM(hkl as isize),
        );
    }
}
