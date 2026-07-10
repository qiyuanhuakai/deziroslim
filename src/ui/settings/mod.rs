pub mod actions;
pub mod appearance;
pub mod clipboard;
pub mod data;
pub mod default_apps;
pub mod general;
pub mod privacy;
pub mod shortcuts;
pub mod snippets;
pub mod sync;
pub mod tags;

use crate::app::ClipboardApp;
use crate::ui::MacosTokens;
use crate::ui::widgets::MacosButton;
use eframe::egui;
use rust_i18n::t;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsTab {
    General,
    Shortcuts,
    Clipboard,
    Appearance,
    DefaultApps,
    Tags,
    Data,
    #[allow(dead_code)]
    Privacy,
    Actions,
    #[allow(dead_code)]
    Primary,
    #[allow(dead_code)]
    Encryption,
    Sync,
    Snippets,
}

impl SettingsTab {
    pub const IMPLEMENTED: &'static [SettingsTab] = &[
        SettingsTab::General,
        SettingsTab::Shortcuts,
        SettingsTab::Clipboard,
        SettingsTab::Appearance,
        SettingsTab::DefaultApps,
        SettingsTab::Tags,
        SettingsTab::Data,
        SettingsTab::Privacy,
        SettingsTab::Actions,
        SettingsTab::Sync,
        SettingsTab::Snippets,
    ];
}

/// Dispatch to the appropriate panel renderer for the given tab.
pub fn dispatch_panel(
    tab: SettingsTab,
    ui: &mut egui::Ui,
    app: &mut ClipboardApp,
    ctx: &egui::Context,
) {
    match tab {
        SettingsTab::General => general::draw_general_panel(ui, app, ctx),
        SettingsTab::Shortcuts => shortcuts::draw_shortcuts_panel(ui, app, ctx),
        SettingsTab::Clipboard => clipboard::draw_clipboard_panel(ui, app, ctx),
        SettingsTab::Appearance => appearance::draw_appearance_panel(ui, app, ctx),
        SettingsTab::DefaultApps => default_apps::draw_default_apps_panel(ui, app, ctx),
        SettingsTab::Tags => tags::draw_tags_panel(ui, app, ctx),
        SettingsTab::Data => data::draw_data_panel(ui, app, ctx),
        SettingsTab::Privacy => privacy::draw_privacy_panel(ui, app, ctx),
        SettingsTab::Actions => actions::draw_actions_panel(ui, app, ctx),
        SettingsTab::Sync => sync::draw_sync_panel(ui, app, ctx),
        SettingsTab::Snippets => snippets::draw_snippets_panel(ui, app, ctx),
        _ => {
            ui.label(t!("settings.panel_not_implemented"));
        }
    }
}

pub(crate) fn apply_settings_widget_rounding(ui: &mut egui::Ui, radius: f32) {
    let mut style = ui.style().as_ref().clone();
    let rounding = egui::Rounding::same(radius);
    style.visuals.widgets.noninteractive.rounding = rounding;
    style.visuals.widgets.inactive.rounding = rounding;
    style.visuals.widgets.hovered.rounding = rounding;
    style.visuals.widgets.active.rounding = rounding;
    style.visuals.widgets.open.rounding = rounding;
    ui.set_style(style);
}

pub(crate) fn settings_footer_button(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    theme: &MacosTokens,
    width: f32,
) -> egui::Response {
    MacosButton::normal()
        .min_width(width)
        .height(30.0)
        .font_size(12.5)
        .show(ui, label, theme)
}

pub(crate) fn settings_primary_button(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    theme: &MacosTokens,
    width: f32,
) -> egui::Response {
    MacosButton::primary()
        .min_width(width)
        .height(30.0)
        .font_size(12.5)
        .show(ui, label, theme)
}

pub(crate) fn settings_danger_button(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    theme: &MacosTokens,
    width: f32,
) -> egui::Response {
    MacosButton::danger()
        .min_width(width)
        .height(30.0)
        .font_size(12.5)
        .show(ui, label, theme)
}

pub(crate) fn hotkey_record_row(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    value: &str,
    recording: bool,
    mut actions: impl FnMut(&mut egui::Ui),
) {
    let label = label.as_ref();
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 30.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(label);
            let display = if recording {
                t!("settings.hotkey.recording_active").to_string()
            } else if value.trim().is_empty() {
                t!("settings.hotkey.not_set").to_string()
            } else {
                value.lines().map(str::trim).collect::<Vec<_>>().join(" / ")
            };
            ui.monospace(display);
            actions(ui);
        },
    );
}

pub(crate) fn hotkey_single_record_row(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    value: &str,
    recording: bool,
    theme: &MacosTokens,
    mut start_recording: impl FnMut(),
) {
    hotkey_record_row(ui, label, value, recording, |ui| {
        if settings_footer_button(ui, t!("common.recording"), theme, 0.0).clicked() {
            start_recording();
        }
    });
}

pub(crate) fn removable_hotkey_chip(
    ui: &mut egui::Ui,
    hotkey: &str,
    theme: &MacosTokens,
) -> egui::Response {
    use crate::app::scale_alpha;

    let label = format!("{hotkey}  \u{00d7}");
    let font_id = egui::FontId::monospace(12.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label.clone(), font_id.clone(), theme.fg);
    let size = egui::vec2((galley.size().x + 18.0).max(44.0), 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let fill = if response.hovered() {
        scale_alpha(theme.danger, 0.14)
    } else {
        theme.input_bg
    };
    let stroke = if response.hovered() {
        egui::Stroke::new(1.0, theme.danger)
    } else {
        egui::Stroke::new(1.0, theme.input_border)
    };
    ui.painter()
        .rect(rect, egui::Rounding::same(theme.radius_input), fill, stroke);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        font_id,
        if response.hovered() {
            theme.danger
        } else {
            theme.fg
        },
    );
    response.on_hover_text(t!("settings.hotkey.delete_tooltip"))
}

pub(crate) struct DropdownOption {
    pub(crate) value: String,
    pub(crate) label: String,
}

impl DropdownOption {
    pub(crate) fn borrowed(value: &str, label: impl AsRef<str>) -> Self {
        Self {
            value: value.to_string(),
            label: label.as_ref().to_string(),
        }
    }

    pub(crate) fn owned(value: String, label: String) -> Self {
        Self { value, label }
    }
}

fn clipped_chip_label(label: impl AsRef<str>, max_chars: usize) -> String {
    use crate::app::clipped_chip_label;
    clipped_chip_label(label, max_chars)
}

fn combo_popup_direction(ui: &egui::Ui, button_rect: egui::Rect) -> egui::AboveOrBelow {
    let screen = ui.ctx().input(|input| input.screen_rect());
    let margin = 12.0;
    let estimated_popup_height = 312.0;
    let below_space = screen.bottom() - button_rect.bottom() - margin;
    let above_space = button_rect.top() - screen.top() - margin;
    if below_space < estimated_popup_height && above_space > below_space {
        egui::AboveOrBelow::Above
    } else {
        egui::AboveOrBelow::Below
    }
}

fn dropdown_option_row(
    ui: &mut egui::Ui,
    option: &DropdownOption,
    selected: bool,
    theme: &MacosTokens,
) -> egui::Response {
    let width = ui.available_width().max(120.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
    let fill = if selected {
        theme.accent_soft
    } else if response.hovered() {
        theme.select_menu_hover_bg
    } else {
        egui::Color32::TRANSPARENT
    };
    if fill != egui::Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(theme.radius_input), fill);
    }
    let color = theme.fg;
    ui.painter().text(
        rect.left_center() + egui::vec2(10.0, 0.0),
        egui::Align2::LEFT_CENTER,
        clipped_chip_label(&option.label, 38),
        egui::FontId::proportional(12.5),
        color,
    );
    if selected {
        ui.painter().text(
            rect.right_center() - egui::vec2(8.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            "\u{2713}",
            egui::FontId::proportional(12.0),
            theme.accent,
        );
    }
    response
}

pub(crate) fn searchable_combo_row(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    selected: &mut String,
    search: &mut String,
    options: &[DropdownOption],
    search_hint: impl AsRef<str>,
    theme: &MacosTokens,
) -> bool {
    let label = label.as_ref();
    let search_hint = search_hint.as_ref();
    let before = selected.clone();
    ui.vertical(|ui| {
        ui.label(label);
        let popup_id = ui.make_persistent_id(format!("searchable_combo_popup_{label}"));
        let search_id = ui.make_persistent_id(format!("searchable_combo_search_{label}"));
        let button_width = ui.available_width().clamp(120.0, 360.0);
        let selected_label = options
            .iter()
            .find(|option| option.value == *selected)
            .map(|option| option.label.as_str())
            .unwrap_or_else(|| selected.as_str());
        let is_open = ui.memory(|mem| mem.is_popup_open(popup_id));
        let arrow = if is_open { "\u{25b4}" } else { "\u{25be}" };
        let fill = if is_open {
            theme.card_hover
        } else {
            theme.input_bg
        };
        let stroke = if is_open {
            egui::Stroke::new(1.2, theme.accent)
        } else {
            egui::Stroke::new(1.0, theme.input_border)
        };
        let (button_rect, button) =
            ui.allocate_exact_size(egui::vec2(button_width, 34.0), egui::Sense::click());
        ui.painter().rect(
            button_rect,
            egui::Rounding::same(theme.radius_input),
            fill,
            stroke,
        );
        ui.painter().text(
            button_rect.left_center() + egui::vec2(12.0, 0.0),
            egui::Align2::LEFT_CENTER,
            clipped_chip_label(selected_label, 32),
            egui::FontId::proportional(12.5),
            theme.fg,
        );
        let arrow_rect = egui::Rect::from_min_max(
            egui::pos2(button_rect.right() - 34.0, button_rect.top()),
            button_rect.right_bottom(),
        );
        ui.painter().text(
            arrow_rect.center() + egui::vec2(0.0, -0.5),
            egui::Align2::CENTER_CENTER,
            arrow,
            egui::FontId::proportional(24.0),
            if is_open { theme.accent } else { theme.muted },
        );
        if button.clicked() {
            if is_open {
                ui.memory_mut(|mem| mem.close_popup());
            } else {
                ui.memory_mut(|mem| mem.open_popup(popup_id));
                ui.memory_mut(|mem| mem.data.insert_temp(search_id.with("focus"), true));
            }
        }

        let popup_direction = combo_popup_direction(ui, button.rect);
        let mut popup_style = ui.style().as_ref().clone();
        popup_style.visuals.window_fill = theme.select_menu_bg;
        popup_style.visuals.window_stroke = egui::Stroke::new(1.0, theme.select_menu_border);
        popup_style.visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 4.0),
            blur: 16.0,
            spread: 2.0,
            color: theme.shadow,
        };
        popup_style.visuals.menu_rounding = egui::Rounding::same(theme.radius_input);
        popup_style.spacing.menu_margin = egui::Margin::same(10.0);
        ui.scope(|ui| {
            ui.set_style(popup_style);
            egui::popup::popup_above_or_below_widget(
                ui,
                popup_id,
                &button,
                popup_direction,
                egui::popup::PopupCloseBehavior::CloseOnClickOutside,
                |ui| {
                    ui.set_min_width((button.rect.width() - 20.0).max(160.0));
                    ui.set_max_width((button.rect.width() - 20.0).max(160.0));
                    let search_response = ui
                        .scope(|ui| {
                            apply_settings_widget_rounding(ui, theme.radius_input);
                            ui.add(
                                egui::TextEdit::singleline(search)
                                    .id(search_id)
                                    .hint_text(search_hint)
                                    .desired_width(ui.available_width().max(120.0)),
                            )
                        })
                        .inner;
                    let should_focus = ui
                        .memory_mut(|mem| mem.data.remove_temp::<bool>(search_id.with("focus")))
                        .unwrap_or(false);
                    if should_focus {
                        search_response.request_focus();
                    }
                    ui.add_space(8.0);

                    let query = search.trim().to_ascii_lowercase();
                    let mut shown = 0usize;
                    egui::ScrollArea::vertical()
                        .max_height(260.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for option in options {
                                let haystack = format!("{} {}", option.label, option.value)
                                    .to_ascii_lowercase();
                                if !query.is_empty() && !haystack.contains(&query) {
                                    continue;
                                }
                                if dropdown_option_row(ui, option, *selected == option.value, theme)
                                    .clicked()
                                {
                                    *selected = option.value.clone();
                                    search.clear();
                                    ui.memory_mut(|mem| mem.close_popup());
                                }
                                shown += 1;
                                if shown >= 80 {
                                    ui.label(
                                        egui::RichText::new(t!("search.fuzzy_hint"))
                                            .italics()
                                            .color(theme.muted),
                                    );
                                    break;
                                }
                            }
                            if shown == 0 {
                                ui.label(
                                    egui::RichText::new(t!("search.no_match"))
                                        .italics()
                                        .color(theme.muted),
                                );
                            }
                        });
                },
            );
        });
    });
    *selected != before
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn font_combo_row(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    selected: &mut String,
    search: &mut String,
    choices: &[String],
    automatic_label: &str,
    search_hint: impl AsRef<str>,
    theme: &MacosTokens,
) -> bool {
    let mut options = Vec::with_capacity(choices.len() + 1);
    options.push(DropdownOption::borrowed(
        crate::app::AUTO_FONT_VALUE,
        automatic_label,
    ));
    options.extend(
        choices
            .iter()
            .map(|choice| DropdownOption::owned(choice.clone(), choice.clone())),
    );
    searchable_combo_row(ui, label, selected, search, &options, search_hint, theme)
}

pub(crate) fn app_combo_row(
    ui: &mut egui::Ui,
    label: impl AsRef<str>,
    selected: &mut String,
    search: &mut String,
    choices: &[crate::platform::AppChoice],
    theme: &MacosTokens,
) -> bool {
    let mut options = Vec::with_capacity(choices.len() + 1);
    options.push(DropdownOption::borrowed(
        "",
        t!("settings.default_app.system_default"),
    ));
    options.extend(choices.iter().map(|choice| {
        DropdownOption::owned(
            choice.command.clone(),
            format!("{}  ({})", choice.name, choice.command),
        )
    }));
    searchable_combo_row(
        ui,
        label,
        selected,
        search,
        &options,
        t!("settings.default_app.search_hint"),
        theme,
    )
}

pub(crate) fn hex_to_color32(hex: &str) -> Option<egui::Color32> {
    let hex = hex.trim().strip_prefix('#').unwrap_or(hex.trim());
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(egui::Color32::from_rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(egui::Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => None,
    }
}

pub(crate) fn pick_database_save_dir_with_dialog(
    current: &std::path::Path,
) -> Result<Option<std::path::PathBuf>, String> {
    use crate::storage::Storage;
    use std::path::{Path, PathBuf};

    let current_dir = if current.as_os_str().is_empty() {
        Storage::default_path()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    } else if current.is_dir() {
        current.to_path_buf()
    } else {
        current
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    #[cfg(target_os = "windows")]
    {
        Ok(rfd::FileDialog::new()
            .set_title(t!("error.db_select_title").to_string())
            .set_directory(current_dir)
            .pick_folder())
    }

    #[cfg(not(target_os = "windows"))]
    {
        match std::process::Command::new("zenity")
            .arg("--file-selection")
            .arg("--directory")
            .arg(format!("--title={}", t!("error.db_select_title")))
            .arg(format!("--filename={}", current_dir.display()))
            .output()
        {
            Ok(output) if output.status.success() => {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                return Ok((!value.is_empty()).then(|| PathBuf::from(value)));
            }
            Ok(_) => return Ok(None),
            Err(_) => {}
        }

        match std::process::Command::new("kdialog")
            .arg("--getexistingdirectory")
            .arg(current_dir.display().to_string())
            .output()
        {
            Ok(output) if output.status.success() => {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Ok((!value.is_empty()).then(|| PathBuf::from(value)))
            }
            Ok(_) => Ok(None),
            Err(_) => {
                let fallback = Storage::default_path();
                Err(format!(
                    "{} ({})",
                    t!("error.dialog_not_found"),
                    fallback.display()
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_panel_implemented_count() {
        assert_eq!(
            SettingsTab::IMPLEMENTED.len(),
            11,
            "Expected 11 implemented panels"
        );
    }

    #[test]
    fn test_dispatch_panel_all_implemented_variants_covered() {
        let mut seen = std::collections::HashSet::new();
        for &tab in SettingsTab::IMPLEMENTED {
            assert!(seen.insert(tab), "Duplicate tab variant: {tab:?}");
        }
    }

    #[test]
    fn test_settings_tab_debug() {
        let tab = SettingsTab::General;
        let debug = format!("{tab:?}");
        assert_eq!(debug, "General");
    }
}
