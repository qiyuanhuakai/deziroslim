use crate::app::ClipboardApp;
use crate::ui::widgets::macos_toggle;
use eframe::egui;
use rust_i18n::t;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum EditorResult {
    Save(crate::actions::Action),
    Delete(u64),
    Cancel,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionEditor {
    pub is_open: bool,
    pub editing_id: Option<u64>,
    pub name: String,
    pub pattern: String,
    pub command: String,
    pub icon: String,
    pub kind: String,
    pub auto_trigger: bool,
    pub auto_trigger_primary: bool,
    pub toolbar_button: bool,
    pub enabled: bool,
    pub sort_order: i32,
    pub test_pattern_text: String,
    pub test_pattern_result: String,
    pub test_pattern_open: bool,
}

impl ActionEditor {
    pub fn open_new(&mut self) {
        self.is_open = true;
        self.editing_id = None;
        self.name.clear();
        self.pattern.clear();
        self.command.clear();
        self.icon.clear();
        self.kind = "shell_command".to_string();
        self.auto_trigger = false;
        self.auto_trigger_primary = false;
        self.toolbar_button = false;
        self.enabled = true;
        self.sort_order = 0;
    }

    pub fn open_edit(&mut self, action: &crate::actions::Action) {
        self.is_open = true;
        self.editing_id = Some(action.id);
        self.name = action.name.clone();
        self.pattern = action.pattern.clone();
        self.command = action.command.clone();
        self.icon = action.icon.clone();
        self.kind = format!("{:?}", action.kind).to_lowercase();
        self.auto_trigger = action.auto_trigger;
        self.auto_trigger_primary = action.auto_trigger_primary;
        self.toolbar_button = action.toolbar_button;
        self.enabled = action.enabled;
        self.sort_order = action.sort_order;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }
}

pub fn draw_action_editor_dialog(
    ctx: &egui::Context,
    app: &mut ClipboardApp,
) -> Option<EditorResult> {
    if !app.action_editor.is_open {
        return None;
    }

    let theme = app.theme.clone();
    let mut result = None;
    let mut close = false;

    egui::Window::new(t!("settings.actions.edit_action"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .frame(
            egui::Frame::none()
                .fill(theme.card)
                .stroke(egui::Stroke::new(1.0, theme.border))
                .rounding(egui::Rounding::same(12.0))
                .inner_margin(egui::Margin::same(20.0))
                .shadow(egui::epaint::Shadow {
                    spread: 8.0,
                    color: egui::Color32::from_black_alpha(30),
                    ..Default::default()
                }),
        )
        .show(ctx, |ui| {
            ui.set_min_width(340.0);

            ui.label(
                egui::RichText::new(t!("settings.actions.name_label"))
                    .size(12.5)
                    .color(theme.muted),
            );
            ui.text_edit_singleline(&mut app.action_editor.name);
            ui.add_space(6.0);

            ui.label(
                egui::RichText::new(t!("settings.actions.pattern_label"))
                    .size(12.5)
                    .color(theme.muted),
            );
            ui.text_edit_singleline(&mut app.action_editor.pattern)
                .on_hover_text(t!("settings.actions.pattern_placeholder"));
            ui.add_space(6.0);

            ui.label(
                egui::RichText::new(t!("settings.actions.command_label"))
                    .size(12.5)
                    .color(theme.muted),
            );
            ui.text_edit_singleline(&mut app.action_editor.command)
                .on_hover_text(t!("settings.actions.command_placeholder"));
            ui.add_space(6.0);

            ui.label(
                egui::RichText::new(t!("settings.actions.icon_label"))
                    .size(12.5)
                    .color(theme.muted),
            );
            ui.text_edit_singleline(&mut app.action_editor.icon);
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t!("settings.actions.enabled")).size(12.5));
                macos_toggle(ui, &mut app.action_editor.enabled, &theme);
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t!("settings.actions.is_automatic")).size(12.5));
                macos_toggle(ui, &mut app.action_editor.auto_trigger, &theme);
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(t!("settings.actions.is_primary")).size(12.5));
                macos_toggle(ui, &mut app.action_editor.toolbar_button, &theme);
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        [100.0, 30.0],
                        egui::Button::new(
                            egui::RichText::new(t!("common.save")).size(12.5).strong(),
                        )
                        .rounding(egui::Rounding::same(6.0))
                        .fill(theme.accent),
                    )
                    .on_disabled_hover_text(t!("settings.actions.name_empty"))
                    .clicked()
                {
                    if app.action_editor.name.trim().is_empty()
                        || app.action_editor.pattern.trim().is_empty()
                        || app.action_editor.command.trim().is_empty()
                    {
                        app.status = t!("settings.actions.name_empty").to_string();
                    } else {
                        let kind = match app.action_editor.kind.as_str() {
                            "open" => crate::actions::ActionKind::Open,
                            "open_with" => crate::actions::ActionKind::OpenWith,
                            "copy" => crate::actions::ActionKind::Copy,
                            _ => crate::actions::ActionKind::ShellCommand,
                        };
                        let mut action = crate::actions::Action::new(
                            app.action_editor.name.trim(),
                            app.action_editor.pattern.trim(),
                            app.action_editor.command.trim(),
                        );
                        action.kind = kind;
                        action.icon = app.action_editor.icon.clone();
                        action.enabled = app.action_editor.enabled;
                        action.auto_trigger = app.action_editor.auto_trigger;
                        action.auto_trigger_primary = app.action_editor.auto_trigger_primary;
                        action.toolbar_button = app.action_editor.toolbar_button;
                        action.sort_order = app.action_editor.sort_order;
                        if let Some(id) = app.action_editor.editing_id {
                            action.id = id;
                        }
                        result = Some(EditorResult::Save(action));
                        close = true;
                    }
                }

                if app.action_editor.editing_id.is_some()
                    && ui
                        .add_sized(
                            [80.0, 30.0],
                            egui::Button::new(
                                egui::RichText::new(t!("common.delete")).size(12.5).strong(),
                            )
                            .rounding(egui::Rounding::same(6.0))
                            .fill(theme.danger),
                        )
                        .clicked()
                    && let Some(id) = app.action_editor.editing_id
                {
                    result = Some(EditorResult::Delete(id));
                    close = true;
                }

                if ui
                    .add_sized(
                        [80.0, 30.0],
                        egui::Button::new(egui::RichText::new(t!("common.cancel")).size(12.5))
                            .rounding(egui::Rounding::same(6.0))
                            .fill(theme.card),
                    )
                    .clicked()
                {
                    result = Some(EditorResult::Cancel);
                    close = true;
                }
            });
        });

    if close {
        app.action_editor.close();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_result_variants() {
        let action = crate::actions::Action::new("test", "pat", "cmd");
        let save = EditorResult::Save(action.clone());
        assert!(matches!(save, EditorResult::Save(_)));

        let del = EditorResult::Delete(42);
        assert!(matches!(del, EditorResult::Delete(42)));

        let cancel = EditorResult::Cancel;
        assert!(matches!(cancel, EditorResult::Cancel));
    }

    #[test]
    fn action_editor_open_new() {
        let mut editor = ActionEditor::default();
        editor.open_new();
        assert!(editor.is_open);
        assert!(editor.editing_id.is_none());
        assert!(editor.name.is_empty());
    }

    #[test]
    fn action_editor_open_edit() {
        let mut editor = ActionEditor::default();
        let mut action = crate::actions::Action::new("Test", "^https://", "firefox %1");
        action.auto_trigger = true;
        action.toolbar_button = true;
        editor.open_edit(&action);
        assert!(editor.is_open);
        assert_eq!(editor.editing_id, Some(action.id));
        assert_eq!(editor.name, "Test");
        assert!(editor.auto_trigger);
        assert!(editor.toolbar_button);
    }

    #[test]
    fn action_editor_close() {
        let mut editor = ActionEditor::default();
        editor.open_new();
        assert!(editor.is_open);
        editor.close();
        assert!(!editor.is_open);
    }
}
