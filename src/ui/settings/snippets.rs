use crate::app::ClipboardApp;
use crate::snippets::Snippet;
use crate::ui::settings::settings_footer_button;
use eframe::egui;
use rust_i18n::t;

const PANEL_INDEX: usize = 10;

pub fn draw_snippets_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app
        .settings_panel_collapsed
        .get(PANEL_INDEX)
        .copied()
        .unwrap_or(false);
    let mut expanded = !prev;
    let theme = app.theme.clone();

    crate::ui::widgets::macos_collapsible_group(
        ui,
        t!("settings.snippets.title"),
        &mut expanded,
        &theme,
        |ui| {
            draw_snippet_list(ui, app);
            ui.add_space(8.0);
            draw_snippet_buttons(ui, app);
            ui.add_space(8.0);
            if app.snippet_editor_open {
                draw_snippet_editor(ui, app);
            }
        },
    );

    let collapsed_ref = app.settings_panel_collapsed.get_mut(PANEL_INDEX);
    if let Some(collapsed) = collapsed_ref
        && expanded == *collapsed
    {
        *collapsed = !expanded;
        app.persist_preferences();
    }
}

fn draw_snippet_list(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();
    let snippets = app.snippets.clone();
    let selected_id = app.snippet_editing_id;

    if snippets.is_empty() {
        ui.label(egui::RichText::new(t!("settings.snippets.no_snippets")).color(theme.muted));
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            for snippet in &snippets {
                let is_selected = selected_id == Some(snippet.id);
                let fill = if is_selected {
                    theme.history_selected
                } else {
                    theme.card
                };
                let stroke = if is_selected {
                    egui::Stroke::new(1.0, theme.accent)
                } else {
                    egui::Stroke::new(1.0, theme.border)
                };

                let frame = egui::Frame::none()
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 6.0));

                let response = frame
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let icon = if snippet.icon.is_empty() {
                                "\u{1F4CB}"
                            } else {
                                &snippet.icon
                            };
                            ui.label(egui::RichText::new(icon).size(14.0).color(theme.accent));
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&snippet.name)
                                        .size(12.5)
                                        .strong()
                                        .color(theme.fg),
                                );
                                let preview: String = snippet.template.chars().take(60).collect();
                                ui.label(
                                    egui::RichText::new(preview)
                                        .size(11.0)
                                        .color(theme.muted),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            snippet.use_count,
                                            t!("settings.snippets.uses")
                                        ))
                                        .size(10.0)
                                        .color(theme.muted),
                                    );
                                },
                            );
                        });
                    })
                    .response;

                if response.interact(egui::Sense::click()).clicked() {
                    app.snippet_editing_id = Some(snippet.id);
                    app.snippet_edit_name = snippet.name.clone();
                    app.snippet_edit_template = snippet.template.clone();
                    app.snippet_edit_description = snippet.description.clone();
                    app.snippet_edit_tags = snippet.tags.join(", ");
                    app.snippet_editor_open = true;
                }
            }
        });
}

fn draw_snippet_buttons(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();

    ui.horizontal(|ui| {
        if settings_footer_button(ui, t!("settings.snippets.new_snippet"), &theme, 120.0).clicked()
        {
            app.snippet_editing_id = None;
            app.snippet_edit_name.clear();
            app.snippet_edit_template.clear();
            app.snippet_edit_description.clear();
            app.snippet_edit_tags.clear();
            app.snippet_editor_open = true;
        }

        if app.snippet_editing_id.is_some() {
            if settings_footer_button(ui, t!("settings.snippets.delete"), &theme, 80.0).clicked()
                && let Some(id) = app.snippet_editing_id
            {
                if let Err(err) = app.storage.delete_snippet(id) {
                    app.status =
                        format!("{}: {err}", t!("settings.snippets.delete_failed"));
                } else {
                    app.snippets = app.storage.load_snippets().unwrap_or_default();
                    app.snippet_editor_open = false;
                    app.snippet_editing_id = None;
                }
            }
        }
    });
}

fn draw_snippet_editor(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();

    ui.separator();
    ui.label(
        egui::RichText::new(if app.snippet_editing_id.is_some() {
            t!("settings.snippets.edit_snippet")
        } else {
            t!("settings.snippets.new_snippet")
        })
        .size(13.0)
        .strong()
        .color(theme.fg),
    );

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("settings.snippets.field_name")).color(theme.fg));
        ui.add(
            egui::TextEdit::singleline(&mut app.snippet_edit_name)
                .desired_width(200.0)
                .hint_text(t!("settings.snippets.name_hint")),
        );
    });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("settings.snippets.field_template")).color(theme.fg));
        ui.add(
            egui::TextEdit::multiline(&mut app.snippet_edit_template)
                .desired_width(ui.available_width())
                .desired_rows(3)
                .hint_text(t!("settings.snippets.template_hint")),
        );
    });

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(t!("settings.snippets.field_description")).color(theme.fg),
        );
        ui.add(
            egui::TextEdit::singleline(&mut app.snippet_edit_description)
                .desired_width(ui.available_width())
                .hint_text(t!("settings.snippets.description_hint")),
        );
    });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("settings.snippets.field_tags")).color(theme.fg));
        ui.add(
            egui::TextEdit::singleline(&mut app.snippet_edit_tags)
                .desired_width(ui.available_width())
                .hint_text(t!("settings.snippets.tags_hint")),
        );
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if settings_footer_button(ui, t!("settings.snippets.save"), &theme, 80.0).clicked() {
            let name = app.snippet_edit_name.trim().to_string();
            let template = app.snippet_edit_template.trim().to_string();
            if name.is_empty() || template.is_empty() {
                app.status = t!("settings.snippets.name_template_required").to_string();
                return;
            }
            let tags: Vec<String> = app
                .snippet_edit_tags
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let mut snippet = Snippet::new(&name, &template);
            snippet.description = app.snippet_edit_description.trim().to_string();
            snippet.tags = tags;
            if let Some(id) = app.snippet_editing_id {
                snippet.id = id;
            }
            match app.storage.save_snippet(&snippet) {
                Ok(_) => {
                    app.snippets = app.storage.load_snippets().unwrap_or_default();
                    app.snippet_editor_open = false;
                    app.snippet_editing_id = None;
                }
                Err(err) => {
                    app.status =
                        format!("{}: {err}", t!("settings.snippets.save_failed"));
                }
            }
        }
        if settings_footer_button(ui, t!("settings.snippets.cancel"), &theme, 80.0).clicked() {
            app.snippet_editor_open = false;
            app.snippet_editing_id = None;
        }
    });
}
