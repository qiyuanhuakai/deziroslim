use crate::app::ClipboardApp;
use crate::storage::Storage;
use crate::ui::settings::{
    pick_database_save_dir_with_dialog, settings_danger_button, settings_footer_button,
};
use crate::ui::widgets::macos_collapsible_group;
use eframe::egui;
use rust_i18n::t;
use std::path::PathBuf;

pub fn draw_data_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[6];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(ui, t!("settings.data.title"), &mut expanded, &theme, |ui| {
        ui.label(t!("settings.data.current_database"));
        egui::Frame::none()
            .fill(app.theme.data_bg)
            .stroke(egui::Stroke::new(1.0_f32, app.theme.data_border))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(&app.current_database_path)
                        .monospace()
                        .color(app.theme.fg),
                );
            });
        ui.add_space(6.0);
        ui.label(t!("settings.data.restart_save_path"));
        egui::Frame::none()
            .fill(app.theme.glass_bg)
            .stroke(egui::Stroke::new(1.0_f32, app.theme.glass_border))
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(&app.database_path_input)
                        .monospace()
                        .color(app.theme.muted),
                );
            });
        ui.horizontal(|ui| {
            if settings_footer_button(ui, t!("settings.data.select_save_path"), &app.theme, 0.0)
                .clicked()
            {
                let current = PathBuf::from(app.database_path_input.trim());
                match pick_database_save_dir_with_dialog(&current) {
                    Ok(Some(dir)) => {
                        let path = dir.join("clipboard.db");
                        match Storage::write_redirect_path(path.clone()) {
                            Ok(()) => {
                                app.database_path_input = path.display().to_string();
                                app.status = t!("settings.data.save_path_updated").to_string();
                            }
                            Err(err) => {
                                app.status =
                                    t!("settings.data.save_path_failed", err => err).to_string()
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(err) => app.status = err,
                }
            }
            if settings_footer_button(ui, t!("settings.data.restore_default"), &app.theme, 0.0)
                .clicked()
            {
                let path = Storage::default_path();
                match Storage::write_redirect_path(path.clone()) {
                    Ok(()) => {
                        app.database_path_input = path.display().to_string();
                        app.status = t!("settings.data.restore_default_done").to_string();
                    }
                    Err(err) => {
                        app.status =
                            t!("settings.data.restore_default_failed", err => err).to_string()
                    }
                }
            }
        });
        ui.label(egui::RichText::new(t!("settings.data.db_hint")).color(app.theme.muted));
        if settings_danger_button(ui, t!("history.clear_unpinned_history"), &app.theme, 0.0)
            .clicked()
        {
            match app.storage.clear_unpinned() {
                Ok(()) => {
                    app.status = t!("history.cleared_unpinned").to_string();
                    app.refresh_entries();
                }
                Err(err) => app.status = format!("{}: {err}", t!("history.clear_failed")),
            }
        }
    });
    let collapsed = !expanded;
    if collapsed != prev {
        app.settings_panel_collapsed[6] = collapsed;
        app.persist_preferences();
    }
}
