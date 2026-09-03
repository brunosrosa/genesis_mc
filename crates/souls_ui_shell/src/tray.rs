//! Suporte a Systray Nativo no Windows 11 via Windows Shell API (Shell_NotifyIconW)

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    LoadIconW, IDI_APPLICATION,
};

pub const WM_SOULS_TRAY: u32 = 0x0400 + 101;
pub const SOULS_TRAY_UID: u32 = 1001;

/// Adiciona o ícone do SOULS MC na bandeja do sistema (Windows 11 Systray)
///
/// # Safety
/// O chamador deve garantir que `hwnd` seja um manipulador de janela Win32 válido
/// associado à thread principal de interface.
#[cfg(target_os = "windows")]
pub unsafe fn add_tray_icon(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }

    let hicon = LoadIconW(core::ptr::null_mut(), IDI_APPLICATION);
    let mut tip: [u16; 128] = [0; 128];
    let tip_text = "SOULS MC // SODA Desktop Assistant";
    for (i, c) in tip_text.encode_utf16().enumerate().take(127) {
        tip[i] = c;
    }

    let mut nid: NOTIFYICONDATAW = core::mem::zeroed();
    nid.cbSize = core::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = SOULS_TRAY_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_SOULS_TRAY;
    nid.hIcon = hicon;
    nid.szTip = tip;

    let res = Shell_NotifyIconW(NIM_ADD, &nid);
    if res != 0 {
        tracing::info!("[souls_ui_shell::tray] Ícone no Systray adicionado com sucesso (UID: {})", SOULS_TRAY_UID);
        true
    } else {
        tracing::warn!("[souls_ui_shell::tray] Falha ao registrar ícone no Systray");
        false
    }
}

/// Remove o ícone do SOULS MC do Systray ao encerrar
///
/// # Safety
/// O chamador deve garantir que `hwnd` seja um manipulador de janela Win32 válido
/// associado à thread principal de interface.
#[cfg(target_os = "windows")]
pub unsafe fn remove_tray_icon(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let mut nid: NOTIFYICONDATAW = core::mem::zeroed();
    nid.cbSize = core::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = SOULS_TRAY_UID;
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    tracing::info!("[souls_ui_shell::tray] Ícone do Systray removido");
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn add_tray_icon(_hwnd: isize) -> bool {
    true
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn remove_tray_icon(_hwnd: isize) {}
