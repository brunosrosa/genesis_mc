//! SOULS MC — Global Hotkey & Atomic Caps Lock Deactivator
//!
//! Registra Shift + Caps Lock no nível Win32 OS e, ao invocar o overlay,
//! inspeciona e desliga atomicamente o Caps Lock via SendInput para apagar o LED físico.

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, RegisterHotKey, SendInput, UnregisterHotKey, INPUT, INPUT_0, INPUT_KEYBOARD,
    KEYBDINPUT, KEYEVENTF_KEYUP, MOD_NOREPEAT, MOD_SHIFT, VK_CAPITAL,
};

pub const SOULS_HOTKEY_ID: i32 = 0x50DA;

/// Registra o atalho global Shift + Caps Lock na HWND especificada
#[cfg(target_os = "windows")]
pub unsafe fn register_global_hotkey(hwnd: HWND) -> bool {
    let success = RegisterHotKey(
        hwnd,
        SOULS_HOTKEY_ID,
        (MOD_SHIFT | MOD_NOREPEAT) as u32,
        VK_CAPITAL as u32,
    );

    if success != 0 {
        tracing::info!("[souls_ui_shell::hotkey] Atalho Global Shift+CapsLock registrado com sucesso (ID: 0x{:X})", SOULS_HOTKEY_ID);
        true
    } else {
        // Fallback: tentar sem MOD_NOREPEAT caso o Windows recuse
        let fallback = RegisterHotKey(hwnd, SOULS_HOTKEY_ID, MOD_SHIFT as u32, VK_CAPITAL as u32);
        if fallback != 0 {
            tracing::info!("[souls_ui_shell::hotkey] Atalho Global Shift+CapsLock registrado via fallback");
            true
        } else {
            tracing::warn!("[souls_ui_shell::hotkey] Falha ao registrar atalho global Shift+CapsLock");
            false
        }
    }
}

/// Desregistra o atalho global ao encerrar a aplicação
#[cfg(target_os = "windows")]
pub unsafe fn unregister_global_hotkey(hwnd: HWND) {
    let _ = UnregisterHotKey(hwnd, SOULS_HOTKEY_ID);
}

/// Inspeciona o estado do Caps Lock e o desativa atomicamente se estiver ligado
#[cfg(target_os = "windows")]
pub unsafe fn auto_deactivate_caps_lock() {
    let state = GetKeyState(VK_CAPITAL as i32);
    // O bit menos significativo (LSB) indica se o Caps Lock está ativo/ligado
    if (state & 0x0001) != 0 {
        tracing::info!("[souls_ui_shell::hotkey] Caps Lock detectado ATIVO. Desativando atomicamente via SendInput...");

        let mut inputs: [INPUT; 2] = [
            // 1. KeyDown para VK_CAPITAL
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CAPITAL,
                        wScan: 0,
                        dwFlags: 0,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // 2. KeyUp para VK_CAPITAL
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CAPITAL,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        let sent = SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );

        if sent == inputs.len() as u32 {
            tracing::info!("[souls_ui_shell::hotkey] Caps Lock e LED físico desativados com sucesso.");
        } else {
            tracing::warn!("[souls_ui_shell::hotkey] SendInput retornou {}, esperado {}", sent, inputs.len());
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn register_global_hotkey(_hwnd: isize) -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn unregister_global_hotkey(_hwnd: isize) {}

#[cfg(not(target_os = "windows"))]
pub unsafe fn auto_deactivate_caps_lock() {}
