//! DJ mode: two platters, four channel strips, a master section and a crossfader, laid
//! out like a club mixer between two turntables. Shown in the bottom panel (under the
//! timeline, so recorded moves appear as automation lanes as you make them).

use crate::app::DAWApp;
use crate::dj::DjTarget;
use crate::model::automation::FxParam;
use crate::model::dj::{XfadeSide, DJ_CHANNELS};
use crate::ui::dj_widgets::{self as w, AutoState};
use crate::ui::theme;
use egui::{vec2, Color32, RichText, Ui};
use egui_phosphor::regular as ph;

/// Height the DJ mixer needs in the bottom panel.
pub const DJ_PANEL_HEIGHT: f32 = 370.0;

const STRIP_W: f32 = 112.0;
const MASTER_W: f32 = 100.0;
/// 33⅓ rpm, like a real turntable.
const RPM: f64 = 100.0 / 3.0;

fn track_color(app: &DAWApp, i: usize) -> Color32 {
    app.project
        .tracks
        .get(i)
        .map(|t| Color32::from_rgb(t.color[0], t.color[1], t.color[2]))
        .unwrap_or(w::LABEL)
}

impl DAWApp {
    /// Open DJ mode docked in the bottom panel, loading the first tracks onto the mixer
    /// if it's empty.
    pub fn open_dj_mode(&mut self) {
        self.show_dj_panel = true;
        self.dj.popped_out = false;
        self.bottom_keyboard = false;
        self.project.auto_assign_channels();
        self.sync_sequencer_visibility();
    }

    /// Leave DJ mode (docked or popped out); the bottom panel goes back to the editor.
    pub fn close_dj_mode(&mut self) {
        self.show_dj_panel = false;
        self.dj.popped_out = false;
        self.sync_sequencer_visibility();
    }

    pub fn toggle_dj_mode(&mut self) {
        if self.show_dj_panel {
            self.close_dj_mode();
        } else {
            self.open_dj_mode();
        }
    }

    /// DJ mode is showing in the bottom panel (not popped out).
    pub fn dj_docked(&self) -> bool {
        self.show_dj_panel && !self.dj.popped_out
    }

    /// Move DJ mode into its own window; the bottom panel shows the editor meanwhile.
    pub fn pop_out_dj_mode(&mut self) {
        self.show_dj_panel = true;
        self.dj.popped_out = true;
        self.sync_sequencer_visibility();
    }
}

pub fn show_dj_mode(app: &mut DAWApp, ui: &mut Ui) {
    header(app, ui);
    ui.add_space(4.0);

    let mut platter_held = false;
    let mut turns = 0.0_f32;

    egui::ScrollArea::horizontal()
        .id_salt("dj_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            egui::Frame::none()
                .fill(w::PANEL_BG)
                .rounding(8.0)
                .stroke(egui::Stroke::new(1.0, w::STRIP_EDGE))
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    let fixed = DJ_CHANNELS as f32 * (STRIP_W + 6.0) + MASTER_W + 60.0;
                    let platter_d = ((ui.available_width() - fixed) / 2.0).clamp(120.0, 230.0);
                    ui.horizontal_top(|ui| {
                        let (h, t) = deck(app, ui, XfadeSide::A, platter_d);
                        platter_held |= h;
                        turns += t;
                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.horizontal_top(|ui| {
                                // Each strip in its own vertical layout: a Frame inherits
                                // its parent's (horizontal) direction otherwise.
                                for ch in 0..DJ_CHANNELS {
                                    ui.vertical(|ui| channel_strip(app, ui, ch));
                                    ui.add_space(2.0);
                                }
                                ui.vertical(|ui| master_strip(app, ui));
                            });
                            ui.add_space(4.0);
                            crossfader_row(app, ui);
                        });
                        ui.add_space(6.0);
                        let (h, t) = deck(app, ui, XfadeSide::B, platter_d);
                        platter_held |= h;
                        turns += t;
                    });
                });
        });

    app.dj_platter_hold(platter_held);
    if turns != 0.0 {
        // One revolution at 33⅓ rpm covers this many beats at the song's tempo.
        let beats_per_rev = app.project.beats_per_second() * 60.0 / RPM;
        app.dj_scrub(turns as f64 * beats_per_rev);
    }
}

fn header(app: &mut DAWApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.heading(format!("{} DJ Mode", ph::VINYL_RECORD));
        ui.separator();

        let play_label = if app.playing { format!("{} Pause", ph::PAUSE) } else { format!("{} Play", ph::PLAY) };
        if ui.button(play_label).on_hover_text("Space").clicked() {
            app.toggle_playback();
        }

        let writing = app.dj.write;
        let write_btn = egui::Button::new(
            RichText::new(format!("{} WRITE", ph::CHART_LINE_UP))
                .strong()
                .color(if writing { Color32::WHITE } else { w::LABEL }),
        )
        .fill(if writing { Color32::from_rgb(170, 40, 40) } else { Color32::from_rgb(46, 47, 55) });
        if ui
            .add(write_btn)
            .on_hover_text(
                "WRITE: while the song plays, every knob, fader and crossfader move is recorded \
                 onto the timeline as automation, so the mix plays back the same way (and in \
                 WAV export). Turn it off to just play live.",
            )
            .clicked()
        {
            app.dj.write = !writing;
        }

        let rec = app.is_mix_recording();
        let rec_text = if rec {
            let s = app.mix_recording_secs();
            format!("{} STOP {:.0}:{:02.0}", ph::STOP, (s / 60.0).floor(), s % 60.0)
        } else {
            format!("{} REC MIX", ph::RECORD)
        };
        let rec_btn = egui::Button::new(RichText::new(rec_text).strong().color(Color32::WHITE)).fill(if rec {
            Color32::from_rgb(210, 40, 40)
        } else {
            Color32::from_rgb(120, 36, 40)
        });
        if ui
            .add(rec_btn)
            .on_hover_text(
                "Record what you hear — every track, effect and live move — to a WAV file. \
                 Starts playback if it's stopped. Press again to stop and save.",
            )
            .clicked()
        {
            app.toggle_mix_recording();
        }

        if app.playing && app.dj.write {
            ui.label(RichText::new("● writing automation").color(w::WRITING).strong());
        } else if let Some(p) = &app.dj.last_mixdown {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            ui.label(RichText::new(format!("Last mix: {name}")).small().color(w::LABEL))
                .on_hover_text(p.display().to_string());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button(format!("{} Tips", ph::LIGHTBULB), |ui| {
                ui.set_max_width(320.0);
                ui.label(RichText::new("Mixing in 5 steps").strong());
                for tip in [
                    "1. Load tracks onto the four channels (menu at the top of each strip).",
                    "2. Put each channel on A or B. The crossfader blends A ↔ B; THRU ignores it.",
                    "3. Turn LOW all the way down on one channel while the other plays its bass — two basslines at once sound muddy.",
                    "4. FILTER left = muffled (underwater), right = thin (telephone). Sweep it slowly into a drop.",
                    "5. ECHO then fade out = the classic 'echo out' ending.",
                ] {
                    ui.label(tip);
                }
                ui.separator();
                ui.label("Arm WRITE and press play to record your moves as automation — edit them afterwards in the lanes under each track (chart button in the track header).");
                ui.label("Double-click any knob to reset it. Right-click a knob to clear its automation.");
                ui.label("Grab a platter to hold and scrub the song like vinyl.");
            });
        });
    });
}

/// One turntable. Returns (held, turns this frame).
fn deck(app: &mut DAWApp, ui: &mut Ui, side: XfadeSide, diameter: f32) -> (bool, f32) {
    let on_deck: Vec<usize> = (0..DJ_CHANNELS)
        .filter_map(|ch| app.project.track_on_channel(ch))
        .filter(|&i| app.project.tracks[i].dj.side == side)
        .collect();
    let mut out = (false, 0.0);
    ui.vertical(|ui| {
        ui.set_width(diameter);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("DECK {}", side.label())).strong().color(Color32::WHITE));
            let names: Vec<String> = on_deck.iter().map(|&i| app.project.tracks[i].name.clone()).collect();
            ui.label(
                RichText::new(if names.is_empty() { "nothing on this side".to_string() } else { names.join(" + ") })
                    .small()
                    .color(w::LABEL),
            );
        });
        let (label, color) = on_deck
            .first()
            .map(|&i| (app.project.tracks[i].name.clone(), track_color(app, i)))
            .unwrap_or_else(|| (String::new(), Color32::from_rgb(70, 72, 80)));
        let secs = app.project.playhead_beat / app.project.beats_per_second().max(1e-6);
        let rotation = secs * RPM / 60.0;
        let (resp, input) = w::platter(ui, diameter, rotation, &label, color);
        resp.on_hover_text("Grab and turn to scrub: the song holds while your hand is on it, like vinyl.");
        out = (input.held, input.turns);
        ui.horizontal(|ui| {
            if ui
                .button(format!("{} CUE", ph::SKIP_BACK))
                .on_hover_text("Back to the start (Home)")
                .clicked()
            {
                app.go_to_start();
            }
            let label = if app.playing { ph::PAUSE } else { ph::PLAY };
            let play = egui::Button::new(RichText::new(label).size(16.0)).fill(if app.playing {
                Color32::from_rgb(40, 120, 70)
            } else {
                Color32::from_rgb(46, 47, 55)
            });
            if ui.add(play).on_hover_text("Play / pause (Space)").clicked() {
                app.toggle_playback();
            }
        });
    });
    out
}

/// A knob bound to a DJ target.
fn dj_knob(app: &mut DAWApp, ui: &mut Ui, target: DjTarget, diameter: f32, accent: Color32) {
    let DjTarget::Track(track, param) = target else { return };
    let mut v = app.dj_value(target);
    let auto = auto_state(app, target);
    let resp = w::knob(ui, &mut v, param.range(), param.neutral(), diameter, param.short(), accent, auto, |v| {
        format!("{}: {}", param.label(), param.describe(v))
    });
    if resp.dragged() || resp.changed() {
        app.dj_set(target, v);
    }
    control_menu(app, &resp, target, Some((track, param)));
}

fn auto_state(app: &DAWApp, target: DjTarget) -> AutoState {
    if app.playing && app.dj.write && app.dj.is_held(target) {
        AutoState::Writing
    } else if app.dj_has_automation(target) {
        AutoState::Automated
    } else {
        AutoState::None
    }
}

fn control_menu(app: &mut DAWApp, resp: &egui::Response, target: DjTarget, lane: Option<(usize, FxParam)>) {
    let automated = app.dj_has_automation(target);
    resp.context_menu(|ui| {
        if let Some((track, param)) = lane {
            if ui.button(format!("{} Show on timeline", ph::CHART_LINE)).clicked() {
                if let Some(t) = app.project.tracks.get_mut(track) {
                    t.lane_view = Some(param);
                }
                ui.close_menu();
            }
        }
        if ui
            .add_enabled(automated, egui::Button::new(format!("{} Clear automation", ph::TRASH)))
            .clicked()
        {
            app.dj_clear_automation(target);
            ui.close_menu();
        }
    });
}

fn channel_strip(app: &mut DAWApp, ui: &mut Ui, ch: usize) {
    let loaded = app.project.track_on_channel(ch);
    let accent = loaded.map(|i| track_color(app, i)).unwrap_or(w::LABEL);
    egui::Frame::none()
        .fill(w::STRIP_BG)
        .rounding(6.0)
        .stroke(egui::Stroke::new(1.0, w::STRIP_EDGE))
        .inner_margin(egui::Margin::symmetric(5.0, 5.0))
        .show(ui, |ui| {
            ui.set_width(STRIP_W - 10.0);
            ui.spacing_mut().item_spacing = vec2(2.0, 3.0);
            // Colour bar + channel number + picker.
            let (bar, _) = ui.allocate_exact_size(vec2(STRIP_W - 10.0, 3.0), egui::Sense::hover());
            ui.painter().rect_filled(bar, 1.0, accent);
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("CH{}", ch + 1)).small().strong().color(w::LABEL));
                if let Some(i) = loaded {
                    let icon = app.project.tracks[i].icon();
                    let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::hover());
                    w::paint_track_icon(&ui.painter_at(r), r, icon, accent);
                }
                track_picker(app, ui, ch, loaded);
            });
            let Some(i) = loaded else {
                ui.add_space(30.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Empty").color(w::LABEL));
                    ui.label(RichText::new("Pick a track above").small().color(w::LABEL));
                });
                ui.add_space(200.0);
                return;
            };

            // EQ row.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                for p in [FxParam::EqHigh, FxParam::EqMid, FxParam::EqLow] {
                    dj_knob(app, ui, DjTarget::Track(i, p), 28.0, Color32::from_rgb(120, 190, 255));
                }
            });
            // Filter + FX sends.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                dj_knob(app, ui, DjTarget::Track(i, FxParam::Filter), 40.0, Color32::from_rgb(190, 120, 255));
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        dj_knob(app, ui, DjTarget::Track(i, FxParam::Echo), 24.0, Color32::from_rgb(90, 220, 200));
                        dj_knob(app, ui, DjTarget::Track(i, FxParam::Reverb), 24.0, Color32::from_rgb(90, 220, 200));
                    });
                });
            });
            // Fader + meter.
            ui.horizontal(|ui| {
                let target = DjTarget::Track(i, FxParam::Volume);
                let mut v = app.dj_value(target);
                let auto = auto_state(app, target);
                ui.add_space(8.0);
                let resp = w::fader(ui, &mut v, 1.0, FxParam::Volume.neutral(), vec2(40.0, 118.0), accent, auto);
                if resp.dragged() || resp.changed() {
                    app.dj_set(target, v);
                }
                control_menu(app, &resp, target, Some((i, FxParam::Volume)));
                let level = app.dj.levels.get(i).copied().unwrap_or(0.0);
                w::vu_meter(ui, level, vec2(10.0, 118.0));
                ui.vertical(|ui| {
                    let muted = app.project.tracks[i].muted;
                    let m = egui::Button::new(RichText::new("M").small().strong().color(if muted {
                        Color32::BLACK
                    } else {
                        Color32::WHITE
                    }))
                    .fill(if muted { Color32::from_rgb(240, 190, 60) } else { Color32::from_rgb(55, 55, 64) })
                    .min_size(vec2(22.0, 18.0));
                    if ui.add(m).on_hover_text("Mute this channel").clicked() {
                        app.project.tracks[i].muted = !muted;
                    }
                    let selected = app.current_track == i;
                    let s = egui::Button::new(RichText::new(ph::PIANO_KEYS).small())
                        .fill(if selected { mix_accent(accent) } else { Color32::from_rgb(55, 55, 64) })
                        .min_size(vec2(22.0, 18.0));
                    if ui
                        .add(s)
                        .on_hover_text("Play this channel live from the piano / phone (selects the track)")
                        .clicked()
                    {
                        app.select_track(i);
                    }
                });
            });
            // Crossfader assign.
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.0;
                for side in [XfadeSide::A, XfadeSide::Thru, XfadeSide::B] {
                    let on = app.project.tracks[i].dj.side == side;
                    let b = egui::Button::new(RichText::new(side.label()).small().strong().color(if on {
                        Color32::BLACK
                    } else {
                        w::LABEL
                    }))
                    .fill(if on { Color32::from_rgb(240, 240, 245) } else { Color32::from_rgb(46, 47, 55) })
                    .min_size(vec2(if side == XfadeSide::Thru { 38.0 } else { 30.0 }, 18.0));
                    if ui
                        .add(b)
                        .on_hover_text("Crossfader side: A = left, B = right, THRU = not affected")
                        .clicked()
                    {
                        app.project.tracks[i].dj.side = side;
                    }
                }
            });
        });
}

fn mix_accent(c: Color32) -> Color32 {
    w::mix(Color32::from_rgb(55, 55, 64), c, 0.6)
}

fn track_picker(app: &mut DAWApp, ui: &mut Ui, ch: usize, loaded: Option<usize>) {
    let current = loaded
        .map(|i| app.project.tracks[i].name.clone())
        .unwrap_or_else(|| "— empty —".to_string());
    let mut choice = loaded;
    egui::ComboBox::from_id_salt(("dj_ch", ch))
        .width(STRIP_W - 50.0)
        .selected_text(RichText::new(current).small())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut choice, None, "— empty —");
            for (i, t) in app.project.tracks.iter().enumerate() {
                let on_other = t.dj.channel.filter(|&c| c as usize != ch);
                let label = match on_other {
                    Some(c) => format!("{}  (on CH{})", t.name, c + 1),
                    None => t.name.clone(),
                };
                ui.selectable_value(&mut choice, Some(i), label);
            }
        });
    if choice != loaded {
        app.project.assign_channel(ch, choice);
    }
}

fn master_strip(app: &mut DAWApp, ui: &mut Ui) {
    egui::Frame::none()
        .fill(Color32::from_rgb(38, 36, 44))
        .rounding(6.0)
        .stroke(egui::Stroke::new(1.0, w::STRIP_EDGE))
        .inner_margin(egui::Margin::symmetric(5.0, 5.0))
        .show(ui, |ui| {
            ui.set_width(MASTER_W - 10.0);
            ui.spacing_mut().item_spacing = vec2(2.0, 3.0);
            ui.label(RichText::new("MASTER").small().strong().color(Color32::WHITE));
            let fx = &mut app.audio.master_effects;
            let accent = Color32::from_rgb(255, 200, 90);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                w::knob(ui, &mut fx.lowpass, (0.0, 1.0), 1.0, 26.0, "TONE", accent, AutoState::None, |v| {
                    format!("Brightness {:.0}% (lower = muffled)", v * 100.0)
                });
                w::knob(ui, &mut fx.reverb, (0.0, 1.0), 0.0, 26.0, "VERB", accent, AutoState::None, |v| {
                    format!("Master reverb {:.0}%", v * 100.0)
                });
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                w::knob(ui, &mut fx.distance, (0.0, 1.0), 0.0, 26.0, "FAR", accent, AutoState::None, |v| {
                    format!("Distance / underwater {:.0}%", v * 100.0)
                });
                w::knob(ui, &mut fx.phaser, (0.0, 1.0), 0.0, 26.0, "PHASE", accent, AutoState::None, |v| {
                    format!("Phaser swirl {:.0}%", v * 100.0)
                });
            });
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                w::fader(ui, &mut app.audio.master_volume, 1.0, 0.8, vec2(36.0, 104.0), accent, AutoState::None);
                let level = app.dj.master_level;
                w::vu_meter(ui, level, vec2(9.0, 104.0));
                w::vu_meter(ui, level, vec2(9.0, 104.0));
            });
        });
}

fn crossfader_row(app: &mut DAWApp, ui: &mut Ui) {
    let width = DJ_CHANNELS as f32 * (STRIP_W + 2.0) - 20.0;
    ui.horizontal(|ui| {
        let bars = app.dj_fade_bars();
        if ui
            .button(RichText::new(format!("{} A", ph::ARROW_LEFT)).strong())
            .on_hover_text(format!(
                "Auto-fade to A over {bars} bars, starting at the playhead (written as automation)"
            ))
            .clicked()
        {
            app.dj_auto_crossfade(-1.0, bars as f64);
        }
        let target = DjTarget::Crossfader;
        let mut v = app.dj_value(target);
        let resp = w::crossfader(ui, &mut v, (width - 150.0).max(160.0), auto_state(app, target));
        if resp.dragged() || resp.changed() {
            app.dj_set(target, v);
        }
        control_menu(app, &resp, target, None);
        if ui
            .button(RichText::new(format!("B {}", ph::ARROW_RIGHT)).strong())
            .on_hover_text(format!(
                "Auto-fade to B over {bars} bars, starting at the playhead (written as automation)"
            ))
            .clicked()
        {
            app.dj_auto_crossfade(1.0, bars as f64);
        }
        let mut b = bars;
        egui::ComboBox::from_id_salt("dj_fade_bars")
            .width(64.0)
            .selected_text(RichText::new(format!("{b} bars")).small())
            .show_ui(ui, |ui| {
                for n in [1u32, 2, 4, 8, 16] {
                    ui.selectable_value(&mut b, n, format!("{n} bars"));
                }
            });
        app.set_dj_fade_bars(b);
    });
    ui.label(
        RichText::new("Crossfader · right-click to clear its automation")
            .font(theme::font(theme::FONT_XS))
            .color(w::LABEL),
    );
}

impl DAWApp {
    fn dj_fade_bars(&self) -> u32 {
        if self.dj.fade_bars == 0 {
            4
        } else {
            self.dj.fade_bars
        }
    }

    fn set_dj_fade_bars(&mut self, bars: u32) {
        self.dj.fade_bars = bars.max(1);
    }
}
