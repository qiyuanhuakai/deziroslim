use crate::app::ClipboardApp;
use crate::ui::settings::settings_footer_button;
use crate::ui::widgets::macos_toggle;
use eframe::egui;
use rust_i18n::t;

const PANEL_INDEX: usize = 8;

pub fn draw_actions_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app
        .settings_panel_collapsed
        .get(PANEL_INDEX)
        .copied()
        .unwrap_or(false);
    let mut expanded = !prev;
    let theme = app.theme.clone();

    crate::ui::widgets::macos_collapsible_group(
        ui,
        t!("settings.actions.title"),
        &mut expanded,
        &theme,
        |ui| {
            draw_global_settings(ui, app);
            ui.add_space(8.0);
            draw_action_list(ui, app);
            ui.add_space(8.0);
            draw_action_buttons(ui, app);
            ui.add_space(8.0);
            if app.action_editor.test_pattern_open {
                draw_test_pattern(ui, app);
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

fn draw_global_settings(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();

    if ui
        .horizontal(|ui| {
            ui.label(t!("settings.actions.enabled"));
            macos_toggle(ui, &mut app.builtin_actions_enabled, &theme)
        })
        .inner
        .changed()
    {
        app.persist_preferences();
    }

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(t!("settings.actions.command_label"))
            .size(12.0)
            .color(theme.muted),
    );

    let allowlist_text = app.action_command_allowlist.clone();
    let mut allowlist_buf = allowlist_text.clone();

    ui.add(
        egui::TextEdit::multiline(&mut allowlist_buf)
            .desired_width(ui.available_width())
            .desired_rows(2)
            .hint_text(t!("settings.actions.command_placeholder")),
    );

    if allowlist_buf != allowlist_text {
        app.action_command_allowlist = allowlist_buf
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        app.persist_preferences();
    }
}

fn draw_action_list(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();
    let actions = app.actions.clone();
    let selected_id = app.action_editor.editing_id;

    if actions.is_empty() {
        ui.label(egui::RichText::new(t!("settings.actions.no_actions")).color(theme.muted));
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            for action in &actions {
                let is_selected = selected_id == Some(action.id);
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
                            let icon = if action.icon.is_empty() {
                                "\u{26A1}"
                            } else {
                                &action.icon
                            };
                            ui.label(egui::RichText::new(icon).size(14.0).color(theme.accent));
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(&action.name)
                                        .size(12.5)
                                        .strong()
                                        .color(theme.fg),
                                );
                                ui.label(
                                    egui::RichText::new(&action.pattern)
                                        .size(11.0)
                                        .color(theme.muted),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let status_color = if action.enabled {
                                        theme.accent
                                    } else {
                                        theme.muted
                                    };
                                    ui.label(
                                        egui::RichText::new(if action.enabled {
                                            "\u{25CF}"
                                        } else {
                                            "\u{25CB}"
                                        })
                                        .size(10.0)
                                        .color(status_color),
                                    );
                                },
                            );
                        });
                    })
                    .response;

                if response.interact(egui::Sense::click()).clicked() {
                    app.action_editor.open_edit(action);
                }
            }
        });
}

fn draw_action_buttons(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();

    ui.horizontal(|ui| {
        if settings_footer_button(ui, t!("settings.actions.new_action"), &theme, 120.0).clicked() {
            app.action_editor.open_new();
        }
        if settings_footer_button(ui, t!("settings.actions.test_pattern"), &theme, 120.0).clicked()
        {
            app.action_editor.test_pattern_open = !app.action_editor.test_pattern_open;
        }
        if settings_footer_button(ui, t!("settings.actions.test_run"), &theme, 120.0).clicked()
            && let Some(id) = app.action_editor.editing_id
            && let Some(action) = app.actions.iter().find(|a| a.id == id)
        {
            let content = app.action_editor.test_pattern_text.clone();
            let action_clone = action.clone();
            let executor = crate::actions::executor::ActionExecutor::new();
            executor.execute_async(&action_clone, &content);
            app.status = t!("settings.actions.run_success").to_string();
        }
    });
}

fn draw_test_pattern(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();

    ui.label(
        egui::RichText::new(t!("settings.actions.test_pattern"))
            .size(13.0)
            .strong()
            .color(theme.fg),
    );
    ui.add_space(4.0);

    ui.add(
        egui::TextEdit::singleline(&mut app.action_editor.test_pattern_text)
            .desired_width(ui.available_width())
            .hint_text(t!("settings.actions.pattern_placeholder")),
    );
    ui.add_space(4.0);

    if ui
        .add_sized(
            [120.0, 28.0],
            egui::Button::new(
                egui::RichText::new(t!("settings.actions.test_run"))
                    .size(12.0)
                    .strong(),
            )
            .rounding(egui::Rounding::same(6.0))
            .fill(theme.accent),
        )
        .clicked()
    {
        let text = app.action_editor.test_pattern_text.clone();
        let test_result = if text.is_empty() {
            t!("settings.actions.test_result_no_match").to_string()
        } else {
            let mut matched_count = 0;
            for action in &app.actions {
                if !action.enabled {
                    continue;
                }
                let matcher = crate::actions::matcher::ActionMatcher::new(vec![action.clone()]);
                if matcher.find_first_match(&text).is_some() {
                    matched_count += 1;
                }
            }
            if matched_count > 0 {
                t!("settings.actions.test_result_match", count = matched_count).to_string()
            } else {
                t!("settings.actions.test_result_no_match").to_string()
            }
        };
        app.action_editor.test_pattern_result = test_result;
    }

    if !app.action_editor.test_pattern_result.is_empty() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(&app.action_editor.test_pattern_result)
                .size(12.0)
                .color(theme.accent),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_index_is_8() {
        assert_eq!(PANEL_INDEX, 8);
    }
}
