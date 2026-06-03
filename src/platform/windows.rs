use super::PlatformCapabilities;
use crate::clipboard::ClipboardEvent;
use crate::platform::{
    AppChoice, HotkeyConfig, HotkeyUpdateHandle, PasteMethod, ScreenGeometry, TrayHandle,
};
use crossbeam_channel::Sender;

pub fn active_app_name() -> String {
    "Windows".to_string()
}

pub fn platform_note() -> &'static str {
    "Windows 预留模式：平台抽象已存在，后续可接入 Win32 剪贴板、窗口追踪和全局快捷键。"
}

#[allow(dead_code)]
pub fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        active_window: "预留：GetForegroundWindow + GetWindowTextW",
        window_management: "egui viewport 基础窗口控制",
        global_hotkey: "预留：RegisterHotKey 或 global-hotkey",
        tray: "预留：tray-icon Windows backend",
        rich_clipboard: "预留：Win32 Clipboard formats",
    }
}

pub fn start_hotkey_listener(
    _sender: Sender<ClipboardEvent>,
    _ctx: egui::Context,
    _config: HotkeyConfig,
) -> HotkeyUpdateHandle {
    let (sender, _receiver) = crossbeam_channel::unbounded();
    HotkeyUpdateHandle::new(sender)
}

pub fn start_tray(
    _sender: Sender<ClipboardEvent>,
    _ctx: egui::Context,
    _enabled: bool,
) -> Option<TrayHandle> {
    None
}

pub fn screen_size() -> Option<(f32, f32)> {
    None
}

pub fn mouse_position() -> Option<(f32, f32)> {
    None
}

pub fn screen_geometry() -> Option<ScreenGeometry> {
    None
}

pub fn simulate_paste(_prefer_formatted: bool, _method: PasteMethod) -> Result<(), String> {
    Err("Windows 粘贴模拟仍使用预留 Win32 后端".to_string())
}

pub fn discover_apps_for_mime(_mime: &str) -> Vec<AppChoice> {
    Vec::new()
}
