use crate::app::DAWApp;
use crate::audio::engine::VelocityCurve;
use crate::music::theory::pitch_to_name;
use egui::{Color32, RichText, Sense, Ui, Vec2};

/// Draw a QR code for `text` with egui's painter (no image crate needed).
fn draw_qr(ui: &mut Ui, text: &str, size: f32) {
    let Ok(code) = qrcode::QrCode::new(text.as_bytes()) else {
        ui.label("(could not make QR code)");
        return;
    };
    let n = code.width();
    let quiet = 2;
    let cells = n + quiet * 2;
    let cell = (size / cells as f32).floor().max(2.0);
    let side = cell * cells as f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::WHITE);
    let colors = code.to_colors();
    for y in 0..n {
        for x in 0..n {
            if colors[y * n + x] == qrcode::Color::Dark {
                let min = rect.min
                    + Vec2::new((x + quiet) as f32 * cell, (y + quiet) as f32 * cell);
                painter.rect_filled(
                    egui::Rect::from_min_size(min, Vec2::splat(cell)),
                    0.0,
                    Color32::BLACK,
                );
            }
        }
    }
}

fn latency_color(ms: f32) -> Color32 {
    if ms < 15.0 {
        Color32::from_rgb(110, 220, 140)
    } else if ms < 40.0 {
        Color32::from_rgb(240, 200, 90)
    } else {
        Color32::from_rgb(240, 120, 110)
    }
}

pub fn show_remote_panel(app: &mut DAWApp, ui: &mut Ui) {
    ui.label(
        "Play this computer from an iPhone or iPad over your Wi-Fi. Notes go straight into \
         the audio engine — no internet, no account, nothing leaves your network.",
    );
    ui.add_space(4.0);

    if app.remote.is_none() {
        if ui
            .add(egui::Button::new(RichText::new("▶ Start remote keyboard").strong()).min_size(Vec2::new(220.0, 30.0)))
            .clicked()
        {
            app.start_remote();
        }
        if let Some(err) = &app.remote_error {
            ui.colored_label(Color32::LIGHT_RED, format!("Could not start: {err}"));
            ui.label("Another app may be using the port. Close it and try again.");
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Windows will ask whether to allow network access the first time — choose \
                 Private networks so your phone can connect.",
            )
            .small()
            .color(Color32::GRAY),
        );
        expression_settings(app, ui);
        return;
    }

    let (http_urls, https_urls, https_error, hostname, http_port) = {
        let s = app.remote.as_ref().unwrap();
        (
            s.http_urls(),
            s.https_urls(),
            s.https_error.clone(),
            s.hostname.clone(),
            s.http_port,
        )
    };

    ui.horizontal(|ui| {
        ui.colored_label(Color32::from_rgb(110, 220, 140), "● Running");
        if ui.button("Stop").clicked() {
            app.stop_remote();
        }
    });
    if app.remote.is_none() {
        return;
    }

    ui.separator();
    if let Some(url) = http_urls.first() {
        ui.horizontal(|ui| {
            draw_qr(ui, url, 150.0);
            ui.vertical(|ui| {
                ui.strong("1. Point your iPhone camera at the code");
                ui.label("or open this in Safari:");
                for u in &http_urls {
                    ui.horizontal(|ui| {
                        ui.monospace(u);
                        if ui.small_button("Copy").clicked() {
                            ui.ctx().copy_text(u.clone());
                        }
                    });
                }
                if let Some(h) = &hostname {
                    ui.label(
                        RichText::new(format!("(also try http://{h}.local:{http_port}/)"))
                            .small()
                            .color(Color32::GRAY),
                    );
                }
                ui.add_space(4.0);
                ui.strong("2. Share → Add to Home Screen");
                ui.label("for a full-screen app. Play in landscape.");
            });
        });
    } else {
        ui.colored_label(
            Color32::LIGHT_RED,
            "No network address found — is this computer connected to Wi-Fi/Ethernet?",
        );
    }

    ui.collapsing("Tap-strength velocity (HTTPS, optional)", |ui| {
        ui.label(
            "iOS only shares the accelerometer with secure pages. Over HTTPS the phone can \
             feel how hard you tap a key, for real velocity on glass.",
        );
        if let Some(err) = &https_error {
            ui.colored_label(Color32::LIGHT_RED, format!("HTTPS unavailable: {err}"));
        }
        for u in &https_urls {
            ui.horizontal(|ui| {
                ui.monospace(u);
                if ui.small_button("Copy").clicked() {
                    ui.ctx().copy_text(u.clone());
                }
            });
        }
        if let Some(url) = http_urls.first() {
            ui.label("One-time setup so Safari trusts this computer:");
            ui.label(format!("  a. On the phone open {url}ca.crt → Allow → Close"));
            ui.label("  b. Settings → Profile Downloaded → Install");
            ui.label("  c. Settings → General → About → Certificate Trust Settings → enable it");
            ui.label("  d. Open the HTTPS address, ⚙ → Enable tap strength");
            ui.label(
                RichText::new(
                    "The certificate is made on this computer and only vouches for it on your \
                     local network.",
                )
                .small()
                .color(Color32::GRAY),
            );
        }
    });

    ui.separator();
    let devices = app.remote.as_ref().map(|s| s.devices()).unwrap_or_default();
    ui.heading(format!("Devices ({})", devices.len()));
    if devices.is_empty() {
        ui.label(RichText::new("Waiting for a phone or tablet to connect…").color(Color32::GRAY));
    }
    let mut move_req = None;
    let mut identify_req = None;
    egui::Grid::new("remote_devices")
        .striped(true)
        .num_columns(5)
        .show(ui, |ui| {
            for (i, d) in devices.iter().enumerate() {
                ui.label(RichText::new(&d.name).strong());
                ui.label(format!("{:.0}×{:.0}", d.width, d.height));
                match d.rtt_ms {
                    Some(rtt) => {
                        let one_way = rtt / 2.0;
                        ui.colored_label(latency_color(one_way), format!("{one_way:.1} ms"))
                            .on_hover_text(format!(
                                "Network latency phone → computer (round trip {rtt:.1} ms). \
                                 Add one audio buffer (~10 ms) for total touch-to-sound."
                            ));
                    }
                    None => {
                        ui.label("…");
                    }
                }
                match d.layout {
                    Some(l) if l.chained => ui.label(format!(
                        "{}–{}",
                        pitch_to_name(l.start),
                        pitch_to_name(crate::remote::protocol::advance_whites(
                            l.start,
                            l.whites.saturating_sub(1) as u32
                        ))
                    )),
                    _ => ui.label(format!("{} notes", d.notes)),
                };
                ui.horizontal(|ui| {
                    if ui.small_button("Flash").on_hover_text("Flash this device's screen").clicked() {
                        identify_req = Some(d.id);
                    }
                    if app.remote_chain {
                        if ui.add_enabled(i > 0, egui::Button::new("◀").small()).clicked() {
                            move_req = Some((d.id, -1));
                        }
                        if ui
                            .add_enabled(i + 1 < devices.len(), egui::Button::new("▶").small())
                            .clicked()
                        {
                            move_req = Some((d.id, 1));
                        }
                    }
                });
                ui.end_row();
            }
        });
    if let Some(server) = &app.remote {
        if let Some(id) = identify_req {
            server.identify(id);
        }
        if let Some((id, delta)) = move_req {
            server.move_device(id, delta);
        }
    }

    ui.add_space(6.0);
    let mut chain = app.remote_chain;
    let mut start = app.remote_chain_start;
    ui.checkbox(&mut chain, "Chain devices into one long keyboard")
        .on_hover_text(
            "Line up several iPhones/iPads side by side. Keys are sized for the smallest \
             screen and continue from one device to the next.",
        );
    if chain {
        ui.horizontal(|ui| {
            ui.label("Lowest note:");
            let mut oct = (start / 12) as i32 - 1;
            if ui
                .add(egui::DragValue::new(&mut oct).range(0..=6).prefix("C"))
                .changed()
            {
                start = ((oct + 1) * 12) as u8;
            }
            ui.label("Use ◀ ▶ to put devices in left-to-right order; Flash to tell them apart.");
        });
    }
    if chain != app.remote_chain || start != app.remote_chain_start {
        app.remote_chain = chain;
        app.remote_chain_start = start;
        if let Some(server) = &app.remote {
            server.set_chain(chain, start);
        }
    }

    expression_settings(app, ui);

    ui.collapsing("Latency tips", |ui| {
        ui.label("• Put the computer on Ethernet and the phone on 5 GHz Wi-Fi.");
        ui.label("• Turn off Low Power Mode on the phone.");
        ui.label("• Under ~10 ms one-way feels instant; spikes usually mean weak Wi-Fi.");
        ui.label("• Audio adds one output buffer (~10 ms on Windows shared mode).");
    });
}

fn expression_settings(app: &mut DAWApp, ui: &mut Ui) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Touch velocity curve:");
        egui::ComboBox::from_id_salt("velocity_curve")
            .selected_text(app.velocity_curve.label())
            .show_ui(ui, |ui| {
                for &c in VelocityCurve::all() {
                    ui.selectable_value(&mut app.velocity_curve, c, c.label());
                }
            });
    });
    ui.label(
        RichText::new(
            "Applies to phones and the on-screen piano. Velocity changes tone as well as \
             volume; slide a held key (or drag on a held piano key) for pressure/expression.",
        )
        .small()
        .color(Color32::GRAY),
    );
}
