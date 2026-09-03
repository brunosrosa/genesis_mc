//! SOULS MC — Windows 11 DWM Native Composition Engine
//!
//! Configuração FFI de Desktop Acrylic (`DWMSBT_TRANSIENTWINDOW`), margens
//! DirectComposition e estilos de janela estendidos com 0.0% GPU em idle.

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::HWND;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT,
};

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
pub struct MARGINS {
    pub cxLeftWidth: i32,
    pub cxRightWidth: i32,
    pub cyTopHeight: i32,
    pub cyBottomHeight: i32,
}

#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmExtendFrameIntoClientArea(hwnd: HWND, pMarInset: *const MARGINS) -> i32;
    pub fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwAttribute: u32,
        pvAttribute: *const core::ffi::c_void,
        cbAttribute: u32,
    ) -> i32;
}

/// Atributo DWMWA_SYSTEMBACKDROP_TYPE introduzido no Windows 11 Build 22621
pub const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;
/// Material Desktop Acrylic (Frosted Glass profundo)
pub const DWMSBT_TRANSIENTWINDOW: u32 = 3;
/// Material Mica Alt (Tabbed Window de alto contraste)
pub const DWMSBT_TABBEDWINDOW: u32 = 4;
/// Material Mica Padrão
pub const DWMSBT_MAINWINDOW: u32 = 2;

/// Aplica o material Desktop Acrylic nativo do Windows 11 no HWND da janela
///
/// # Safety
/// O chamador deve garantir que `hwnd` seja um manipulador de janela Win32 válido
/// com suporte à composição DWM.
#[cfg(target_os = "windows")]
pub unsafe fn apply_native_dwm_acrylic(hwnd: HWND) -> Result<(), String> {
    if hwnd.is_null() {
        return Err("HWND inválido (null)".to_string());
    }

    // 1. Injetar estilos de janela estendidos para suporte a camadas e flutuação
    let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let new_ex_style = ex_style | (WS_EX_TOPMOST as isize) | (WS_EX_LAYERED as isize);
    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_ex_style);

    // 2. Estender a margem DirectComposition para 100% da área do cliente (-1)
    let margins = MARGINS {
        cxLeftWidth: -1,
        cxRightWidth: -1,
        cyTopHeight: -1,
        cyBottomHeight: -1,
    };
    let hr_margins = DwmExtendFrameIntoClientArea(hwnd, &margins);
    if hr_margins != 0 {
        tracing::warn!("[souls_ui_shell::dwm] DwmExtendFrameIntoClientArea retornou HRESULT: 0x{:X}", hr_margins);
    }

    // 3. Solicitar ao DWM do Windows 11 o Desktop Acrylic (DWMSBT_TRANSIENTWINDOW = 3)
    let backdrop_type: u32 = DWMSBT_TRANSIENTWINDOW;
    let hr_attr = DwmSetWindowAttribute(
        hwnd,
        DWMWA_SYSTEMBACKDROP_TYPE,
        &backdrop_type as *const _ as *const _,
        std::mem::size_of::<u32>() as u32,
    );

    if hr_attr != 0 {
        tracing::warn!("[souls_ui_shell::dwm] DwmSetWindowAttribute (Backdrop) retornou HRESULT: 0x{:X}", hr_attr);
    } else {
        tracing::info!("[souls_ui_shell::dwm] Desktop Acrylic (DWM) injetado com sucesso no HWND {:?}", hwnd);
    }

    Ok(())
}

/// Alterna dinamicamente a transparência a cliques (Click-Through)
///
/// # Safety
/// O chamador deve garantir que `hwnd` seja um HWND Win32 válido.
#[cfg(target_os = "windows")]
pub unsafe fn set_click_through(hwnd: HWND, passthrough: bool) {
    if hwnd.is_null() {
        return;
    }
    let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    let new_style = if passthrough {
        current_style | (WS_EX_TRANSPARENT as isize)
    } else {
        current_style & !(WS_EX_TRANSPARENT as isize)
    };

    if current_style != new_style {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
    }
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn apply_native_dwm_acrylic(_hwnd: isize) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn set_click_through(_hwnd: isize, _passthrough: bool) {}
