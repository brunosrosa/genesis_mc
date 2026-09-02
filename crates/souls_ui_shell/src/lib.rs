//! SOULS MC — UI Shell (Wry + Winit + Win32 DWM Desktop Chassis)
//!
//! Arquitetura bare-metal de alto desempenho para Windows 11 com 0.0% GPU em idle.

pub mod dwm;
pub mod hotkey;
pub mod ipc;
pub mod suspend;

pub use dwm::apply_native_dwm_acrylic;
pub use hotkey::{auto_deactivate_caps_lock, register_global_hotkey, unregister_global_hotkey};
pub use ipc::{IpcBridge, WebViewProxy};
pub use suspend::{ComApartmentGuard, SuspensionController};
