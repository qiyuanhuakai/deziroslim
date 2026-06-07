use crate::app::ClipboardApp;
use crate::sound::SoundEffect;
use crate::ui::settings::DropdownOption;
use crate::ui::settings::searchable_combo_row;
use crate::ui::widgets::{macos_collapsible_group, macos_range_slider, macos_toggle};
use eframe::egui;
use rust_i18n::t;

pub fn draw_general_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[0];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(
        ui,
        t!("settings.general.title"),
        &mut expanded,
        &theme,
        |ui| {
            let lang_options = [
                DropdownOption::borrowed("zh-CN", t!("settings.general.language_option_zh_cn")),
                DropdownOption::borrowed("en-US", t!("settings.general.language_option_en_us")),
                DropdownOption::borrowed(
                    "follow-system",
                    t!("settings.general.language_option_follow_system"),
                ),
            ];
            if searchable_combo_row(
                ui,
                t!("settings.general.language"),
                &mut app.language,
                &mut app.language_search,
                &lang_options,
                "",
                &app.theme,
            ) {
                let new_value = app.language.clone();
                if new_value == "follow-system" {
                    let detected = crate::i18n::detect_system_locale();
                    crate::i18n::set_app_locale(&detected);
                    app.language = "follow-system".to_string();
                } else {
                    crate::i18n::set_app_locale(&new_value);
                    app.language = new_value;
                }
                app.persist_preferences();
            }
            ui.label(
                egui::RichText::new(t!("settings.general.language_desc")).color(app.theme.muted),
            );
            ui.label(
                egui::RichText::new(t!("settings.general.language_restart_notice"))
                    .color(app.theme.muted),
            );
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.emoji_entry"));
                    macos_toggle(ui, &mut app.emoji_panel_enabled, &app.theme)
                })
                .inner
                .changed()
            {
                if !app.emoji_panel_enabled && app.current_page == crate::app::AppPage::Emoji {
                    app.current_page = crate::app::AppPage::Clipboard;
                }
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.symbol_entry"));
                    macos_toggle(ui, &mut app.symbol_panel_enabled, &app.theme)
                })
                .inner
                .changed()
            {
                if !app.symbol_panel_enabled && app.current_page == crate::app::AppPage::Symbol {
                    app.current_page = crate::app::AppPage::Clipboard;
                }
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.autostart"));
                    macos_toggle(ui, &mut app.autostart_enabled, &app.theme)
                })
                .inner
                .changed()
            {
                match crate::platform::set_autostart(app.autostart_enabled) {
                    Ok(()) => {
                        app.status = if app.autostart_enabled {
                            t!("settings.general.autostart_enabled").to_string()
                        } else {
                            t!("settings.general.autostart_disabled").to_string()
                        };
                        app.persist_preferences();
                    }
                    Err(err) => {
                        app.autostart_enabled = !app.autostart_enabled;
                        app.status = format!("{}: {err}", t!("settings.general.autostart_failed"));
                    }
                }
            }
            ui.label(
                egui::RichText::new(t!("settings.general.autostart_hint")).color(app.theme.muted),
            );
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.tag_manager"));
                    macos_toggle(ui, &mut app.tag_manager_enabled, &app.theme)
                })
                .inner
                .changed()
            {
                if !app.tag_manager_enabled {
                    app.tag_filter = None;
                    app.new_tag_input.clear();
                    app.tag_editor.clear();
                    app.refresh_entries();
                }
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.always_show_search"));
                    macos_toggle(ui, &mut app.show_search_box, &app.theme)
                })
                .inner
                .changed()
            {
                app.search_box_revealed = app.show_search_box;
                app.persist_preferences();
            }
            ui.label(
                egui::RichText::new(t!("settings.general.always_show_search_hint"))
                    .color(app.theme.muted),
            );
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.compact_mode"));
                    macos_toggle(ui, &mut app.compact_rows, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            ui.label(
                egui::RichText::new(t!("settings.general.compact_mode_hint"))
                    .color(app.theme.muted),
            );
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.arrow_key_selection"));
                    macos_toggle(ui, &mut app.arrow_key_selection, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.hide_tray_icon"));
                    macos_toggle(ui, &mut app.hide_tray_icon, &app.theme)
                })
                .inner
                .changed()
            {
                app.apply_tray_visibility(ctx);
                app.persist_preferences();
            }
            let can_close_to_tray = !app.hide_tray_icon && app.tray_handle.is_some();
            if ui
                .add_enabled_ui(can_close_to_tray, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(t!("settings.general.close_to_tray"));
                        macos_toggle(ui, &mut app.close_to_tray, &app.theme)
                    })
                    .inner
                })
                .inner
                .changed()
            {
                app.persist_preferences();
            }
            if ui
                .horizontal(|ui| {
                    ui.label(t!("settings.general.sound"));
                    macos_toggle(ui, &mut app.sound_enabled, &app.theme)
                })
                .inner
                .changed()
            {
                app.persist_preferences();
                if app.sound_enabled {
                    app.play_sound(SoundEffect::Copy);
                }
            }
            if app.sound_enabled {
                let mut volume = app.sound_volume as f32;
                if ui
                    .horizontal(|ui| {
                        ui.label(t!("settings.general.sound_volume"));
                        let changed =
                            macos_range_slider(ui, &mut volume, 0.0..=100.0, &app.theme).changed();
                        ui.label(
                            egui::RichText::new(format!("{}%", volume.round() as u8))
                                .color(app.theme.muted),
                        );
                        changed
                    })
                    .inner
                {
                    app.sound_volume = volume.round().clamp(0.0, 100.0) as u8;
                    app.persist_preferences();
                }
                if ui
                    .horizontal(|ui| {
                        ui.label(t!("settings.general.paste_sound"));
                        macos_toggle(ui, &mut app.paste_sound_enabled, &app.theme)
                    })
                    .inner
                    .changed()
                {
                    app.persist_preferences();
                }
            }
        },
    );
    if expanded == prev {
        app.settings_panel_collapsed[0] = !expanded;
        app.persist_preferences();
    }
}
