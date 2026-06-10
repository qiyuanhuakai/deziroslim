use crate::app::ClipboardApp;
use crate::ui::settings::app_combo_row;
use crate::ui::widgets::macos_collapsible_group;
use eframe::egui;
use rust_i18n::t;

pub fn draw_default_apps_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[4];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(
        ui,
        t!("settings.default_app.title"),
        &mut expanded,
        &theme,
        |ui| {
            ui.label(egui::RichText::new(t!("settings.default_app.hint")).color(app.theme.muted));
            let mut changed = false;
            changed |= app_combo_row(
                ui,
                "TEXT",
                &mut app.default_text_app,
                &mut app.text_app_search,
                &app.text_app_choices,
                &app.theme,
            );
            changed |= app_combo_row(
                ui,
                "URL",
                &mut app.default_url_app,
                &mut app.url_app_search,
                &app.url_app_choices,
                &app.theme,
            );
            changed |= app_combo_row(
                ui,
                "CODE",
                &mut app.default_code_app,
                &mut app.code_app_search,
                &app.code_app_choices,
                &app.theme,
            );
            changed |= app_combo_row(
                ui,
                "FILE",
                &mut app.default_file_app,
                &mut app.file_app_search,
                &app.file_app_choices,
                &app.theme,
            );
            changed |= app_combo_row(
                ui,
                "IMAGE",
                &mut app.default_image_app,
                &mut app.image_app_search,
                &app.image_app_choices,
                &app.theme,
            );
            changed |= app_combo_row(
                ui,
                "VIDEO",
                &mut app.default_video_app,
                &mut app.video_app_search,
                &app.video_app_choices,
                &app.theme,
            );
            if ui.button(t!("settings.default_app.rescan")).clicked() {
                app.text_app_choices = crate::platform::discover_apps_for_mime("text/plain");
                app.url_app_choices =
                    crate::platform::discover_apps_for_mime("x-scheme-handler/http");
                app.code_app_choices = crate::platform::discover_apps_for_mime("text/plain");
                app.file_app_choices =
                    crate::platform::discover_apps_for_mime("application/octet-stream");
                app.image_app_choices = crate::platform::discover_apps_for_mime("image/png");
                app.video_app_choices = crate::platform::discover_apps_for_mime("video/mp4");
                app.status = t!("settings.default_app.rescan_done").to_string();
            }
            if changed {
                app.persist_preferences();
            }
        },
    );
    if expanded != prev {
        app.settings_panel_collapsed[4] = !expanded;
        app.persist_preferences();
    }
}
