use crate::app::ClipboardApp;
use crate::ui::settings::{DropdownOption, searchable_combo_row};
use crate::ui::widgets::{macos_collapsible_group, macos_toggle};
use eframe::egui;
use rust_i18n::t;

pub fn draw_clipboard_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[2];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(
        ui,
        t!("settings.clipboard.title"),
        &mut expanded,
        &theme,
        |ui| {
            ui.add_enabled_ui(false, |ui| {
                ui.horizontal(|ui| {
                    ui.label(t!("settings.clipboard.persistent"));
                    macos_toggle(ui, &mut app.persistent, &app.theme);
                });
            });
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.clipboard.deduplicate"));
                    macos_toggle(ui, &mut app.deduplicate, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.clipboard.capture_files"));
                    macos_toggle(ui, &mut app.capture_files, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.clipboard.capture_rich_text"));
                    macos_toggle(ui, &mut app.capture_rich_text, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.clipboard.delete_after_paste"));
                    macos_toggle(ui, &mut app.delete_after_paste, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.clipboard.move_to_top_after_paste"));
                    macos_toggle(ui, &mut app.move_to_top_after_paste, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            let paste_options = [
                DropdownOption::borrowed(
                    "shift_insert",
                    t!("settings.clipboard.paste_method_shift_insert"),
                ),
                DropdownOption::borrowed("ctrl_v", "Ctrl+V"),
                DropdownOption::borrowed(
                    "type_text",
                    t!("settings.clipboard.paste_method_type_text"),
                ),
            ];
            if searchable_combo_row(
                ui,
                t!("settings.clipboard.paste_method"),
                &mut app.paste_method,
                &mut app.paste_method_search,
                &paste_options,
                t!("settings.clipboard.paste_method_search_hint"),
                &app.theme,
            ) {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.clipboard.privacy_protection"));
                    macos_toggle(ui, &mut app.privacy_protection, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            ui.label(
                egui::RichText::new(t!("settings.clipboard.clipboard_status_hint"))
                    .color(app.theme.muted),
            );
        },
    );
    let collapsed = !expanded;
    if collapsed != prev {
        app.settings_panel_collapsed[2] = collapsed;
        app.persist_preferences();
    }
}
