use super::PlatformCapabilities;
use crate::clipboard::ClipboardEvent;
use crate::platform::{
    AppChoice, HotkeyAction, HotkeyConfig, HotkeyUpdateHandle, KeyboardModifiers, PasteMethod,
    ScreenGeometry, TrayHandle,
};
use crossbeam_channel::Sender;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use rust_i18n::t;
use std::collections::HashMap;
use std::ffi::c_void;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::{
    CloseHandle, HINSTANCE, HMODULE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows::Win32::Globalization::GetUserDefaultLocaleName;
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::Media::Audio::{PlaySoundW, SND_MEMORY, SND_NODEFAULT, SND_SYNC, SND_SYSTEM};
use windows::Win32::System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_SZ,
    RRF_RT_REG_DWORD, RegCloseKey, RegDeleteValueW, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
    RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, HOT_KEY_MODIFIERS, INPUT, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN,
    RegisterHotKey, SendInput, UnregisterHotKey, VIRTUAL_KEY, VK_CONTROL, VK_INSERT, VK_LWIN,
    VK_MBUTTON, VK_MENU, VK_RWIN, VK_SHIFT, VK_V,
};
use windows::Win32::UI::Shell::{
    ASSOCF, ASSOCSTR_EXECUTABLE, ASSOCSTR_FRIENDLYAPPNAME, AssocQueryStringW, NIF_ICON,
    NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_SETVERSION, NOTIFYICON_VERSION_4,
    NOTIFYICONDATAW, SetCurrentProcessExplicitAppUserModelID, Shell_NotifyIconW, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreatePopupMenu,
    CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GWL_EXSTYLE,
    GWLP_USERDATA, GetClassNameW, GetCursorPos, GetForegroundWindow, GetSystemMetrics,
    GetWindowLongPtrW, GetWindowThreadProcessId, HCURSOR, HICON, HMENU, HWND_MESSAGE, IDC_ARROW,
    IsIconic, IsWindowVisible, LoadCursorW, LoadIconW, MF_SEPARATOR, MF_STRING, MSG, PM_REMOVE,
    PeekMessageW, PostQuitMessage, RegisterClassExW, SM_CXSCREEN, SM_CYSCREEN, SW_RESTORE, SW_SHOW,
    SW_SHOWNORMAL, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, TPM_LEFTALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WM_CREATE, WM_DESTROY,
    WM_HOTKEY, WM_LBUTTONUP, WM_QUIT, WM_RBUTTONUP, WM_USER, WNDCLASSEXW, WS_EX_APPWINDOW,
    WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, PWSTR, w};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Custom message sent to the hotkey window when the config should be refreshed.
const WM_HOTKEY_UPDATE: u32 = WM_USER + 1;
/// Custom message sent by the tray icon when the user interacts with it.
const WM_TRAY_CALLBACK: u32 = WM_USER + 2;
/// Tray icon unique identifier.
const TRAY_ICON_ID: u32 = 1;
/// Menu item identifiers for the tray popup menu.
const MENU_ITEM_SHOW_HIDE: usize = 1;
const MENU_ITEM_SETTINGS: usize = 2;
const MENU_ITEM_QUIT: usize = 3;
static MAIN_WINDOW_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Encode a Rust `&str` as a null-terminated UTF-16 `Vec<u16>`.
fn wrap_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Parse a human-readable key name into a Windows virtual-key code.
fn key_to_vk(key: &str) -> Option<u32> {
    let normalized = key.trim();
    if normalized.chars().count() == 1 {
        let ch = normalized.chars().next()?.to_ascii_uppercase();
        if ch.is_ascii_alphanumeric() {
            return Some(ch as u32);
        }
    }
    match normalized.to_ascii_lowercase().as_str() {
        "space" => Some(0x20),
        "tab" => Some(0x09),
        "backspace" => Some(0x08),
        "enter" | "return" => Some(0x0D),
        "escape" | "esc" => Some(0x1B),
        "insert" => Some(0x2D),
        "delete" => Some(0x2E),
        "up" | "arrowup" => Some(0x26),
        "down" | "arrowdown" => Some(0x28),
        "left" | "arrowleft" => Some(0x25),
        "right" | "arrowright" => Some(0x27),
        "home" => Some(0x24),
        "end" => Some(0x23),
        "pageup" => Some(0x21),
        "pagedown" => Some(0x22),
        "plus" => Some(0xBB),
        "minus" => Some(0xBD),
        "comma" => Some(0xBC),
        "period" => Some(0xBE),
        "slash" => Some(0xBF),
        "semicolon" => Some(0xBA),
        "quote" => Some(0xDE),
        "backslash" => Some(0xDC),
        "bracketleft" => Some(0xDB),
        "bracketright" => Some(0xDD),
        "backquote" => Some(0xC0),
        "f1" => Some(0x70),
        "f2" => Some(0x71),
        "f3" => Some(0x72),
        "f4" => Some(0x73),
        "f5" => Some(0x74),
        "f6" => Some(0x75),
        "f7" => Some(0x76),
        "f8" => Some(0x77),
        "f9" => Some(0x78),
        "f10" => Some(0x79),
        "f11" => Some(0x7A),
        "f12" => Some(0x7B),
        "f13" => Some(0x7C),
        "f14" => Some(0x7D),
        "f15" => Some(0x7E),
        "f16" => Some(0x7F),
        "f17" => Some(0x80),
        "f18" => Some(0x81),
        "f19" => Some(0x82),
        "f20" => Some(0x83),
        "f21" => Some(0x84),
        "f22" => Some(0x85),
        "f23" => Some(0x86),
        "f24" => Some(0x87),
        _ => None,
    }
}

/// Parse a combo string like "Ctrl+Shift+V" into `(modifiers, vk)`.
fn parse_hotkey_combo(combo: &str) -> Option<(u32, u32)> {
    let mut mods: u32 = 0;
    let mut key: Option<u32> = None;
    for part in combo.split('+').map(str::trim).filter(|p| !p.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL.0,
            "alt" | "option" => mods |= MOD_ALT.0,
            "shift" => mods |= MOD_SHIFT.0,
            "super" | "win" | "meta" | "cmd" => mods |= MOD_WIN.0,
            _ => {
                key = Some(key_to_vk(part)?);
            }
        }
    }
    mods |= MOD_NOREPEAT.0;
    key.map(|vk| (mods, vk))
}

fn is_middle_mouse_combo(combo: &str) -> bool {
    combo.trim().eq_ignore_ascii_case("MouseMiddle")
}

fn load_system_arrow_cursor() -> Result<HCURSOR, String> {
    // SAFETY: `IDC_ARROW` is a predefined system cursor identifier and a null
    // module handle instructs Windows to load it from the system resources.
    unsafe { LoadCursorW(HINSTANCE::default(), IDC_ARROW) }.map_err(|err| err.to_string())
}

fn tray_notification_code(lparam: LPARAM) -> u32 {
    (lparam.0 as u32) & 0xffff
}

fn load_application_icon() -> Result<HICON, String> {
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|err| err.to_string())?;
    let resource_id = PCWSTR(std::ptr::without_provenance(1));
    // SAFETY: Win32 resource APIs interpret pointer values in the range
    // 1..=0xffff as integer resource identifiers and do not dereference them.
    unsafe { LoadIconW(HINSTANCE(module.0), resource_id) }.map_err(|err| err.to_string())
}

fn wake_main_window_if_hidden() {
    let raw = MAIN_WINDOW_HANDLE.load(Ordering::Acquire);
    if raw.is_null() {
        return;
    }
    let hwnd = HWND(raw);
    // SAFETY: the handle is captured from eframe's live root viewport and
    // Windows window handles are valid across threads within the process.
    let hidden = unsafe { !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() };
    if hidden {
        // SAFETY: `hwnd` identifies the live eframe root window; these calls
        // only request a visibility/state transition and retain no pointers.
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// Map a `HotkeyAction` to the corresponding `ClipboardEvent`.
fn send_hotkey_action(sender: &Sender<ClipboardEvent>, action: HotkeyAction) {
    if matches!(
        action,
        HotkeyAction::ToggleWindow | HotkeyAction::FocusSearch | HotkeyAction::SnippetPicker
    ) {
        wake_main_window_if_hidden();
    }
    let event = match action {
        HotkeyAction::ToggleWindow => ClipboardEvent::ToggleWindow,
        HotkeyAction::SequentialPaste => ClipboardEvent::SequentialPaste,
        HotkeyAction::RichPaste => ClipboardEvent::PasteLatestRich,
        HotkeyAction::FocusSearch => ClipboardEvent::FocusSearch,
        HotkeyAction::TogglePrivateMode => ClipboardEvent::TogglePrivateMode,
        HotkeyAction::SnippetPicker => ClipboardEvent::SnippetPicker,
    };
    let _ = sender.send(event);
}

/// Read the default value of a registry sub-key.
fn read_registry_default_value(hkey: HKEY, subkey: &str) -> Option<String> {
    read_registry_value(hkey, subkey, None)
}

fn read_registry_value(hkey: HKEY, subkey: &str, value_name: Option<&str>) -> Option<String> {
    let mut h_key: HKEY = HKEY::default();
    let wide = wrap_wide(subkey);
    let result = unsafe { RegOpenKeyExW(hkey, PCWSTR(wide.as_ptr()), 0, KEY_READ, &mut h_key) };
    if result.is_err() {
        return None;
    }
    let value_name_wide = value_name.map(wrap_wide);
    let value_name = value_name_wide
        .as_ref()
        .map_or(PCWSTR::null(), |wide| PCWSTR(wide.as_ptr()));
    let mut size: u32 = 0;
    let result = unsafe { RegQueryValueExW(h_key, value_name, None, None, None, Some(&mut size)) };
    if result.is_err() || size == 0 {
        unsafe {
            let _ = RegCloseKey(h_key);
        }
        return None;
    }
    let mut buf = vec![0u16; (size as usize).div_ceil(2)];
    let result = unsafe {
        RegQueryValueExW(
            h_key,
            value_name,
            None,
            None,
            Some(buf.as_mut_ptr().cast::<u8>()),
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(h_key);
    }
    if result.is_err() {
        return None;
    }
    let len = buf.iter().position(|ch| *ch == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

fn enum_registry_subkeys(hkey: HKEY, subkey: &str) -> Vec<String> {
    let mut h_key: HKEY = HKEY::default();
    let wide = wrap_wide(subkey);
    let result = unsafe { RegOpenKeyExW(hkey, PCWSTR(wide.as_ptr()), 0, KEY_READ, &mut h_key) };
    if result.is_err() {
        return Vec::new();
    }

    let mut names = Vec::new();
    let mut index = 0;
    loop {
        let mut name = [0u16; 260];
        let mut len = name.len() as u32;
        let result = unsafe {
            RegEnumKeyExW(
                h_key,
                index,
                PWSTR(name.as_mut_ptr()),
                &mut len,
                None,
                PWSTR::null(),
                None,
                None,
            )
        };
        if result.is_err() {
            break;
        }
        names.push(String::from_utf16_lossy(&name[..len as usize]));
        index += 1;
    }
    unsafe {
        let _ = RegCloseKey(h_key);
    }
    names
}

/// Parse a shell command string like `"C:\path\app.exe" "%1"` into the executable path.
fn parse_command_to_exe(command: &str) -> String {
    let trimmed = command.trim();
    if let Some(rest) = trimmed.strip_prefix('"')
        && let Some(end) = rest.find('"')
    {
        return rest[..end].to_string();
    }
    if let Some(space) = trimmed.find(' ') {
        trimmed[..space].to_string()
    } else {
        trimmed.to_string()
    }
}

fn assoc_query_string(assoc: &str, value: windows::Win32::UI::Shell::ASSOCSTR) -> Option<String> {
    let assoc = wrap_wide(assoc);
    let mut len = 0;
    let result = unsafe {
        AssocQueryStringW(
            ASSOCF(0),
            value,
            PCWSTR(assoc.as_ptr()),
            PCWSTR::null(),
            PWSTR::null(),
            &mut len,
        )
    };
    if result.is_err() || len == 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize];
    let result = unsafe {
        AssocQueryStringW(
            ASSOCF(0),
            value,
            PCWSTR(assoc.as_ptr()),
            PCWSTR::null(),
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    if result.is_err() {
        return None;
    }
    let len = buf.iter().position(|ch| *ch == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..len]))
}

fn app_name_from_command(command: &str) -> String {
    let exe = parse_command_to_exe(command);
    Path::new(&exe)
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(exe.trim())
        .to_string()
}

fn push_app_choice(choices: &mut Vec<AppChoice>, name: String, command: String, is_default: bool) {
    let command = parse_command_to_exe(&command);
    if command.trim().is_empty() {
        return;
    }
    let duplicate = choices
        .iter()
        .any(|choice| choice.command.eq_ignore_ascii_case(&command));
    if duplicate {
        return;
    }
    choices.push(AppChoice {
        name,
        command,
        is_default,
    });
}

fn default_app_choice_for_ext(ext: &str) -> Option<AppChoice> {
    let command = assoc_query_string(ext, ASSOCSTR_EXECUTABLE).or_else(|| {
        read_registry_default_value(HKEY_CLASSES_ROOT, ext).and_then(|prog_id| {
            read_registry_default_value(
                HKEY_CLASSES_ROOT,
                &format!(r"{}\shell\open\command", prog_id),
            )
        })
    })?;
    let name = assoc_query_string(ext, ASSOCSTR_FRIENDLYAPPNAME)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| app_name_from_command(&command));
    Some(AppChoice {
        name: format!("{}: {}", t!("settings.default_app.system_default"), name),
        command: parse_command_to_exe(&command),
        is_default: true,
    })
}

fn app_path_choices() -> Vec<AppChoice> {
    const APP_PATHS: &str = r"Software\Microsoft\Windows\CurrentVersion\App Paths";
    let mut choices = Vec::new();
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for app_key in enum_registry_subkeys(root, APP_PATHS) {
            let subkey = format!(r"{}\{}", APP_PATHS, app_key);
            if let Some(command) = read_registry_default_value(root, &subkey) {
                push_app_choice(
                    &mut choices,
                    app_name_from_command(&command),
                    command,
                    false,
                );
            }
        }
    }
    choices
}

fn shell_application_choices() -> Vec<AppChoice> {
    let mut choices = Vec::new();
    for app_key in enum_registry_subkeys(HKEY_CLASSES_ROOT, "Applications") {
        let command_key = format!(r"Applications\{}\shell\open\command", app_key);
        if let Some(command) = read_registry_default_value(HKEY_CLASSES_ROOT, &command_key) {
            push_app_choice(
                &mut choices,
                app_name_from_command(&command),
                command,
                false,
            );
        }
    }
    choices
}

fn push_common_windows_choices(mime: &str, choices: &mut Vec<AppChoice>) {
    match mime {
        "application/octet-stream" => {
            push_app_choice(
                choices,
                "Explorer".to_string(),
                "explorer.exe".to_string(),
                false,
            );
        }
        "text/plain" => {
            push_app_choice(
                choices,
                "Notepad".to_string(),
                "notepad.exe".to_string(),
                false,
            );
        }
        "x-scheme-handler/http" | "x-scheme-handler/https" => {
            push_app_choice(
                choices,
                "Microsoft Edge".to_string(),
                "msedge.exe".to_string(),
                false,
            );
        }
        "image/png" | "image/jpeg" => {
            push_app_choice(
                choices,
                "Paint".to_string(),
                "mspaint.exe".to_string(),
                false,
            );
        }
        "video/mp4" => {
            push_app_choice(
                choices,
                "Windows Media Player".to_string(),
                "wmplayer.exe".to_string(),
                false,
            );
        }
        _ => {}
    }
}

fn extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime {
        "text/plain" => Some(".txt"),
        "x-scheme-handler/http" | "x-scheme-handler/https" => Some("http"),
        "image/png" => Some(".png"),
        "image/jpeg" => Some(".jpg"),
        "video/mp4" => Some(".mp4"),
        "application/octet-stream" => Some("Directory"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Simple functions
// ---------------------------------------------------------------------------

pub fn active_app_name() -> String {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() || hwnd.0.is_null() {
            return "Windows".to_string();
        }
        let mut pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return "Windows".to_string();
        }
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return "Windows".to_string(),
        };
        let mut buffer: [u16; 260] = [0u16; 260];
        let mut size = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            PWSTR(buffer.as_mut_ptr()),
            &mut size,
        );
        let _ = CloseHandle(handle);
        if result.is_err() || size == 0 {
            return "Windows".to_string();
        }
        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Windows".to_string())
    }
}

pub fn active_window_class() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() || hwnd.0.is_null() {
        return None;
    }
    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if len == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..len as usize]))
}

pub fn platform_note() -> String {
    t!("platform.note.windows").to_string()
}

#[allow(dead_code)]
pub fn capabilities() -> PlatformCapabilities {
    PlatformCapabilities {
        active_window: t!("platform.capability.active_window_windows").to_string(),
        window_management: t!("platform.capability.window_mgmt_generic").to_string(),
        global_hotkey: t!("platform.capability.hotkey_windows").to_string(),
        tray: t!("platform.capability.tray_windows").to_string(),
        rich_clipboard: t!("platform.capability.rich_clipboard_windows").to_string(),
    }
}

pub fn current_keyboard_modifiers() -> KeyboardModifiers {
    KeyboardModifiers {
        ctrl: key_is_pressed(VK_CONTROL),
        shift: key_is_pressed(VK_SHIFT),
        alt: key_is_pressed(VK_MENU),
        super_key: key_is_pressed(VK_LWIN) || key_is_pressed(VK_RWIN),
    }
}

fn key_is_pressed(key: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(key.0 as i32) < 0 }
}

pub fn validate_hotkey(combo: &str) -> Result<(), String> {
    if is_middle_mouse_combo(combo) || parse_hotkey_combo(combo).is_some() {
        Ok(())
    } else {
        Err(t!("platform.windows_hotkey_parse_error", combo => combo).to_string())
    }
}

pub fn autostart_enabled() -> Result<bool, String> {
    let key_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let mut h_key: HKEY = HKEY::default();
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wrap_wide(key_path).as_ptr()),
            0,
            KEY_READ,
            &mut h_key,
        )
    };
    if result.is_err() {
        return Ok(false);
    }
    let mut size: u32 = 0;
    let result = unsafe {
        RegQueryValueExW(
            h_key,
            PCWSTR(wrap_wide("deziroslim").as_ptr()),
            None,
            None,
            None,
            Some(&mut size),
        )
    };
    unsafe {
        let _ = RegCloseKey(h_key);
    }
    Ok(result.is_ok())
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let key_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let mut h_key: HKEY = HKEY::default();
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wrap_wide(key_path).as_ptr()),
            0,
            KEY_WRITE,
            &mut h_key,
        )
    };
    if result.is_err() {
        return Err(t!("platform.windows_registry_error", err => "cannot open key").to_string());
    }
    if enabled {
        let mut path_buf = vec![0u16; 260];
        let len = unsafe { GetModuleFileNameW(None, &mut path_buf) as usize };
        if len == 0 {
            unsafe {
                let _ = RegCloseKey(h_key);
            }
            return Err(
                t!("platform.windows_registry_error", err => "cannot get exe path").to_string(),
            );
        }
        let exe_path = String::from_utf16_lossy(&path_buf[..len]);
        let value = format!("\"{}\"", exe_path);
        let wide_value = wrap_wide(&value);
        let result = unsafe {
            RegSetValueExW(
                h_key,
                PCWSTR(wrap_wide("deziroslim").as_ptr()),
                0,
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    wide_value.as_ptr() as *const u8,
                    (wide_value.len()) * 2,
                )),
            )
        };
        if result.is_err() {
            unsafe {
                let _ = RegCloseKey(h_key);
            }
            return Err(
                t!("platform.windows_registry_error", err => "cannot set value").to_string(),
            );
        }
    } else {
        let result = unsafe { RegDeleteValueW(h_key, PCWSTR(wrap_wide("deziroslim").as_ptr())) };
        if result.is_err() {
            unsafe {
                let _ = RegCloseKey(h_key);
            }
            return Err(
                t!("platform.windows_registry_error", err => "cannot delete value").to_string(),
            );
        }
    }
    unsafe {
        let _ = RegCloseKey(h_key);
    }
    Ok(())
}

pub fn mouse_position() -> Option<(f32, f32)> {
    unsafe {
        let mut pt = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut pt).is_ok() {
            Some((pt.x as f32, pt.y as f32))
        } else {
            None
        }
    }
}

pub fn screen_size() -> Option<(f32, f32)> {
    unsafe {
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w == 0 || h == 0 {
            None
        } else {
            Some((w as f32, h as f32))
        }
    }
}

pub fn screen_geometry() -> Option<ScreenGeometry> {
    let (w, h) = screen_size()?;
    Some(ScreenGeometry {
        x: 0.0,
        y: 0.0,
        width: w,
        height: h,
    })
}

pub fn initialize_process() {
    let app_id = wrap_wide("qiyuaner.deziroslim");
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(PCWSTR(app_id.as_ptr()));
    }
}

pub fn system_locale_name() -> Option<String> {
    let mut locale = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(&mut locale) };
    if len <= 1 {
        return None;
    }
    let len = (len as usize).saturating_sub(1);
    Some(String::from_utf16_lossy(&locale[..len]))
}

fn apps_use_light_theme_to_dark(value: u32) -> bool {
    value == 0
}

pub fn system_dark_mode() -> Option<bool> {
    let subkey = wrap_wide(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let value_name = wrap_wide("AppsUseLightTheme");
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: `value` points to a writable `u32`, `size` exactly describes
    // that allocation, and RRF_RT_REG_DWORD constrains the registry value type.
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value_name.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast::<c_void>()),
            Some(&mut size),
        )
    };
    result.is_ok().then(|| apps_use_light_theme_to_dark(value))
}

pub fn open_file_manager(path: &Path) -> Result<(), String> {
    let target = path.to_string_lossy();
    if target.trim().is_empty() {
        return Err(t!("error.open_file_manager_failed", err = "empty path").to_string());
    }
    let operation = wrap_wide("open");
    let explorer = wrap_wide("explorer.exe");
    let parameters = wrap_wide(&format!("\"{target}\""));
    let result = unsafe {
        ShellExecuteW(
            HWND::default(),
            PCWSTR(operation.as_ptr()),
            PCWSTR(explorer.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    if code <= 32 {
        Err(t!(
            "error.open_file_manager_failed",
            err = format!("ShellExecuteW returned {code}")
        )
        .to_string())
    } else {
        Ok(())
    }
}

pub fn set_taskbar_visible(frame: &eframe::Frame, visible: bool) -> Result<(), String> {
    let hwnd = frame_hwnd(frame)?;
    MAIN_WINDOW_HANDLE.store(hwnd.0, Ordering::Release);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let app_window = WS_EX_APPWINDOW.0 as isize;
        let tool_window = WS_EX_TOOLWINDOW.0 as isize;
        let next = if visible {
            (style | app_window) & !tool_window
        } else {
            (style | tool_window) & !app_window
        };
        if next == style {
            return Ok(());
        }
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        SetWindowPos(
            hwnd,
            HWND::default(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
        .map_err(|err| {
            t!(
                "error.window_op_failed",
                err = format!("SetWindowPos: {err}")
            )
            .to_string()
        })
    }
}

pub fn remember_main_window(frame: &eframe::Frame) -> Result<(), String> {
    let hwnd = frame_hwnd(frame)?;
    MAIN_WINDOW_HANDLE.store(hwnd.0, Ordering::Release);
    Ok(())
}

fn frame_hwnd(frame: &eframe::Frame) -> Result<HWND, String> {
    let handle = frame
        .window_handle()
        .map_err(|err| t!("error.window_op_failed", err = err.to_string()).to_string())?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut c_void)),
        _ => Err(t!("error.window_op_failed", err = "unsupported window handle").to_string()),
    }
}

pub fn play_wav(wav: &[u8]) -> bool {
    if wav.is_empty() {
        return false;
    }
    unsafe {
        PlaySoundW(
            PCWSTR(wav.as_ptr().cast::<u16>()),
            HMODULE::default(),
            SND_MEMORY | SND_SYNC | SND_NODEFAULT | SND_SYSTEM,
        )
        .as_bool()
    }
}

pub fn discover_apps_for_mime(mime: &str) -> Vec<AppChoice> {
    let mut choices = Vec::new();
    if let Some(ext) = extension_for_mime(mime)
        && let Some(default_choice) = default_app_choice_for_ext(ext)
    {
        push_app_choice(
            &mut choices,
            default_choice.name,
            default_choice.command,
            true,
        );
    }
    push_common_windows_choices(mime, &mut choices);
    for choice in app_path_choices()
        .into_iter()
        .chain(shell_application_choices())
    {
        push_app_choice(&mut choices, choice.name, choice.command, false);
        if choices.len() >= 200 {
            break;
        }
    }
    choices.sort_by_key(|choice| (!choice.is_default, choice.name.to_ascii_lowercase()));
    choices
}

// ---------------------------------------------------------------------------
// Paste simulation
// ---------------------------------------------------------------------------

pub fn simulate_paste(prefer_formatted: bool, method: PasteMethod) -> Result<(), String> {
    thread::Builder::new()
        .name("simulate-paste".to_string())
        .spawn(move || {
            let _ = run_simulate_paste(prefer_formatted, method);
        })
        .map_err(|err| t!("platform.paste_thread_failed", err => err).to_string())?;
    Ok(())
}

fn run_simulate_paste(prefer_formatted: bool, method: PasteMethod) -> Result<(), String> {
    release_modifiers();
    thread::sleep(Duration::from_millis(50));

    match method {
        PasteMethod::CtrlV => send_ctrl_v(),
        PasteMethod::ShiftInsert => send_shift_insert(),
        PasteMethod::TypeText => {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| t!("platform.paste_read_clipboard_failed", err => e).to_string())?;
            let text = clipboard
                .get_text()
                .map_err(|e| t!("platform.paste_read_text_failed", err => e).to_string())?;
            send_unicode_text(&text)
        }
        PasteMethod::Auto => {
            if prefer_formatted {
                send_ctrl_v()
            } else {
                send_shift_insert()
            }
        }
    }
}

fn send_ctrl_v() -> Result<(), String> {
    send_keybd_input(VK_CONTROL, false)?;
    send_keybd_input(VK_V, false)?;
    send_keybd_input(VK_V, true)?;
    send_keybd_input(VK_CONTROL, true)?;
    Ok(())
}

fn send_shift_insert() -> Result<(), String> {
    send_keybd_input(VK_SHIFT, false)?;
    send_keybd_input(VK_INSERT, false)?;
    send_keybd_input(VK_INSERT, true)?;
    send_keybd_input(VK_SHIFT, true)?;
    Ok(())
}

fn send_keybd_input(vk: VIRTUAL_KEY, key_up: bool) -> Result<(), String> {
    let mut input = INPUT {
        r#type: INPUT_KEYBOARD,
        ..Default::default()
    };
    input.Anonymous.ki = KEYBDINPUT {
        wVk: vk,
        wScan: 0,
        dwFlags: if key_up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        },
        time: 0,
        dwExtraInfo: 0,
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
    Ok(())
}

fn send_unicode_text(text: &str) -> Result<(), String> {
    for ch in text.chars() {
        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            ..Default::default()
        };
        input.Anonymous.ki = KEYBDINPUT {
            wVk: VIRTUAL_KEY(0),
            wScan: ch as u16,
            dwFlags: KEYEVENTF_UNICODE,
            time: 0,
            dwExtraInfo: 0,
        };
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
        input.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
        unsafe {
            SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
        }
    }
    Ok(())
}

fn release_modifiers() {
    for vk in [VK_CONTROL, VK_SHIFT, VK_MENU] {
        send_keybd_input(vk, true).ok();
    }
}

// ---------------------------------------------------------------------------
// Hotkey listener
// ---------------------------------------------------------------------------

/// State shared between the hotkey window procedure and the message loop.
struct HotkeyState {
    sender: Sender<ClipboardEvent>,
    ctx: egui::Context,
    /// Map of hotkey id -> HotkeyAction.
    actions: HashMap<i32, HotkeyAction>,
    /// Next hotkey id to assign.
    next_id: i32,
    /// Receiver for config updates.
    updates: crossbeam_channel::Receiver<HotkeyConfig>,
    middle_mouse_action: Option<HotkeyAction>,
    middle_mouse_was_down: bool,
}

/// Register all hotkeys from the given config, populating `actions`.
fn register_hotkeys(hwnd: HWND, config: &HotkeyConfig, state: &mut HotkeyState) {
    // Unregister existing hotkeys.
    for id in state.actions.keys() {
        unsafe {
            let _ = UnregisterHotKey(hwnd, *id);
        }
    }
    state.actions.clear();
    state.middle_mouse_action = None;

    // Main hotkeys (toggle window) – supports multiple combos separated by newlines.
    for combo in config
        .main_hotkeys
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        if is_middle_mouse_combo(combo) {
            state.middle_mouse_action = Some(HotkeyAction::ToggleWindow);
            continue;
        }
        if let Some((mods, vk)) = parse_hotkey_combo(combo) {
            let id = state.next_id;
            state.next_id += 1;
            match unsafe { RegisterHotKey(hwnd, id, HOT_KEY_MODIFIERS(mods), vk) } {
                Ok(_) => {
                    state.actions.insert(id, HotkeyAction::ToggleWindow);
                }
                Err(_) => {
                    let _ = state.sender.send(ClipboardEvent::Status(
                        t!("platform.windows_hotkey_register_failed", combo => combo).to_string(),
                    ));
                    state.ctx.request_repaint();
                }
            }
        }
    }

    // Single hotkeys.
    let singles: [(&str, HotkeyAction); 5] = [
        (&config.sequential_hotkey, HotkeyAction::SequentialPaste),
        (&config.rich_paste_hotkey, HotkeyAction::RichPaste),
        (&config.search_hotkey, HotkeyAction::FocusSearch),
        (&config.private_mode_hotkey, HotkeyAction::TogglePrivateMode),
        (&config.snippet_picker_hotkey, HotkeyAction::SnippetPicker),
    ];
    for (combo, action) in singles {
        let combo = combo.trim();
        if combo.is_empty() {
            continue;
        }
        if is_middle_mouse_combo(combo) {
            if state.middle_mouse_action.is_none() {
                state.middle_mouse_action = Some(action);
            }
            continue;
        }
        if let Some((mods, vk)) = parse_hotkey_combo(combo) {
            let id = state.next_id;
            state.next_id += 1;
            match unsafe { RegisterHotKey(hwnd, id, HOT_KEY_MODIFIERS(mods), vk) } {
                Ok(_) => {
                    state.actions.insert(id, action);
                }
                Err(_) => {
                    let _ = state.sender.send(ClipboardEvent::Status(
                        t!("platform.windows_hotkey_register_failed", combo => combo).to_string(),
                    ));
                    state.ctx.request_repaint();
                }
            }
        }
    }
    state.middle_mouse_was_down = key_is_pressed(VK_MBUTTON);
}

fn registered_hotkey_count(state: &HotkeyState) -> usize {
    state.actions.len() + usize::from(state.middle_mouse_action.is_some())
}

fn poll_middle_mouse_hotkey(state: &mut HotkeyState) {
    let is_down = key_is_pressed(VK_MBUTTON);
    if is_down
        && !state.middle_mouse_was_down
        && let Some(action) = state.middle_mouse_action
    {
        send_hotkey_action(&state.sender, action);
        state.ctx.request_repaint();
    }
    state.middle_mouse_was_down = is_down;
}

fn apply_pending_hotkey_config(hwnd: HWND, state: &mut HotkeyState) {
    let mut latest = None;
    while let Ok(config) = state.updates.try_recv() {
        latest = Some(config);
    }
    if let Some(config) = latest {
        register_hotkeys(hwnd, &config, state);
        let _ = state.sender.send(ClipboardEvent::Status(
            t!("platform.hotkey_updated", keys => registered_hotkey_count(state)).to_string(),
        ));
        state.ctx.request_repaint();
    }
}

/// Window procedure for the hotkey listener window.
unsafe extern "system" fn hotkey_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
            let state_ptr = create.lpCreateParams as *mut HotkeyState;
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            }
            LRESULT(0)
        }
        WM_HOTKEY => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut HotkeyState;
            if let Some(state) = unsafe { state_ptr.as_ref() } {
                let id = wparam.0 as i32;
                if let Some(action) = state.actions.get(&id) {
                    send_hotkey_action(&state.sender, *action);
                    state.ctx.request_repaint();
                }
            }
            LRESULT(0)
        }
        WM_HOTKEY_UPDATE => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut HotkeyState;
            if let Some(state) = unsafe { state_ptr.as_mut() } {
                apply_pending_hotkey_config(hwnd, state);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut HotkeyState;
            if let Some(state) = unsafe { state_ptr.as_ref() } {
                for id in state.actions.keys() {
                    unsafe {
                        let _ = UnregisterHotKey(hwnd, *id);
                    }
                }
            }
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub fn start_hotkey_listener(
    sender: Sender<ClipboardEvent>,
    ctx: egui::Context,
    config: HotkeyConfig,
) -> HotkeyUpdateHandle {
    let (update_sender, update_receiver) = crossbeam_channel::unbounded();
    let sender_clone = sender.clone();
    thread::Builder::new()
        .name("win32-hotkey-listener".to_string())
        .spawn(move || {
            if let Err(err) = hotkey_thread(sender_clone.clone(), ctx, config, update_receiver) {
                // Thread-level fatal error – nothing more we can do.
                let _ = sender_clone.send(ClipboardEvent::Status(
                    t!("platform.hotkey_unavailable", err => err).to_string(),
                ));
            }
        })
        .expect("spawn win32 hotkey listener");
    HotkeyUpdateHandle::new(update_sender)
}

fn hotkey_thread(
    sender: Sender<ClipboardEvent>,
    ctx: egui::Context,
    config: HotkeyConfig,
    updates: crossbeam_channel::Receiver<HotkeyConfig>,
) -> Result<(), String> {
    let class_name = w!("DeziroslimHotkeyWindow");

    // Register window class.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|e| e.to_string())?;
    let h_instance = HINSTANCE(module.0);

    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(hotkey_wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance,
        hIcon: HICON::default(),
        hCursor: load_system_arrow_cursor()?,
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: class_name,
        hIconSm: HICON::default(),
    };
    unsafe {
        RegisterClassExW(&wnd_class);
    }

    // Create state.
    let mut state = HotkeyState {
        sender: sender.clone(),
        ctx: ctx.clone(),
        actions: HashMap::new(),
        next_id: 1,
        updates,
        middle_mouse_action: None,
        middle_mouse_was_down: false,
    };

    // Create message-only window.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!(""),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            HWND_MESSAGE,
            HMENU::default(),
            h_instance,
            Some(&mut state as *mut _ as *mut c_void),
        )
    }
    .map_err(|e| e.to_string())?;

    // Register initial hotkeys.
    register_hotkeys(hwnd, &config, &mut state);
    let _ = sender.send(ClipboardEvent::Status(
        t!("platform.hotkey_updated", keys => registered_hotkey_count(&state)).to_string(),
    ));
    ctx.request_repaint();

    // Message loop.
    let mut msg = MSG::default();
    loop {
        // Non-blocking peek so we can also check for updates.
        while unsafe { PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() } {
            if msg.message == WM_QUIT {
                return Ok(());
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        apply_pending_hotkey_config(hwnd, &mut state);
        poll_middle_mouse_hotkey(&mut state);
        thread::sleep(Duration::from_millis(10));
    }
}

// ---------------------------------------------------------------------------
// System tray
// ---------------------------------------------------------------------------

/// State shared between the tray window procedure and the message loop.
struct TrayState {
    sender: Sender<ClipboardEvent>,
    ctx: egui::Context,
    hwnd: HWND,
}

/// Window procedure for the tray icon window.
unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create = unsafe { &*(lparam.0 as *const CREATESTRUCTW) };
            let state_ptr = create.lpCreateParams as *mut TrayState;
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            }
            LRESULT(0)
        }
        WM_TRAY_CALLBACK => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut TrayState;
            if let Some(state) = unsafe { state_ptr.as_ref() } {
                match tray_notification_code(lparam) {
                    WM_LBUTTONUP => {
                        send_hotkey_action(&state.sender, HotkeyAction::ToggleWindow);
                        state.ctx.request_repaint();
                    }
                    WM_RBUTTONUP => {
                        let mut pt = POINT { x: 0, y: 0 };
                        let _ = unsafe { GetCursorPos(&mut pt) };
                        let hmenu = unsafe { CreatePopupMenu() }.unwrap_or_default();
                        let _ = unsafe { SetForegroundWindow(hwnd) };
                        let _ = unsafe {
                            AppendMenuW(
                                hmenu,
                                MF_STRING,
                                MENU_ITEM_SHOW_HIDE,
                                PCWSTR(wrap_wide(&t!("tray.menu.show_hide")).as_ptr()),
                            )
                        };
                        let _ = unsafe {
                            AppendMenuW(
                                hmenu,
                                MF_STRING,
                                MENU_ITEM_SETTINGS,
                                PCWSTR(wrap_wide(&t!("tray.menu.settings")).as_ptr()),
                            )
                        };
                        let _ = unsafe { AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null()) };
                        let _ = unsafe {
                            AppendMenuW(
                                hmenu,
                                MF_STRING,
                                MENU_ITEM_QUIT,
                                PCWSTR(wrap_wide(&t!("tray.menu.quit")).as_ptr()),
                            )
                        };
                        let cmd = unsafe {
                            TrackPopupMenu(
                                hmenu,
                                TPM_RETURNCMD | TPM_LEFTALIGN | TPM_RIGHTBUTTON,
                                pt.x,
                                pt.y,
                                0,
                                hwnd,
                                None,
                            )
                        };
                        match cmd.0 as usize {
                            MENU_ITEM_SHOW_HIDE => {
                                send_hotkey_action(&state.sender, HotkeyAction::ToggleWindow);
                                state.ctx.request_repaint();
                            }
                            MENU_ITEM_SETTINGS => {
                                wake_main_window_if_hidden();
                                let _ = state.sender.send(ClipboardEvent::OpenSettings);
                                state.ctx.request_repaint();
                            }
                            MENU_ITEM_QUIT => {
                                let _ = state.sender.send(ClipboardEvent::Quit);
                                state.ctx.request_repaint();
                            }
                            _ => {}
                        }
                        let _ = unsafe { DestroyMenu(hmenu) };
                    }
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let state_ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut TrayState;
            if let Some(state) = unsafe { state_ptr.as_ref() } {
                remove_tray_icon(state.hwnd);
            }
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn add_tray_icon(hwnd: HWND) -> Result<(), String> {
    let h_icon = load_application_icon()?;
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY_CALLBACK,
        hIcon: h_icon,
        ..Default::default()
    };

    // Set tooltip.
    let tip = t!("tray.menu.show_hide");
    let wide_tip = wrap_wide(&tip);
    for (i, ch) in wide_tip.iter().enumerate().take(128) {
        nid.szTip[i] = *ch;
    }

    let added = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
    if !added {
        return Err(t!(
            "error.system_tray_unavailable",
            err = "Shell_NotifyIconW NIM_ADD failed"
        )
        .to_string());
    }
    nid.Anonymous.uVersion = NOTIFYICON_VERSION_4;
    let _ = unsafe { Shell_NotifyIconW(NIM_SETVERSION, &nid) };
    Ok(())
}

fn remove_tray_icon(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

pub fn start_tray(
    sender: Sender<ClipboardEvent>,
    ctx: egui::Context,
    enabled: bool,
    _private_mode: Arc<AtomicBool>,
) -> Option<TrayHandle> {
    if !enabled {
        return None;
    }

    // Channel to signal shutdown.
    let (stop_sender, stop_receiver) = crossbeam_channel::bounded::<()>(1);

    thread::Builder::new()
        .name("win32-tray".to_string())
        .spawn(move || {
            if let Err(err) = tray_thread(sender.clone(), ctx, stop_receiver) {
                let _ = sender.send(ClipboardEvent::Status(
                    t!("error.system_tray_unavailable", err => err).to_string(),
                ));
            }
        })
        .expect("spawn win32 tray thread");

    Some(TrayHandle::new(move || {
        let _ = stop_sender.send(());
    }))
}

fn tray_thread(
    sender: Sender<ClipboardEvent>,
    ctx: egui::Context,
    stop_receiver: crossbeam_channel::Receiver<()>,
) -> Result<(), String> {
    let class_name = w!("DeziroslimTrayWindow");

    // Register window class.
    let module = unsafe { GetModuleHandleW(PCWSTR::null()) }.map_err(|e| e.to_string())?;
    let h_instance = HINSTANCE(module.0);

    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(tray_wndproc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance,
        hIcon: HICON::default(),
        hCursor: load_system_arrow_cursor()?,
        hbrBackground: HBRUSH::default(),
        lpszMenuName: PCWSTR::null(),
        lpszClassName: class_name,
        hIconSm: HICON::default(),
    };
    unsafe {
        RegisterClassExW(&wnd_class);
    }

    // Create state (hwnd will be filled in after window creation).
    let mut state = TrayState {
        sender: sender.clone(),
        ctx: ctx.clone(),
        hwnd: HWND::default(),
    };

    // Create hidden window.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            w!(""),
            WS_OVERLAPPED,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            HWND::default(),
            HMENU::default(),
            h_instance,
            Some(&mut state as *mut _ as *mut c_void),
        )
    }
    .map_err(|e| e.to_string())?;

    // Update state with the actual HWND.
    state.hwnd = hwnd;

    // Add tray icon.
    add_tray_icon(hwnd)?;

    // Message loop.
    let mut msg = MSG::default();
    loop {
        // Check for stop signal.
        if stop_receiver.try_recv().is_ok() {
            remove_tray_icon(hwnd);
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            return Ok(());
        }

        while unsafe { PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_REMOVE).as_bool() } {
            if msg.message == WM_QUIT {
                return Ok(());
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_arrow_cursor_loads_from_windows_resources() {
        assert!(load_system_arrow_cursor().is_ok());
    }

    #[test]
    fn mouse_middle_is_a_valid_windows_hotkey() {
        assert!(validate_hotkey("MouseMiddle").is_ok());
    }

    #[test]
    fn tray_notification_uses_low_word_with_version_four() {
        let packed = LPARAM(((TRAY_ICON_ID << 16) | WM_LBUTTONUP) as isize);
        assert_eq!(tray_notification_code(packed), WM_LBUTTONUP);
    }

    #[test]
    fn application_icon_is_embedded() {
        assert!(load_application_icon().is_ok());
    }

    #[test]
    fn windows_light_theme_value_maps_to_dark_mode() {
        assert!(!apps_use_light_theme_to_dark(1));
        assert!(apps_use_light_theme_to_dark(0));
    }
}
