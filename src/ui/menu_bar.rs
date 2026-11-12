//! Menu bar (File / Edit / View / Help), file shortcuts, the "unsaved changes" prompt
//! and the status bar.

use crate::app::{DAWApp, FileAction};
use egui::{Color32, Context, Key, KeyboardShortcut, Modifiers, RichText, Ui};

const NEW: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::N);
const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const SAVE_AS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);
const EXPORT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::E);

/// Ctrl+N / Ctrl+O / Ctrl+S / Ctrl+Shift+S / Ctrl+E.
pub fn handle_file_shortcuts(app: &mut DAWApp, ctx: &Context) {
    // Read (and consume) first, act after: the file dialogs block, and nothing may run
    // inside `ctx.input`. Save As before Save: Ctrl+S would also match Ctrl+Shift+S.
    let (new, open, save_as, save, export) = ctx.input_mut(|i| {
        (
            i.consume_shortcut(&NEW),
            i.consume_shortcut(&OPEN),
            i.consume_shortcut(&SAVE_AS),
            i.consume_shortcut(&SAVE),
            i.consume_shortcut(&EXPORT),
        )
    });
    if new {
        app.request_file_action(FileAction::New);
    }
    if open {
        app.request_file_action(FileAction::Open);
    }
    if save_as {
        app.save_project_as();
    } else if save {
        app.save_project();
    }
    if export {
        app.export_wav_dialog();
    }
}

pub fn show_menu_bar(app: &mut DAWApp, ui: &mut Ui) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| file_menu(app, ui));
        ui.menu_button("Edit", |ui| edit_menu(app, ui));
        ui.menu_button("View", |ui| view_menu(app, ui));
        ui.menu_button("Help", |ui| {
            if ui.button("User guide").clicked() {
                app.show_user_guide = true;
                ui.close_menu();
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let name = app.project_display_name();
            if app.is_dirty() {
                ui.label(RichText::new(format!("{name} • unsaved")).color(Color32::from_rgb(240, 190, 90)));
            } else {
                ui.label(RichText::new(name).color(Color32::GRAY));
            }
        });
    });
}

fn item(ui: &mut Ui, label: &str, shortcut: Option<&KeyboardShortcut>, enabled: bool) -> bool {
    let mut button = egui::Button::new(label);
    if let Some(s) = shortcut {
        button = button.shortcut_text(ui.ctx().format_shortcut(s));
    }
    let clicked = ui.add_enabled(enabled, button).clicked();
    if clicked {
        ui.close_menu();
    }
    clicked
}

fn file_menu(app: &mut DAWApp, ui: &mut Ui) {
    if item(ui, "New project", Some(&NEW), true) {
        app.request_file_action(FileAction::New);
    }
    if item(ui, "Open project…", Some(&OPEN), true) {
        app.request_file_action(FileAction::Open);
    }
    ui.separator();
    if item(ui, "Save", Some(&SAVE), true) {
        app.save_project();
    }
    if item(ui, "Save as…", Some(&SAVE_AS), true) {
        app.save_project_as();
    }
    ui.separator();
    if item(ui, "Export WAV…", Some(&EXPORT), true) {
        app.export_wav_dialog();
    }
}

fn edit_menu(app: &mut DAWApp, ui: &mut Ui) {
    let z = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
    let y = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
    if item(ui, "Undo", Some(&z), app.can_undo()) {
        app.undo();
    }
    if item(ui, "Redo", Some(&y), app.can_redo()) {
        app.redo();
    }
    ui.separator();
    let selected = app.selected_clip_indices().len();
    let c = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
    let v = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);
    let d = KeyboardShortcut::new(Modifiers::COMMAND, Key::D);
    let del = KeyboardShortcut::new(Modifiers::NONE, Key::Delete);
    if item(ui, "Copy clips", Some(&c), selected > 0) {
        if let Some(json) = app.copy_selected_clips() {
            ui.ctx().copy_text(json);
        }
    }
    if item(ui, "Paste clips at playhead", Some(&v), app.clip_clipboard.is_some()) {
        app.paste_clips_at_playhead();
    }
    if item(ui, "Duplicate clips", Some(&d), selected > 0) {
        app.duplicate_selected_clips();
    }
    if item(ui, "Move clips to playhead", None, selected > 0) {
        app.move_selected_clips_to_playhead();
    }
    if item(ui, "Delete clips", Some(&del), selected > 0) {
        app.delete_selected_clips();
    }
    if item(ui, "Merge clips into one", None, selected > 1) {
        app.merge_selected_clips();
    }
    if item(ui, "Select all clips on track", None, true) {
        app.select_all_clips();
    }
}

fn view_menu(app: &mut DAWApp, ui: &mut Ui) {
    let mut piano = app.show_piano || app.bottom_keyboard;
    if ui.checkbox(&mut piano, "Virtual piano").clicked() {
        app.toggle_piano();
        ui.close_menu();
    }
    let mut seq = app.show_drum_sequencer;
    if ui.checkbox(&mut seq, "Step sequencer (drum tracks)").clicked() {
        if app.show_drum_sequencer {
            app.close_drum_sequencer();
        } else {
            app.open_drum_sequencer();
        }
        ui.close_menu();
    }
    if ui.checkbox(&mut app.show_composition, "Composition assistant").clicked() {
        ui.close_menu();
    }
    if ui.checkbox(&mut app.show_instrument_designer, "Sound designer").clicked() {
        ui.close_menu();
    }
    let mut dj = app.show_dj_panel;
    if ui.checkbox(&mut dj, "DJ mode").clicked() {
        app.toggle_dj_mode();
        ui.close_menu();
    }
    let mut remote = app.show_remote;
    if ui.checkbox(&mut remote, "Phone remote").clicked() {
        app.toggle_remote();
        ui.close_menu();
    }
    ui.separator();
    if ui.button("Options…").clicked() {
        app.show_options = true;
        ui.close_menu();
    }
}

/// "Save changes?" before New / Open throws away unsaved work.
pub fn show_unsaved_prompt(app: &mut DAWApp, ctx: &Context) {
    let Some(action) = app.pending_file_action else { return };
    let mut open = true;
    egui::Window::new("Unsaved changes")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(format!("Save changes to \"{}\" first?", app.project_display_name()));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Save").strong()).clicked() {
                    app.pending_file_action = None;
                    app.save_project();
                    if !app.is_dirty() {
                        app.perform_file_action(action);
                    }
                }
                if ui.button("Don't save").clicked() {
                    app.pending_file_action = None;
                    app.perform_file_action(action);
                }
                if ui.button("Cancel").clicked() {
                    app.pending_file_action = None;
                }
            });
        });
    if !open {
        app.pending_file_action = None;
    }
}

/// Status line along the bottom of the window (messages used to sit at the end of the
/// transport row, where they were cut off).
pub fn show_status_bar(app: &mut DAWApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if let Some(err) = &app.status_error {
            ui.label(RichText::new(err).color(Color32::LIGHT_RED));
        } else if let Some(msg) = &app.status_message {
            ui.label(RichText::new(msg).color(Color32::LIGHT_GREEN));
        } else {
            ui.label(RichText::new("Ready").color(Color32::GRAY));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let path = app.project_file_path.clone().unwrap_or_else(|| "Not saved yet".to_string());
            ui.label(RichText::new(path).small().color(Color32::GRAY));
        });
    });
}
