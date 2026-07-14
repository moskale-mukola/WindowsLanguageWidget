//! Global low-level keyboard hook that PREVENTS the language-switch
//! hotkeys (Alt+Shift / Ctrl+Shift, in either press order) from being
//! recognized while the layout lock is on — instead of letting the switch
//! happen and reverting it after the fact, which games notice (they rebind
//! keys on the switch event and never see our revert).
//!
//! How it works: Windows commits the hotkey toggle only if the modifier
//! pair is pressed and released "cleanly" — pressing any other key in
//! between cancels it (that's why Alt+Shift+Tab doesn't switch language).
//! So when the second modifier of a pair goes down, we inject a harmless
//! dummy key (VK 0xFF, reserved/no-op). Nothing is swallowed: the game
//! still receives the real Alt and Shift events and its own combos keep
//! working, but the system's toggle detector resets, so the layout never
//! changes in the first place.
//!
//! Win+Space and switching from the taskbar language indicator are
//! intentionally left alone — those are deliberate actions, not accidents.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use windows::Win32::Foundation::{HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetForegroundWindow, GetMessageW, SetWindowsHookExW,
    TranslateMessage, EVENT_SYSTEM_FOREGROUND, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
    WH_KEYBOARD_LL, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS, WM_KEYDOWN, WM_SYSKEYDOWN,
};

/// Set by the enforcement loop: true while (layout locked && blocking
/// enabled in settings). The hook itself stays installed permanently and
/// is a no-op when this is false.
pub static BLOCK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// The most recent *external* foreground window (as an isize HWND). Kept up
/// to date by a WINEVENT_SKIPOWNPROCESS foreground hook, so our own windows
/// (widget/settings) never overwrite it — the widget therefore always tracks
/// the real active app, even when it briefly holds focus itself. 0 until the
/// first foreground event or the install-time seed.
pub static LAST_FOREGROUND: AtomicIsize = AtomicIsize::new(0);

pub fn last_foreground() -> isize {
    LAST_FOREGROUND.load(Ordering::Relaxed)
}

unsafe extern "system" fn winevent_proc(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND && !hwnd.0.is_null() {
        LAST_FOREGROUND.store(hwnd.0 as isize, Ordering::Relaxed);
    }
}

const VK_DUMMY: u16 = 0xE8; // unassigned VK — apps ignore it (0xFF gets filtered by some input paths)

fn key_held(vk: VIRTUAL_KEY) -> bool {
    unsafe { (GetAsyncKeyState(vk.0 as i32) as u16) & 0x8000 != 0 }
}

/// Inject a no-op key press+release to reset the hotkey toggle detector.
fn send_dummy() {
    let mut down = INPUT::default();
    down.r#type = INPUT_KEYBOARD;
    down.Anonymous.ki = KEYBDINPUT {
        wVk: VIRTUAL_KEY(VK_DUMMY),
        ..Default::default()
    };
    let mut up = down;
    up.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
    unsafe {
        SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
    }
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && BLOCK_ACTIVE.load(Ordering::Relaxed) {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            // Skip injected events (including our own dummy key).
            if kb.flags.0 & LLKHF_INJECTED.0 == 0 {
                let vk = kb.vkCode;
                let is_shift = vk == 0x10 || vk == 0xA0 || vk == 0xA1;
                let is_alt = vk == 0x12 || vk == 0xA4 || vk == 0xA5;
                let is_ctrl = vk == 0x11 || vk == 0xA2 || vk == 0xA3;
                // Second modifier of a toggle pair just went down (either
                // press order) — break the combo before the system sees a
                // clean pair.
                let completes_pair = (is_shift && (key_held(VK_MENU) || key_held(VK_CONTROL)))
                    || (is_alt && key_held(VK_SHIFT))
                    || (is_ctrl && key_held(VK_SHIFT));
                if completes_pair {
                    send_dummy();
                }
            }
        }
    }
    CallNextHookEx(HHOOK::default(), code, wparam, lparam)
}

/// Install both hooks on a dedicated thread with a message loop (required
/// for WH_KEYBOARD_LL callbacks and for OUTOFCONTEXT WinEvents to be
/// delivered).
pub fn install() {
    std::thread::spawn(|| unsafe {
        let _kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), HINSTANCE::default(), 0);

        // Seed with whatever is foreground right now, but only if main()
        // didn't already capture the real pre-launch foreground (by now our
        // widget may have grabbed focus, so we'd rather not overwrite a good
        // value with our own window). SKIPOWNPROCESS keeps our windows from
        // ever registering as the "active app" thereafter.
        if LAST_FOREGROUND.load(Ordering::Relaxed) == 0 {
            let fg = GetForegroundWindow();
            if !fg.0.is_null() {
                LAST_FOREGROUND.store(fg.0 as isize, Ordering::Relaxed);
            }
        }
        let _we = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            HMODULE::default(),
            Some(winevent_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}
