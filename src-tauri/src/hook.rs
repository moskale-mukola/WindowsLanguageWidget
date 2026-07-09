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

use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_CONTROL, VK_MENU, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, HHOOK,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

/// Set by the enforcement loop: true while (layout locked && blocking
/// enabled in settings). The hook itself stays installed permanently and
/// is a no-op when this is false.
pub static BLOCK_ACTIVE: AtomicBool = AtomicBool::new(false);

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

/// Install the hook on a dedicated thread with a message loop (required
/// for WH_KEYBOARD_LL callbacks to be delivered).
pub fn install() {
    std::thread::spawn(|| unsafe {
        if SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), HINSTANCE::default(), 0).is_ok() {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, windows::Win32::Foundation::HWND::default(), 0, 0)
                .as_bool()
            {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    });
}
