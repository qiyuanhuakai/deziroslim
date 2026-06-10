use crate::app::{ClipboardApp, HotkeyTarget, hotkey_lines};
use crate::ui::settings::{hotkey_record_row, hotkey_single_record_row, removable_hotkey_chip};
use crate::ui::widgets::macos_collapsible_group;
use eframe::egui;
use rust_i18n::t;

pub fn draw_shortcuts_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[1];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(
        ui,
        t!("settings.hotkey.title"),
        &mut expanded,
        &theme,
        |ui| {
            ui.label(egui::RichText::new(t!("settings.hotkey.hint")).color(app.theme.muted));
            let main_hotkeys = app.main_hotkeys.clone();
            let sequential_hotkey = app.sequential_hotkey.clone();
            let rich_paste_hotkey = app.rich_paste_hotkey.clone();
            let search_hotkey = app.search_hotkey.clone();
            hotkey_record_row(
                ui,
                t!("settings.hotkey.main_invoke"),
                &main_hotkeys,
                app.recording_hotkey == Some(HotkeyTarget::Main),
                |ui| {
                    if ui.button(t!("settings.hotkey.record_new")).clicked() {
                        app.recording_hotkey = Some(HotkeyTarget::Main);
                        app.status = t!("settings.hotkey.recording_main").to_string();
                    }
                    if ui.button(t!("settings.hotkey.clear_all")).clicked() {
                        app.main_hotkeys.clear();
                        app.update_hotkeys();
                        app.persist_preferences();
                    }
                },
            );
            let main_hotkey_items = hotkey_lines(&main_hotkeys);
            if !main_hotkey_items.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(t!("settings.hotkey.recorded")).color(app.theme.muted),
                    );
                    let mut remove_hotkey = None;
                    for hotkey in &main_hotkey_items {
                        if removable_hotkey_chip(ui, hotkey, &app.theme).clicked() {
                            remove_hotkey = Some(hotkey.clone());
                        }
                    }
                    if let Some(remove_hotkey) = remove_hotkey {
                        app.remove_main_hotkey(&remove_hotkey);
                    }
                });
            }
            hotkey_single_record_row(
                ui,
                t!("settings.hotkey.sequential_paste"),
                &sequential_hotkey,
                app.recording_hotkey == Some(HotkeyTarget::Sequential),
                || {
                    app.recording_hotkey = Some(HotkeyTarget::Sequential);
                    app.status = t!("settings.hotkey.recording_sequential").to_string();
                },
            );
            hotkey_single_record_row(
                ui,
                t!("settings.hotkey.rich_paste"),
                &rich_paste_hotkey,
                app.recording_hotkey == Some(HotkeyTarget::RichPaste),
                || {
                    app.recording_hotkey = Some(HotkeyTarget::RichPaste);
                    app.status = t!("settings.hotkey.recording_rich_paste").to_string();
                },
            );
            hotkey_single_record_row(
                ui,
                t!("settings.hotkey.search_focus"),
                &search_hotkey,
                app.recording_hotkey == Some(HotkeyTarget::Search),
                || {
                    app.recording_hotkey = Some(HotkeyTarget::Search);
                    app.status = t!("settings.hotkey.recording_search").to_string();
                },
            );
        },
    );
    if expanded != prev {
        app.settings_panel_collapsed[1] = !expanded;
        app.persist_preferences();
    }
}
