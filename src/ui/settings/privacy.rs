use crate::app::ClipboardApp;
use crate::blacklist::ExclusionMode;
use crate::ui::settings::apply_settings_widget_rounding;
use crate::ui::widgets::{macos_collapsible_group, macos_toggle};
use eframe::egui;
use rust_i18n::t;

pub fn draw_privacy_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[7];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(
        ui,
        t!("settings.private_mode.title"),
        &mut expanded,
        &theme,
        |ui| {
            apply_settings_widget_rounding(ui, theme.radius_input);

            // #3 Whitelist mode toggle (v1.1 deferred stub)
            let mut whitelist_enabled = app.exclusion_mode == ExclusionMode::Whitelist;
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.exclusion_list.whitelist_mode"));
                    macos_toggle(ui, &mut whitelist_enabled, &app.theme)
                })
                .inner
                .changed()
            {
                app.exclusion_mode = if whitelist_enabled {
                    ExclusionMode::Whitelist
                } else {
                    ExclusionMode::Blacklist
                };
                if app.exclusion_mode == ExclusionMode::Whitelist {
                    eprintln!("[tiez-slim] Whitelist mode is not yet implemented (v1.1 deferred)");
                }
                app.persist_preferences();
            }
            ui.label(
                egui::RichText::new(t!("settings.exclusion_list.whitelist_mode_notice"))
                    .color(theme.muted)
                    .size(12.0),
            );
        },
    );
}
