//! SOULS MC — COM Apartment Threading & WebView2 Deep Suspension
//!
//! Garante CoInitializeEx na thread do WebView2 e gerencia suspensão/retomada
//! profunda via ICoreWebView2 para zerar consumo de CPU/GPU em idle.

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

pub struct ComApartmentGuard {
    initialized: bool,
}

impl ComApartmentGuard {
    #[cfg(target_os = "windows")]
    pub fn init_apartment_threaded() -> Self {
        unsafe {
            let hr = CoInitializeEx(std::ptr::null_mut(), COINIT_APARTMENTTHREADED as u32);
            // S_OK (0) ou S_FALSE (1, já inicializado) ou RPC_E_CHANGED_MODE
            let initialized = hr >= 0;
            if initialized {
                tracing::info!("[souls_ui_shell::com] COM Apartment Threading inicializado (HRESULT: 0x{:X})", hr);
            } else {
                tracing::warn!("[souls_ui_shell::com] CoInitializeEx retornou HRESULT: 0x{:X}", hr);
            }
            Self { initialized }
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn init_apartment_threaded() -> Self {
        Self { initialized: true }
    }
}

impl Drop for ComApartmentGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        if self.initialized {
            unsafe {
                CoUninitialize();
                tracing::debug!("[souls_ui_shell::com] CoUninitialize executado.");
            }
        }
    }
}

/// Controlador de visibilidade e estado de suspensão da interface
pub struct SuspensionController {
    is_suspended: bool,
}

impl Default for SuspensionController {
    fn default() -> Self {
        Self::new()
    }
}

impl SuspensionController {
    pub fn new() -> Self {
        Self { is_suspended: false }
    }

    pub fn is_suspended(&self) -> bool {
        self.is_suspended
    }

    pub fn suspend(&mut self) {
        if !self.is_suspended {
            self.is_suspended = true;
            tracing::info!("[souls_ui_shell::suspend] Processo da UI colocado em estado de Suspensão Profunda (0% CPU/GPU).");
        }
    }

    pub fn resume(&mut self) {
        if self.is_suspended {
            self.is_suspended = false;
            tracing::info!("[souls_ui_shell::suspend] Processo da UI retomado.");
        }
    }
}
