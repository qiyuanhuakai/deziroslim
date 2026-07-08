use crate::app::ClipboardApp;
use crate::ui::settings::{hex_to_color32, settings_danger_button, settings_footer_button};
use crate::ui::widgets::{MacosButton, macos_collapsible_group};
use eframe::egui;
use rust_i18n::t;

pub fn draw_tags_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app.settings_panel_collapsed[5];
    let mut expanded = !prev;
    let theme = app.theme.clone();
    macos_collapsible_group(ui, t!("settings.tags.title"), &mut expanded, &theme, |ui| {
        if !app.tag_manager_enabled {
            ui.label(
                egui::RichText::new(t!("settings.tags.manager_closed_hint")).color(app.theme.muted),
            );
            return;
        }

        let available_width = (ui.available_width() - 12.0).max(220.0);
        let gap = ui.spacing().item_spacing.x;
        let sidebar_w = (available_width * 0.34).clamp(96.0, 156.0);
        let detail_w = (available_width - sidebar_w - gap * 2.0).max(88.0);

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(sidebar_w);
                let bg = app.theme.glass_bg;
                let accent = app.theme.accent;
                egui::Frame::none()
                    .fill(bg)
                    .rounding(egui::Rounding::same(8.0))
                    .stroke(egui::Stroke::new(1.0, app.theme.glass_border))
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ui.set_width((sidebar_w - 12.0).max(80.0));
                        let new_tag_button_width = ui.available_width().max(80.0);
                        if settings_footer_button(
                            ui,
                            t!("settings.tags.new_tag"),
                            &app.theme,
                            new_tag_button_width,
                        )
                        .clicked()
                        {
                            app.show_tag_input = !app.show_tag_input;
                        }

                        if app.show_tag_input {
                            ui.horizontal(|ui| {
                                let input_width = (ui.available_width() - 42.0).max(40.0);
                                let response = ui.add_sized(
                                    [input_width, 22.0],
                                    egui::TextEdit::singleline(&mut app.new_tag_input)
                                        .hint_text(t!("settings.tags.tag_name_hint"))
                                        .desired_width(input_width),
                                );
                                let enter = response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                if settings_footer_button(
                                    ui,
                                    t!("settings.tags.add_button"),
                                    &app.theme,
                                    0.0,
                                )
                                .clicked()
                                    || enter
                                {
                                    app.add_saved_tag_from_input();
                                    app.show_tag_input = false;
                                }
                            });
                            ui.add_space(2.0);
                        }

                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if app.saved_tags.is_empty() {
                                ui.label(
                                    egui::RichText::new(t!("settings.tags.no_tags"))
                                        .size(11.0)
                                        .color(app.theme.muted),
                                );
                            } else {
                                let tags = app.saved_tags.clone();
                                for tag in &tags {
                                    let selected = app.selected_saved_tag.as_deref() == Some(tag);
                                    let (bg, fg, stroke) = if selected {
                                        (
                                            accent,
                                            egui::Color32::WHITE,
                                            egui::Stroke::new(1.0, accent),
                                        )
                                    } else {
                                        (
                                            egui::Color32::TRANSPARENT,
                                            app.theme.fg,
                                            egui::Stroke::NONE,
                                        )
                                    };
                                    let width = ui.available_width().max(80.0);
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(width, 24.0),
                                        egui::Sense::click(),
                                    );
                                    if ui.is_rect_visible(rect) {
                                        ui.painter().rect(
                                            rect,
                                            egui::Rounding::same(6.0),
                                            bg,
                                            stroke,
                                        );
                                        ui.painter().text(
                                            rect.center(),
                                            egui::Align2::CENTER_CENTER,
                                            tag.as_str(),
                                            egui::FontId::new(11.5, egui::FontFamily::Proportional),
                                            fg,
                                        );
                                    }
                                    if response.clicked() {
                                        if selected {
                                            app.selected_saved_tag = None;
                                        } else {
                                            app.load_tag_detail(tag);
                                        }
                                    }
                                }
                            }
                        });
                    });
            });

            ui.vertical(|ui| {
                ui.set_width(detail_w);
                egui::Frame::none()
                    .fill(app.theme.data_bg)
                    .rounding(egui::Rounding::same(8.0))
                    .stroke(egui::Stroke::new(1.0, app.theme.data_border))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.set_width((detail_w - 20.0).max(72.0));
                        if let Some(ref sel) = app.selected_saved_tag.clone() {
                            ui.label(
                                egui::RichText::new(sel.as_str())
                                    .size(14.0)
                                    .color(app.theme.fg),
                            );
                            ui.add_space(4.0);

                            let count = app.storage.count_entries_for_tag(sel).unwrap_or(0);
                            ui.label(
                                egui::RichText::new(format!(
                                    "{}: {}",
                                    t!("settings.tags.related_records"),
                                    count
                                ))
                                .size(11.5)
                                .color(app.theme.muted),
                            );
                            ui.add_space(8.0);

                            ui.label(
                                egui::RichText::new(t!("settings.tags.tag_color"))
                                    .size(11.0)
                                    .color(app.theme.muted),
                            );
                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                let preview_color = hex_to_color32(&app.tag_detail_color)
                                    .unwrap_or(app.theme.accent);
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(20.0, 20.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    egui::Rounding::same(4.0),
                                    preview_color,
                                );
                                let color_response = ui.add_sized(
                                    [80.0, 20.0],
                                    egui::TextEdit::singleline(&mut app.tag_detail_color)
                                        .desired_width(80.0),
                                );
                                if color_response.changed()
                                    && let Err(err) = app
                                        .storage
                                        .update_saved_tag_color(sel, &app.tag_detail_color)
                                {
                                    app.status = format!(
                                        "{}: {err}",
                                        t!("settings.tags.update_color_failed")
                                    );
                                }
                            });

                            ui.add_space(8.0);
                            let add_to_current_width = ui.available_width().max(72.0);
                            if MacosButton::normal()
                                .min_width(add_to_current_width)
                                .height(26.0)
                                .font_size(11.0)
                                .show(ui, t!("settings.tags.add_to_current"), &app.theme)
                                .clicked()
                            {
                                let tag = sel.clone();
                                app.add_tag_to_editor(&tag);
                            }
                            ui.add_space(2.0);
                            let remove_width = ui.available_width().max(72.0);
                            if settings_danger_button(
                                ui,
                                t!("settings.tags.remove_from_catalog"),
                                &app.theme,
                                remove_width,
                            )
                            .clicked()
                            {
                                let tag = sel.clone();
                                app.delete_saved_tag(&tag);
                                app.selected_saved_tag = None;
                            }
                        } else {
                            ui.label(
                                egui::RichText::new(t!("settings.tags.click_left_hint"))
                                    .size(12.0)
                                    .color(app.theme.muted),
                            );
                        }
                    });
            });
        });
    });
    let collapsed = !expanded;
    if collapsed != prev {
        app.settings_panel_collapsed[5] = collapsed;
        app.persist_preferences();
    }
}
