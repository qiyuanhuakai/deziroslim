use crate::app::ClipboardApp;
use eframe::egui;
use rust_i18n::t;

const PANEL_INDEX: usize = 11;

pub fn draw_sync_panel(ui: &mut egui::Ui, app: &mut ClipboardApp, _ctx: &egui::Context) {
    let prev = app
        .settings_panel_collapsed
        .get(PANEL_INDEX)
        .copied()
        .unwrap_or(false);
    let mut expanded = !prev;
    let theme = app.theme.clone();

    crate::ui::widgets::macos_collapsible_group(
        ui,
        t!("settings.sync.title"),
        &mut expanded,
        &theme,
        |ui| {
            #[cfg(feature = "kde_connect")]
            {
                draw_sync_content(ui, app, &theme);
            }

            #[cfg(not(feature = "kde_connect"))]
            {
                ui.label(
                    egui::RichText::new(t!("settings.sync.feature_disabled")).color(theme.muted),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new(t!("settings.sync.enable_hint")).color(theme.muted));
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

#[cfg(feature = "kde_connect")]
fn draw_sync_content(ui: &mut egui::Ui, app: &mut ClipboardApp, theme: &crate::ui::MacosTokens) {
    use crate::ui::settings::settings_footer_button;
    use crate::ui::widgets::macos_toggle;

    let changed = ui
        .horizontal(|ui| {
            ui.label(t!("settings.sync.enable"));
            macos_toggle(ui, &mut app.sync_enabled, theme)
        })
        .inner
        .changed();

    if changed {
        if app.sync_enabled {
            app.sync_manager_mut().enable();
        } else {
            app.sync_manager_mut().disable();
        }
        app.persist_preferences();
    }

    ui.add_space(12.0);

    let state_label = match app.sync_manager().state() {
        crate::sync::SyncState::Disabled => t!("settings.sync.state_disabled"),
        crate::sync::SyncState::Discovering => t!("settings.sync.state_discovering"),
        crate::sync::SyncState::Pairing { device_name } => {
            t!("settings.sync.state_pairing", name = device_name)
        }
        crate::sync::SyncState::Connected { device_name } => {
            t!("settings.sync.state_connected", name = device_name)
        }
        crate::sync::SyncState::Error(msg) => {
            t!("settings.sync.state_error", err = msg)
        }
    };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("settings.sync.status")).color(theme.muted));
        ui.label(egui::RichText::new(state_label.to_string()).color(theme.fg));
    });

    ui.add_space(8.0);

    let device_id = app.sync_manager().device_id().to_string();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(t!("settings.sync.device_id")).color(theme.muted));
        ui.monospace(&device_id);
    });

    ui.add_space(12.0);

    if settings_footer_button(ui, t!("settings.sync.show_qr"), theme, 160.0).clicked() {
        app.show_sync_qr = !app.show_sync_qr;
    }

    if app.show_sync_qr {
        ui.add_space(8.0);
        draw_qr_code(ui, &device_id, theme);
    }

    ui.add_space(12.0);

    ui.label(
        egui::RichText::new(t!("settings.sync.paired_devices"))
            .size(13.0)
            .strong()
            .color(theme.fg),
    );
    ui.add_space(4.0);

    let devices = app.sync_manager().discovered_devices().to_vec();
    if devices.is_empty() {
        ui.label(egui::RichText::new(t!("settings.sync.no_devices")).color(theme.muted));
    } else {
        for dev in &devices {
            ui.horizontal(|ui| {
                let status_icon = if dev.paired { "\u{2705}" } else { "\u{26aa}" };
                ui.label(status_icon);
                ui.label(egui::RichText::new(&dev.name).color(theme.fg));
                if dev.paired {
                    ui.label(egui::RichText::new(t!("settings.sync.paired")).color(theme.accent));
                }
            });
        }
    }
}

#[cfg(feature = "kde_connect")]
fn draw_qr_code(ui: &mut egui::Ui, device_id: &str, theme: &crate::ui::MacosTokens) {
    use qrcode::QrCode;
    use qrcode::render::unicode;

    let qr_data = format!("kdeconnect://{device_id}");
    match QrCode::new(qr_data.as_bytes()) {
        Ok(code) => {
            let image = code.render::<unicode::Dense1x2>().build();
            ui.label(
                egui::RichText::new(image)
                    .monospace()
                    .size(8.0)
                    .color(theme.fg),
            );
        }
        Err(e) => {
            ui.label(egui::RichText::new(format!("QR error: {e}")).color(theme.danger));
        }
    }
}
