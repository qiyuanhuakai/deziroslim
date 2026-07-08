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
        let popup_id = ui.make_persistent_id(("entry_actions_menu", entry.id));
        let response = MacosButton::normal()
            .min_width(0.0)
            .height(26.0)
            .font_size(12.5)
            .show(ui, t!("settings.actions.context_menu_title"), &app.theme);
        if response.clicked() {
            let is_open = ui.memory(|mem| mem.is_popup_open(popup_id));
            if is_open {
                ui.memory_mut(|mem| mem.close_popup());
            } else {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
            }
        }
        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &response,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.set_min_width(180.0);
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
                        ui.memory_mut(|mem| mem.close_popup());
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
