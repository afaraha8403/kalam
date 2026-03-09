//! Windows-specific: suppress Start Menu when releasing Win key after dictation hotkey (Ctrl+Win).
//! Injects a dummy key (VK_E8, unassigned) before the Win key release is processed so Windows
//! does not open the Start Menu.

use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYUP,
};

const VK_LWIN: u32 = 0x5B;
const VK_RWIN: u32 = 0x5C;
const VK_E8: u16 = 0xE8; // Unassigned; used to suppress Start Menu

static mut HOOK_HANDLE: isize = 0;

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code == HC_ACTION as i32 && w_param == WM_KEYUP as WPARAM && l_param != 0 {
        let hook_struct = &*(l_param as *const KBDLLHOOKSTRUCT);
        let vk = hook_struct.vkCode;
        if (vk == VK_LWIN || vk == VK_RWIN) && crate::hotkey::HOTKEY_ACTIVE.load(Ordering::SeqCst) {
            inject_dummy_key();
        }
    }
    CallNextHookEx(HOOK_HANDLE as HHOOK, n_code, w_param, l_param)
}

/// Inject VK_E8 key down and key up so Windows does not open Start Menu on Win key release.
unsafe fn inject_dummy_key() {
    let mut inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_E8,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VK_E8,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];
    SendInput(
        inputs.len() as u32,
        inputs.as_mut_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    );
}

/// Start a dedicated thread that runs a low-level keyboard hook to inject VK_E8
/// when the Win key is released while the dictation hotkey was active.
pub fn start_win_key_suppression() {
    std::thread::spawn(|| unsafe {
        let hmod = GetModuleHandleW(null_mut());
        if hmod == 0 {
            log::error!("GetModuleHandleW failed");
            return;
        }
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), hmod, 0);
        HOOK_HANDLE = hook;
        if hook == 0 {
            log::error!("SetWindowsHookExW WH_KEYBOARD_LL failed");
            return;
        }
        log::info!("Windows key suppression hook installed (Ctrl+Win will not open Start Menu)");

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, 0 as _, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        UnhookWindowsHookEx(HOOK_HANDLE as HHOOK);
        HOOK_HANDLE = 0;
    });
}
