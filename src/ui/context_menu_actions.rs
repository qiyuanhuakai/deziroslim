use crate::app::ClipboardApp;
use crate::model::ClipboardEntrySummary;
use crate::ui::widgets::MacosButton;
use eframe::egui;
use rust_i18n::t;

pub fn show_entry_actions_menu(
    ui: &mut egui::Ui,
    app: &mut ClipboardApp,
    entry: &ClipboardEntrySummary,
) -> Option<crate::actions::Action> {
    let matched = app.matching_actions_for_content(&entry.preview);
    let mut result = None;

    if !matched.is_empty() {
        ui.menu_button(
            egui::RichText::new(t!("settings.actions.context_menu_title")).size(12.5),
            |ui| {
                for action in &matched {
                    let label = if action.icon.is_empty() {
                        action.name.clone()
                    } else {
                        format!("{} {}", action.icon, action.name)
                    };
                    if MacosButton::normal()
                        .min_width(0.0)
                        .height(26.0)
                        .font_size(12.0)
                        .show(ui, &label, &app.theme)
                        .on_hover_text(&action.command)
                        .clicked()
                    {
                        result = Some(action.clone());
                        ui.close_menu();
                    }
                }
            },
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_entry_actions_menu_compiles() {
        let _ = show_entry_actions_menu
            as fn(
                &mut egui::Ui,
                &mut ClipboardApp,
                &ClipboardEntrySummary,
            ) -> Option<crate::actions::Action>;
    }
}
