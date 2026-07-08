use crate::app::ClipboardApp;
use crate::ui::widgets::MacosButton;
use eframe::egui;
use rust_i18n::t;

#[derive(Debug, Clone, Default)]
pub struct ActionsPopover {
    pub open: bool,
}

impl ActionsPopover {
    pub fn new() -> Self {
        Self { open: false }
    }
}

pub fn draw_toolbar_actions_button(ui: &mut egui::Ui, app: &mut ClipboardApp) {
    let theme = app.theme.clone();
    let toolbar_actions = app.toolbar_actions();

    if toolbar_actions.is_empty() {
        return;
    }

    let popup_id = ui.make_persistent_id("actions_popover");

    let button_response = MacosButton::normal()
        .min_width(32.0)
        .height(32.0)
        .padding_x(0.0)
        .font_size(16.0)
        .show(ui, "\u{26A1}", &theme);
    let button_response = button_response.on_hover_text(t!("settings.actions.popover_title"));

    if toolbar_actions.len() == 1 {
        if button_response.clicked() {
            let action = toolbar_actions[0].clone();
            app.pending_toolbar_action = Some(action);
        }
    } else if button_response.clicked() {
        app.actions_popover.open = !app.actions_popover.open;
    }

    if app.actions_popover.open && toolbar_actions.len() > 1 {
        egui::popup::popup_below_widget(
            ui,
            popup_id,
            &button_response,
            egui::popup::PopupCloseBehavior::CloseOnClickOutside,
            |ui| {
                ui.set_min_width(180.0);
                ui.label(
                    egui::RichText::new(t!("settings.actions.popover_title"))
                        .strong()
                        .size(13.0),
                );
                ui.separator();
                for action in &toolbar_actions {
                    let label = if action.icon.is_empty() {
                        action.name.clone()
                    } else {
                        format!("{} {}", action.icon, action.name)
                    };
                    if MacosButton::normal()
                        .min_width(0.0)
                        .height(28.0)
                        .font_size(12.5)
                        .show(ui, &label, &theme)
                        .on_hover_text(&action.command)
                        .clicked()
                    {
                        app.actions_popover.open = false;
                        app.pending_toolbar_action = Some(action.clone());
                        return;
                    }
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_popover_default_is_closed() {
        let popover = ActionsPopover::default();
        assert!(!popover.open);
    }

    #[test]
    fn actions_popover_new_is_closed() {
        let popover = ActionsPopover::new();
        assert!(!popover.open);
    }
}
