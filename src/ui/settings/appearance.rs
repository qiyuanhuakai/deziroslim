use crate::app::{
    AUTO_FALLBACK_FONT_LABEL, AUTO_PRIMARY_FONT_LABEL, ClipboardApp, DockMode, configure_fonts,
    discover_system_font_names, filter_chip,
};
use crate::ui::settings::font_combo_row;
use crate::ui::widgets::{macos_collapsible_group, macos_range_slider, macos_toggle};
use eframe::egui;
use rust_i18n::t;

pub fn draw_appearance_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[3];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(
        ui,
        t!("settings.appearance.title"),
        &mut expanded,
        &theme,
        |ui| {
            ui.label(t!("settings.appearance.theme_mode"));
            ui.horizontal(|ui| {
                let modes = [
                    (t!("settings.appearance.theme_follow_system"), "system"),
                    (t!("settings.appearance.theme_light"), "light"),
                    (t!("settings.appearance.theme_dark"), "dark"),
                ];
                for (label, value) in modes {
                    if filter_chip(ui, label.as_ref(), app.color_mode == value, &app.theme)
                        .clicked()
                    {
                        app.color_mode = value.to_string();
                        app.theme = crate::app::resolve_theme(&app.color_mode);
                        app.configure_style(ctx);
                        app.persist_preferences();
                    }
                }
            });
            ui.add_space(4.0);
            ui.label(t!("settings.appearance.font"));
            let mut font_changed = false;
            font_changed |= font_combo_row(
                ui,
                t!("settings.appearance.primary_font"),
                &mut app.primary_font,
                &mut app.primary_font_search,
                &app.font_choices,
                AUTO_PRIMARY_FONT_LABEL,
                t!("settings.appearance.primary_font_search"),
                &app.theme,
            );
            font_changed |= font_combo_row(
                ui,
                t!("settings.appearance.fallback_font"),
                &mut app.fallback_font,
                &mut app.fallback_font_search,
                &app.font_choices,
                AUTO_FALLBACK_FONT_LABEL,
                t!("settings.appearance.fallback_font_search"),
                &app.theme,
            );
            ui.vertical(|ui| {
                if ui.button(t!("settings.appearance.rescan_fonts")).clicked() {
                    app.font_choices = discover_system_font_names();
                    app.status = format!(
                        "{}: {}",
                        t!("settings.appearance.rescan_fonts_done"),
                        app.font_choices.len()
                    );
                }
                ui.label(
                    egui::RichText::new(t!("settings.appearance.fallback_font_hint"))
                        .color(app.theme.muted),
                );
            });
            if font_changed {
                configure_fonts(ctx, &app.font_selection());
                app.persist_preferences();
                if let Some(message) = app.font_load_warning() {
                    app.status = message;
                }
            }
            ui.add_space(4.0);
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.appearance.show_sensitive"));
                    macos_toggle(ui, &mut app.show_sensitive, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.appearance.show_detail_panel"));
                    macos_toggle(ui, &mut app.show_detail_panel, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.appearance.show_app_border"));
                    macos_toggle(ui, &mut app.show_app_border, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.appearance.window_pin"));
                    macos_toggle(ui, &mut app.window_pinned, &app.theme)
                })
                .inner
                .changed()
            {
                app.apply_window_level(ctx);
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.appearance.follow_mouse"));
                    macos_toggle(ui, &mut app.follow_mouse, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            let mut edge_docking_enabled = app.edge_docking != DockMode::Off;
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.appearance.edge_docking"));
                    macos_toggle(ui, &mut edge_docking_enabled, &app.theme)
                })
                .inner
                .changed()
            {
                app.edge_docking = if edge_docking_enabled {
                    DockMode::Right
                } else {
                    DockMode::Off
                };
                if app.edge_docking == DockMode::Off && app.edge_hidden {
                    app.reveal_edge_hidden(ctx, false);
                }
                app.persist_preferences();
            }
            ui.label(
                egui::RichText::new(t!("settings.appearance.edge_docking_hint"))
                    .color(app.theme.muted),
            );
            ui.add_space(4.0);
            ui.label(t!("settings.appearance.surface_opacity"));
            let mut opacity_f32 = app.surface_opacity as f32;
            if macos_range_slider(ui, &mut opacity_f32, 0.0..=100.0, &app.theme).changed() {
                app.surface_opacity = opacity_f32 as u8;
                app.configure_style(ctx);
                app.persist_preferences();
            }
            ui.label(t!("settings.appearance.interaction_hint"));
        },
    );
    let collapsed = !expanded;
    if collapsed != prev {
        app.settings_panel_collapsed[3] = collapsed;
        app.persist_preferences();
    }
}
