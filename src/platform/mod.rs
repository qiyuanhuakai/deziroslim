use rust_i18n::t;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub mod linux_xfixes;
#[cfg(target_os = "windows")]
mod windows;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PlatformCapabilities {
    pub active_window: String,
    pub window_management: String,
    pub global_hotkey: String,
    pub tray: String,
    pub rich_clipboard: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HotkeyConfig {
    pub main_hotkeys: String,
    pub sequential_hotkey: String,
    pub rich_paste_hotkey: String,
    pub search_hotkey: String,
    pub private_mode_hotkey: String,
    pub snippet_picker_hotkey: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyboardModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteMethod {
    Auto,
    ShiftInsert,
    CtrlV,
    TypeText,
}

impl PasteMethod {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Self {
        match value {
            "shift_insert" => PasteMethod::ShiftInsert,
            "ctrl_v" => PasteMethod::CtrlV,
            "type_text" => PasteMethod::TypeText,
            _ => PasteMethod::Auto,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppChoice {
    pub name: String,
    pub command: String,
    pub is_default: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotkeyAction {
    ToggleWindow,
    SequentialPaste,
    RichPaste,
    FocusSearch,
    TogglePrivateMode,
    SnippetPicker,
}

#[derive(Clone)]
pub struct HotkeyUpdateHandle {
    sender: crossbeam_channel::Sender<HotkeyConfig>,
}

pub struct TrayHandle {
    stop: Option<Box<dyn FnOnce() + Send>>,
}

impl TrayHandle {
    pub fn new(stop: impl FnOnce() + Send + 'static) -> Self {
        Self {
            stop: Some(Box::new(stop)),
        }
    }

    pub fn stop(mut self) {
        if let Some(stop) = self.stop.take() {
            stop();
        }
    }
}

impl HotkeyUpdateHandle {
    pub fn new(sender: crossbeam_channel::Sender<HotkeyConfig>) -> Self {
        Self { sender }
    }

    pub fn update(&self, config: HotkeyConfig) -> Result<(), String> {
        self.sender
            .send(config)
            .map_err(|err| t!("platform.hotkey_update_failed", err => err).to_string())
    }
}

#[cfg(target_os = "linux")]
pub use linux::active_window_class;
#[cfg(target_os = "linux")]
pub use linux::current_keyboard_modifiers;
#[cfg(target_os = "linux")]
pub use linux::validate_hotkey;
#[cfg(target_os = "linux")]
pub use linux::{
    active_app_name, discover_apps_for_mime, platform_note, simulate_paste, start_hotkey_listener,
};
#[cfg(target_os = "linux")]
pub use linux::{autostart_enabled, set_autostart};
#[cfg(target_os = "linux")]
pub use linux::{mouse_position, screen_geometry, start_tray};
#[cfg(target_os = "linux")]
pub use linux_xfixes::start_primary_watcher;
#[cfg(target_os = "windows")]
pub use windows::active_window_class;
#[cfg(target_os = "windows")]
pub use windows::current_keyboard_modifiers;
#[cfg(target_os = "windows")]
pub use windows::validate_hotkey;
#[cfg(target_os = "windows")]
pub use windows::{
    active_app_name, discover_apps_for_mime, platform_note, simulate_paste, start_hotkey_listener,
};
#[cfg(target_os = "windows")]
pub use windows::{autostart_enabled, set_autostart};
#[cfg(target_os = "windows")]
pub use windows::{mouse_position, screen_geometry, start_tray};

#[cfg(target_os = "windows")]
pub fn initialize_process() {
    windows::initialize_process();
}

#[cfg(not(target_os = "windows"))]
pub fn initialize_process() {}

#[cfg(target_os = "windows")]
pub fn system_locale_name() -> Option<String> {
    windows::system_locale_name()
}

#[cfg(not(target_os = "windows"))]
pub fn system_locale_name() -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
pub fn system_dark_mode() -> Option<bool> {
    windows::system_dark_mode()
}

#[cfg(not(target_os = "windows"))]
pub fn system_dark_mode() -> Option<bool> {
    None
}

#[cfg(target_os = "windows")]
pub fn remember_main_window(frame: &eframe::Frame) -> Result<(), String> {
    windows::remember_main_window(frame)
}

#[cfg(not(target_os = "windows"))]
pub fn remember_main_window(_frame: &eframe::Frame) -> Result<(), String> {
    Ok(())
}

pub fn open_file_manager(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        windows::open_file_manager(path)
    }
    #[cfg(not(target_os = "windows"))]
    {
        open::that(path).map_err(|err| t!("error.open_file_manager_failed", err = err).to_string())
    }
}

#[cfg(target_os = "windows")]
pub fn set_taskbar_visible(frame: &eframe::Frame, visible: bool) -> Result<(), String> {
    windows::set_taskbar_visible(frame, visible)
}

#[cfg(not(target_os = "windows"))]
pub fn set_taskbar_visible(_frame: &eframe::Frame, _visible: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn play_wav(wav: &[u8]) -> bool {
    windows::play_wav(wav)
}

#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub fn capabilities() -> PlatformCapabilities {
    linux::capabilities()
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
pub fn capabilities() -> PlatformCapabilities {
    windows::capabilities()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn active_app_name() -> String {
    "Unknown".to_string()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn platform_note() -> String {
    t!("platform.note.generic")
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
#[allow(dead_code)]
pub fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        active_window: t!("platform.capability.active_window_generic"),
        window_management: t!("platform.capability.window_mgmt_generic"),
        global_hotkey: t!("platform.capability.hotkey_generic"),
        tray: t!("platform.capability.tray_generic"),
        rich_clipboard: t!("platform.capability.rich_clipboard_generic"),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn start_hotkey_listener(
    _sender: crossbeam_channel::Sender<crate::clipboard::ClipboardEvent>,
    _ctx: egui::Context,
    _config: HotkeyConfig,
) -> HotkeyUpdateHandle {
    let (sender, _receiver) = crossbeam_channel::unbounded();
    HotkeyUpdateHandle::new(sender)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn current_keyboard_modifiers() -> KeyboardModifiers {
    KeyboardModifiers::default()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn validate_hotkey(_combo: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn autostart_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Err(t!("platform.autostart_not_supported"))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn simulate_paste(_prefer_formatted: bool, _method: PasteMethod) -> Result<(), String> {
    Err(t!("platform.paste_not_supported"))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn discover_apps_for_mime(_mime: &str) -> Vec<AppChoice> {
    Vec::new()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn start_tray(
    _sender: crossbeam_channel::Sender<crate::clipboard::ClipboardEvent>,
    _ctx: egui::Context,
    _enabled: bool,
    _private_mode: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Option<TrayHandle> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn screen_geometry() -> Option<ScreenGeometry> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn mouse_position() -> Option<(f32, f32)> {
    None
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn active_window_class() -> Option<String> {
    None
}

#[cfg(not(target_os = "linux"))]
pub fn start_primary_watcher(
    _sender: crossbeam_channel::Sender<crate::clipboard::ClipboardEvent>,
    _primary_enabled: std::sync::Arc<std::sync::atomic::AtomicBool>,
    _echo_guard: crate::clipboard::PrimaryEchoGuard,
) {
    // Primary selection is X11/Linux-only; no-op on other platforms.
}
