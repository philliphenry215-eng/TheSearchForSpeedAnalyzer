use eframe::egui;
use egui_extras::{Column, TableBuilder};
use crate::{display_font, with_alpha, WarRoomApp};
use crate::formatters::*;
use crate::models::*;
use crate::settings::*;
use crate::settings;
use crate::{load_logo_texture, setup_custom_style};
use crate::telemetry::is_real_track_name;
use crate::views::*;
use crate::stint_db;
use crate::updater;

fn render_color_picker_row(
    ui: &mut egui::Ui,
    name: &str,
    color: &mut [u8; 3],
    tooltip: &str,
    text_color: egui::Color32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        if ui.color_edit_button_srgb(color).on_hover_text(tooltip).changed() {
            changed = true;
        }
        ui.label(egui::RichText::new(name).color(text_color).size(11.5));
    });
    ui.add_space(2.0);
    changed
}

impl WarRoomApp {
    pub(crate) fn render_telemetry_debugger(&mut self, ctx: &egui::Context, s: &WarRoomState) {
        if !self.show_telemetry_debugger { return; }

        let primary = self.settings.primary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let green = self.settings.status_green();

        let player = s.drivers.get(&s.player_car_idx);
        let active_stint = player.and_then(|p| p.stints.iter().rev().find(|st| st.is_active).or_else(|| p.stints.last()));

        egui::Window::new("Live Telemetry & Stint Inspector")
            .fixed_size([620.0, 560.0])
            .anchor(egui::Align2::RIGHT_BOTTOM, [-16.0, -16.0])
            .collapsible(true)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("RAW SDK VARIABLES · 60 HZ").color(muted).size(11.0).extra_letter_spacing(0.8));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("× Close").color(white).size(12.0)).fill(elevated)).clicked() {
                            self.show_telemetry_debugger = false;
                        }
                    });
                });
                ui.add_space(4.0);

                egui::Frame::none()
                    .fill(elevated)
                    .rounding(egui::Rounding::same(10.0))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        egui::Grid::new("raw_telemetry_grid").num_columns(4).spacing([18.0, 8.0]).show(ui, |ui| {
                            ui.label(egui::RichText::new("Lap").color(muted));
                            ui.label(egui::RichText::new(format!("{}", s.debug_lap_number)).color(white).strong());
                            
                            ui.label(egui::RichText::new("LapCompleted").color(muted));
                            ui.label(egui::RichText::new(format!("{}", s.debug_lap_completed)).color(white).strong());
                            ui.end_row();

                            ui.label(egui::RichText::new("LapCurrentLapTime").color(muted));
                            ui.label(egui::RichText::new(format_time(s.debug_lap_current_time)).color(white));

                            ui.label(egui::RichText::new("LapLastLapTime").color(muted));
                            ui.label(egui::RichText::new(format_time(s.debug_lap_last_time)).color(primary).strong());
                            ui.end_row();

                            ui.label(egui::RichText::new("LapBestLapTime").color(muted));
                            ui.label(egui::RichText::new(format_time(s.debug_lap_best_time)).color(green));

                            ui.label(egui::RichText::new("LapDistPct").color(muted));
                            ui.label(egui::RichText::new(format!("{:.1}% ({:.0}m)", s.player_lap_distance_pct * 100.0, s.debug_lap_dist)).color(white));
                            ui.end_row();

                            ui.label(egui::RichText::new("CarIdxOnPitRoad").color(muted));
                            let pit_text = if s.debug_on_pit_road { "In Pit Road" } else { "On Track" };
                            let pit_col = if s.debug_on_pit_road { self.settings.status_amber() } else { green };
                            ui.label(egui::RichText::new(pit_text).color(pit_col).strong());

                            ui.label(egui::RichText::new("TrackSurface").color(muted));
                            let surf_desc = match s.debug_track_surface {
                                0 => "0 NotInWorld",
                                1 => "1 OffTrack",
                                2 => "2 InPitStall",
                                3 => "3 Aprox",
                                4 => "4 OnTrack",
                                _ => "Unknown",
                            };
                            ui.label(egui::RichText::new(surf_desc).color(white));
                            ui.end_row();

                            ui.label(egui::RichText::new("Speed / Gear").color(muted));
                            ui.label(egui::RichText::new(format!("{:.0} mph · Gear {}", s.debug_speed, s.debug_gear)).color(white));

                            ui.label(egui::RichText::new("Fuel").color(muted));
                            let dbg_fuel = if self.settings.use_metric {
                                format!("{:.2} L ({:.1}%)", s.player_fuel_level, s.player_fuel_pct * 100.0)
                            } else {
                                format!("{:.2} gal ({:.1}%)", s.player_fuel_level * 0.264172, s.player_fuel_pct * 100.0)
                            };
                            ui.label(egui::RichText::new(dbg_fuel).color(white));
                            ui.end_row();

                            ui.label(egui::RichText::new("Tires (Pit)").color(muted));
                            let tire_summary = if s.latest_tire_data.has_data {
                                format!("LF {:.0}% ({:.1} psi) · RF {:.0}% ({:.1} psi) · LR {:.0}% · RR {:.0}%",
                                    s.latest_tire_data.lf.wear_m, s.latest_tire_data.lf.pressure * 0.145038,
                                    s.latest_tire_data.rf.wear_m, s.latest_tire_data.rf.pressure * 0.145038,
                                    s.latest_tire_data.lr.wear_m, s.latest_tire_data.rr.wear_m)
                            } else {
                                "Awaiting pit entry".to_string()
                            };
                            ui.label(egui::RichText::new(tire_summary).color(white));
                            ui.end_row();
                        });
                    });

                ui.add_space(10.0);

                // Live Active Stint Summary
                ui.label(egui::RichText::new("ACTIVE STINT").color(muted).size(11.0).extra_letter_spacing(0.8));
                if let Some(st) = active_stint {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Stint S{}", st.stint_number)).color(primary).strong());
                        ui.label(egui::RichText::new(format!("· Start lap {} · {} laps · Best {}", st.start_lap, st.laps_completed, format_time(st.fast_lap))).color(white));
                    });
                    ui.label(egui::RichText::new(format!("Laps: {:?}", st.lap_times.iter().map(|t| format!("{:.3}", *t)).collect::<Vec<String>>())).color(muted).size(12.0));
                } else {
                    ui.label(egui::RichText::new("No active stint running.").color(muted));
                }

                ui.add_space(10.0);

                // Live Event Stream
                ui.label(egui::RichText::new("EVENT STREAM").color(muted).size(11.0).extra_letter_spacing(0.8));
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(12, 14, 20))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(130.0).stick_to_bottom(true).show(ui, |ui| {
                            if s.debug_event_logs.is_empty() {
                                ui.label(egui::RichText::new("Waiting for track events (pit exit, lap line crossing)...").color(muted).small());
                            } else {
                                for log in &s.debug_event_logs {
                                    ui.label(egui::RichText::new(log).color(white).size(12.0).monospace());
                                }
                            }
                        });
                    });
            });
    }

    pub(crate) fn render_release_notes_modal(&mut self, ctx: &egui::Context) {
        if !self.show_release_notes_modal {
            return;
        }

        let primary = self.settings.primary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let panel_bg = self.settings.panel_color();
        let border = self.settings.divider_color();

        let embedded_notes = include_str!("../../RELEASE_NOTES.md");
        let (display_title, display_body, html_url) = if let Some(ref custom) = self.release_notes_custom_content {
            ("Release Notes".to_string(), custom.clone(), "https://github.com/philliphenry215-eng/TheSearchForSpeedAnalyzer/releases".to_string())
        } else if let updater::UpdateStatus::UpdateAvailable(info) = &self.updater.status {
            (info.name.clone(), if !info.body.is_empty() { info.body.clone() } else { embedded_notes.to_string() }, info.html_url.clone())
        } else {
            (format!("What's New in v{}", updater::CURRENT_VERSION), embedded_notes.to_string(), format!("https://github.com/philliphenry215-eng/TheSearchForSpeedAnalyzer/releases/tag/v{}", updater::CURRENT_VERSION))
        };

        egui::Window::new("📖 Release Notes & What's New")
            .fixed_size([580.0, 620.0])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .frame(egui::Frame::window(&ctx.style()).fill(panel_bg).stroke(egui::Stroke::new(1.0, border)).rounding(10.0).inner_margin(16.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(display_title).font(display_font(18.0)).color(primary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Open on GitHub ↗").color(white).size(12.0)).fill(elevated)).clicked() {
                            updater::open_browser(&html_url);
                        }
                    });
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .max_height(480.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(540.0);
                        for line in display_body.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("# ") {
                                ui.add_space(8.0);
                                ui.add(egui::Label::new(egui::RichText::new(trimmed.trim_start_matches("# ").trim()).font(display_font(17.0)).color(primary)).wrap(true));
                            } else if trimmed.starts_with("## ") {
                                ui.add_space(6.0);
                                ui.add(egui::Label::new(egui::RichText::new(trimmed.trim_start_matches("## ").trim()).font(display_font(15.0)).color(primary)).wrap(true));
                            } else if trimmed.starts_with("### ") {
                                ui.add_space(5.0);
                                ui.add(egui::Label::new(egui::RichText::new(trimmed.trim_start_matches("### ").trim()).color(white).strong().size(13.5)).wrap(true));
                            } else if trimmed.starts_with("#### ") {
                                ui.add_space(4.0);
                                ui.add(egui::Label::new(egui::RichText::new(trimmed.trim_start_matches("#### ").trim()).color(primary).strong().size(12.5)).wrap(true));
                            } else if trimmed.starts_with("* ") || trimmed.starts_with("- ") {
                                let bullet = trimmed[2..].trim();
                                ui.add(egui::Label::new(
                                    egui::RichText::new(format!("  •  {}", bullet)).color(white).size(12.0)
                                ).wrap(true));
                            } else if trimmed.is_empty() {
                                ui.add_space(4.0);
                            } else if trimmed == "---" {
                                ui.separator();
                            } else {
                                ui.add(egui::Label::new(egui::RichText::new(trimmed).color(muted).size(12.0)).wrap(true));
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Installed Version: v{}", updater::CURRENT_VERSION)).color(muted).size(11.5));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized(egui::vec2(90.0, 28.0), egui::Button::new(egui::RichText::new("Done").color(egui::Color32::BLACK).strong()).fill(primary)).clicked() {
                            self.show_release_notes_modal = false;
                            self.release_notes_custom_content = None;
                        }

                        let updater_status = self.updater.status.clone();
                        match &updater_status {
                            updater::UpdateStatus::UpdateAvailable(info) => {
                                let info_clone = info.clone();
                                let update_btn = egui::Button::new(
                                    egui::RichText::new("Update Now ⚡").color(egui::Color32::BLACK).strong().size(12.0)
                                ).fill(primary);
                                if ui.add(update_btn).clicked() {
                                    self.updater.trigger_download_and_install(&info_clone);
                                }
                            }
                            updater::UpdateStatus::Downloading { percent, downloaded_bytes, total_bytes, .. } => {
                                let mb_down = *downloaded_bytes as f32 / (1024.0 * 1024.0);
                                let mb_tot = *total_bytes as f32 / (1024.0 * 1024.0);
                                ui.label(egui::RichText::new(format!("Downloading: {:.0}% ({:.1}/{:.1} MB)", percent, mb_down, mb_tot)).color(primary).size(12.0));
                            }
                            updater::UpdateStatus::ReadyToRestart { .. } => {
                                let restart_btn = egui::Button::new(
                                    egui::RichText::new("Restart to Apply 🔄").color(egui::Color32::BLACK).strong().size(12.0)
                                ).fill(self.settings.status_green());
                                if ui.add(restart_btn).clicked() {
                                    if let Err(e) = updater::apply_update_and_restart() { self.updater.status = updater::UpdateStatus::Error(e); }
                                }
                            }
                            _ => {}
                        }
                    });
                });
            });
    }

    pub(crate) fn render_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings_modal { return; }

        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();

        let screen_rect = ctx.screen_rect();
        let win_w = 700.0f32.min(screen_rect.width() - 40.0);
        let win_h = 580.0f32.min(screen_rect.height() - 40.0);
        let mut is_open = self.show_settings_modal;
        let mut close_modal = false;

        egui::Window::new("⚙ Engineering & App Settings")
            .id(egui::Id::new("app_settings_modal_window"))
            .open(&mut is_open)
            .fixed_size([win_w, win_h])
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_modal = true;
                }

                // TOP HEADER: Section Title & Quick Close
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("CONFIGURATION & PREFERENCES")
                            .color(muted)
                            .size(11.0)
                            .extra_letter_spacing(0.8),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized(
                            egui::vec2(76.0, 24.0),
                            egui::Button::new(
                                egui::RichText::new("Done ✓")
                                    .color(egui::Color32::BLACK)
                                    .strong()
                                    .size(11.5),
                            ).fill(primary).rounding(egui::Rounding::same(5.0)),
                        ).clicked() {
                            close_modal = true;
                        }
                    });
                });

                ui.add_space(4.0);

                // TOP TAB BAR: Horizontal Row of Category Tabs
                ui.horizontal_wrapped(|ui| {
                    let tabs = [
                        (SettingsTab::General, "🏁 General"),
                        (SettingsTab::Telemetry, "📈 Telemetry (.IBT)"),
                        (SettingsTab::OverlayHud, "🖥 In-Car HUD"),
                        (SettingsTab::TirePopup, "◎ Tire Popups"),
                        (SettingsTab::GridColumns, "📊 Columns & Avgs"),
                        (SettingsTab::Themes, "🎨 Themes"),
                        (SettingsTab::Updates, "🔄 Updates"),
                    ];

                    for (tab, label) in tabs {
                        let is_active = self.settings_active_tab == tab;
                        let btn = egui::Button::new(
                            egui::RichText::new(label)
                                .color(if is_active { primary } else { white })
                                .size(11.5)
                                .strong(),
                        )
                        .fill(if is_active { with_alpha(primary, 40) } else { elevated })
                        .stroke(if is_active { egui::Stroke::new(1.2, primary) } else { egui::Stroke::NONE })
                        .rounding(egui::Rounding::same(5.0));

                        if ui.add(btn).clicked() {
                            self.settings_active_tab = tab;
                        }
                    }
                });

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(6.0);

                // MAIN CONTENT AREA: Vertical ScrollArea with explicit top_down layout
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(win_h - 130.0)
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            match self.settings_active_tab {
                                SettingsTab::General => {
                                    self.render_settings_general_tab(ui, primary, white, muted, elevated);
                                }
                                SettingsTab::Telemetry => {
                                    self.render_settings_telemetry_tab(ui, primary, secondary, white, muted, elevated);
                                }
                                SettingsTab::OverlayHud => {
                                    self.render_settings_overlay_tab(ui, white, muted, elevated);
                                }
                                SettingsTab::TirePopup => {
                                    self.render_settings_tire_popup_tab(ui, white, muted, elevated);
                                }
                                SettingsTab::GridColumns => {
                                    self.render_settings_grid_columns_tab(ui, secondary, white, muted, elevated);
                                }
                                SettingsTab::Themes => {
                                    self.render_settings_themes_tab(ui, ctx, primary, secondary, white, muted, elevated);
                                }
                                SettingsTab::Updates => {
                                    self.render_settings_updates_tab(ui, primary, white, muted, elevated);
                                }
                            }
                            ui.add_space(16.0);
                        });
                    });

                // BOTTOM STATUS & CLOSE BAR
                ui.separator();
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("S4S Analyzer v{}", updater::CURRENT_VERSION))
                            .color(muted)
                            .size(10.5),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized(
                            egui::vec2(90.0, 26.0),
                            egui::Button::new(
                                egui::RichText::new("Save & Close")
                                    .color(egui::Color32::BLACK)
                                    .strong()
                                    .size(11.5),
                            ).fill(primary).rounding(egui::Rounding::same(5.0)),
                        ).clicked() {
                            close_modal = true;
                        }
                    });
                });
            });

        if !is_open || close_modal {
            self.settings.save();
            self.show_settings_modal = false;
        }
    }

    fn render_settings_general_tab(
        &mut self,
        ui: &mut egui::Ui,
        primary: egui::Color32,
        white: egui::Color32,
        muted: egui::Color32,
        elevated: egui::Color32,
    ) {
        ui.label(egui::RichText::new("GENERAL & STARTUP CONFIGURATION").color(muted).size(11.0).extra_letter_spacing(0.8));
        ui.add_space(6.0);

        // Section 1: Default Startup View
        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Default Startup Workspace").color(white).size(13.0).strong());
                ui.label(egui::RichText::new("Choose which screen opens automatically when the application launches:").color(muted).size(11.0));
                ui.add_space(8.0);

                let views = [
                    (ActiveViewMode::CockpitEngineer, "★ Cockpit Stint & Fuel Analyzer", "Real-time live telemetry, stint pacing, fuel strategy, tires"),
                    (ActiveViewMode::GridTable, "📊 Multi-Driver Stint Grid", "Multi-car standings, stint history, sector pace comparison"),
                    (ActiveViewMode::IbtTelemetry, "📈 Offline .IBT Telemetry Lab", "Deep telemetry analysis, lap comparison, ride heights, exports"),
                ];

                for (mode, label, desc) in views {
                    let is_sel = self.settings.default_startup_view == mode;
                    let btn = egui::Button::new(
                        egui::RichText::new(label)
                            .color(if is_sel { primary } else { white })
                            .strong()
                            .size(12.0),
                    )
                    .fill(if is_sel { with_alpha(primary, 35) } else { egui::Color32::from_rgba_unmultiplied(255, 255, 255, 6) })
                    .stroke(if is_sel { egui::Stroke::new(1.0, primary) } else { egui::Stroke::NONE })
                    .rounding(egui::Rounding::same(5.0));

                    if ui.add_sized(egui::vec2(ui.available_width(), 28.0), btn).clicked() {
                        self.settings.default_startup_view = mode;
                        self.settings.view_mode = mode;
                        self.settings.save();
                    }
                    ui.label(egui::RichText::new(desc).color(muted).size(10.5));
                    ui.add_space(6.0);
                }
            });

        ui.add_space(8.0);

        // Section 2: Measurement Units
        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Measurement Units").color(white).size(13.0).strong());
                ui.label(egui::RichText::new("Select imperial or metric units across HUD, telemetry, and tables:").color(muted).size(11.0));
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.selectable_label(!self.settings.use_metric, "🇺🇸 Imperial (mph, °F, gal, psi)").clicked() {
                        self.settings.use_metric = false;
                        self.settings.save();
                    }
                    ui.add_space(8.0);
                    if ui.selectable_label(self.settings.use_metric, "🌐 Metric (km/h, °C, L, kPa)").clicked() {
                        self.settings.use_metric = true;
                        self.settings.save();
                    }
                });
            });

        ui.add_space(8.0);

        // Section 3: Layout Presets
        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Grid & Screen Layout Presets").color(white).size(13.0).strong());
                ui.label(egui::RichText::new("Quickly configure column sets and card layouts for your current racing discipline:").color(muted).size(11.0));
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    if ui.selectable_label(self.settings.layout_preset == LayoutPreset::FullStint, "Full Stint").clicked() {
                        self.settings.apply_preset(LayoutPreset::FullStint);
                    }
                    if ui.selectable_label(self.settings.layout_preset == LayoutPreset::EnduranceEngineer, "Endurance Engineer").clicked() {
                        self.settings.apply_preset(LayoutPreset::EnduranceEngineer);
                    }
                    if ui.selectable_label(self.settings.layout_preset == LayoutPreset::SprintQualifying, "Sprint / Qualy").clicked() {
                        self.settings.apply_preset(LayoutPreset::SprintQualifying);
                    }
                    if ui.selectable_label(self.settings.layout_preset == LayoutPreset::Minimalist, "Minimalist").clicked() {
                        self.settings.apply_preset(LayoutPreset::Minimalist);
                    }
                });
            });
    }

    fn render_settings_telemetry_tab(
        &mut self,
        ui: &mut egui::Ui,
        primary: egui::Color32,
        secondary: egui::Color32,
        white: egui::Color32,
        muted: egui::Color32,
        elevated: egui::Color32,
    ) {
        ui.label(egui::RichText::new("OFFLINE .IBT TELEMETRY CUSTOMIZATION").color(muted).size(11.0).extra_letter_spacing(0.8));
        ui.add_space(6.0);

        // Card 1: Comparison Line Rendering (User Request: "opposing or compare lines different colors rather than a shaded version")
        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Comparison & Opposing Lap Style (Lap B)").color(white).size(13.0).strong());
                ui.label(egui::RichText::new("Choose how reference / comparison laps are rendered alongside primary Lap A traces:").color(muted).size(11.0));
                ui.add_space(8.0);

                let mut style_changed = false;

                ui.vertical(|ui| {
                    if ui.radio_value(&mut self.settings.telemetry.compare_style, CompareLineStyle::DistinctColor, "🎨 Distinct High-Contrast Colors (Recommended)").changed() {
                        style_changed = true;
                    }
                    ui.label(egui::RichText::new("    Uses distinct vibrant colors for Lap B channels (e.g. Cyan/Amber vs Primary) for maximum visibility.").color(muted).size(10.5));
                    ui.add_space(4.0);

                    if ui.radio_value(&mut self.settings.telemetry.compare_style, CompareLineStyle::DashedLine, "--- Dashed Reference Line").changed() {
                        style_changed = true;
                    }
                    ui.label(egui::RichText::new("    Renders Lap B with a high-contrast dashed line stroke, making trace shapes instantly distinguishable even on overlapping lines.").color(muted).size(10.5));
                    ui.add_space(4.0);

                    if ui.radio_value(&mut self.settings.telemetry.compare_style, CompareLineStyle::ShadedAlpha, "~ Shaded Semi-Transparent (Legacy)").changed() {
                        style_changed = true;
                    }
                    ui.label(egui::RichText::new("    Renders Lap B with semi-transparent alpha shading of Lap A's color.").color(muted).size(10.5));
                });

                if style_changed {
                    self.settings.save();
                }
            });

        ui.add_space(8.0);

        // Card 2: Trace Colors (Lap A & Lap B Channels)
        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Telemetry Trace Channel Colors").color(white).size(13.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Reset to Defaults").color(muted).size(11.0)).fill(egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10))).clicked() {
                            self.settings.telemetry.trace_colors = TelemetryTraceColors::default();
                            self.settings.save();
                        }
                    });
                });
                ui.label(egui::RichText::new("Click any color swatch to open the interactive color picker wheel:").color(muted).size(11.0));
                ui.add_space(8.0);

                let mut color_changed = false;

                ui.columns(2, |cols| {
                    // Column 1: Lap A (Primary)
                    cols[0].vertical(|ui| {
                        ui.label(egui::RichText::new("PRIMARY SESSION (LAP A)").color(primary).size(11.0).strong());
                        ui.add_space(4.0);

                        color_changed |= render_color_picker_row(ui, "Speed / Base", &mut self.settings.telemetry.trace_colors.lap_a_primary, "Vehicle speed & base accent", white);
                        color_changed |= render_color_picker_row(ui, "Throttle", &mut self.settings.telemetry.trace_colors.throttle_a, "0 - 100% pedal trace", white);
                        color_changed |= render_color_picker_row(ui, "Brake", &mut self.settings.telemetry.trace_colors.brake_a, "0 - 100% pedal pressure", white);
                        color_changed |= render_color_picker_row(ui, "Steering", &mut self.settings.telemetry.trace_colors.steering_a, "Steering wheel angle", white);
                    });

                    // Column 2: Lap B (Comparison / Opposing Lap)
                    cols[1].vertical(|ui| {
                        ui.label(egui::RichText::new("REFERENCE SESSION (LAP B)").color(secondary).size(11.0).strong());
                        ui.add_space(4.0);

                        color_changed |= render_color_picker_row(ui, "Speed (Ref)", &mut self.settings.telemetry.trace_colors.lap_b_reference, "Reference comparison speed", white);
                        color_changed |= render_color_picker_row(ui, "Throttle (Ref)", &mut self.settings.telemetry.trace_colors.throttle_b, "Reference throttle comparison", white);
                        color_changed |= render_color_picker_row(ui, "Brake (Ref)", &mut self.settings.telemetry.trace_colors.brake_b, "Reference brake comparison", white);
                        color_changed |= render_color_picker_row(ui, "Steering (Ref)", &mut self.settings.telemetry.trace_colors.steering_b, "Reference steering comparison", white);
                        ui.add_space(4.0);
                        color_changed |= render_color_picker_row(ui, "Delta Faster", &mut self.settings.telemetry.trace_colors.delta_ahead, "Lap A is faster / gaining", white);
                        color_changed |= render_color_picker_row(ui, "Delta Slower", &mut self.settings.telemetry.trace_colors.delta_behind, "Lap A is slower / losing", white);
                    });
                });

                if color_changed {
                    self.settings.save();
                }
            });

        ui.add_space(8.0);

        // Card 3: Chart Display & Downsampling Performance
        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Display Tuning & Performance").color(white).size(13.0).strong());
                ui.add_space(6.0);

                let mut tuning_changed = false;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Trace Line Thickness:").color(muted).size(11.5));
                    if ui.add(egui::Slider::new(&mut self.settings.telemetry.trace_stroke_width, 1.0..=4.0).suffix(" px")).changed() {
                        tuning_changed = true;
                    }
                });

                if ui.checkbox(&mut self.settings.telemetry.enable_downsampling_lod, "Adaptive LOD downsampling for 60Hz traces (smoother 60+ FPS)").changed() {
                    tuning_changed = true;
                }

                if tuning_changed {
                    self.settings.save();
                }
            });

        ui.add_space(8.0);

        // Card 4: Directory & Subfolder Management
        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Telemetry Storage & Folders").color(white).size(13.0).strong());
                ui.add_space(6.0);

                if ui.checkbox(&mut self.settings.telemetry.include_subfolders, "Scan subfolders recursively in telemetry directory").changed() {
                    self.ibt_lab.include_subfolders = self.settings.telemetry.include_subfolders;
                    self.settings.save();
                }
                ui.label(egui::RichText::new("Sessions in subfolders (e.g. Telemetry/Sebring/session.ibt) automatically appear with folder tags.").color(muted).size(10.5));
            });
    }

    fn render_settings_overlay_tab(
        &mut self,
        ui: &mut egui::Ui,
        white: egui::Color32,
        muted: egui::Color32,
        elevated: egui::Color32,
    ) {
        ui.label(egui::RichText::new("IN-CAR OVERLAY & HUD MODULES").color(muted).size(11.0).extra_letter_spacing(0.8));
        ui.add_space(6.0);

        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("HUD Behavior & Visibility").color(white).size(13.0).strong());
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.settings.overlay.enabled, "Auto-show overlay in car").changed() {
                        self.settings.save();
                    }
                    if ui.checkbox(&mut self.settings.overlay.always_on_top, "Always on top").changed() {
                        self.settings.save();
                    }
                    let preview_btn_text = if self.preview_overlay_hud { "Hide Preview" } else { "Show Preview" };
                    if ui.selectable_label(self.preview_overlay_hud, preview_btn_text).clicked() {
                        self.preview_overlay_hud = !self.preview_overlay_hud;
                    }
                });

                ui.add_space(6.0);
                ui.label(egui::RichText::new("Visible HUD Sections:").color(muted).size(11.0));
                ui.horizontal_wrapped(|ui| {
                    if ui.checkbox(&mut self.settings.overlay.show_pace_card, "Pace & Stint").changed() {
                        self.settings.save();
                    }
                    if ui.checkbox(&mut self.settings.overlay.show_fuel_card, "Fuel & Strategy").changed() {
                        self.settings.save();
                    }
                    if ui.checkbox(&mut self.settings.overlay.show_race_control_card, "Race Control & Flags").changed() {
                        self.settings.save();
                    }
                    if ui.checkbox(&mut self.settings.overlay.show_weather, "Track Temp & Wind").changed() {
                        self.settings.save();
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Primary Fuel Stat:").color(muted));
                    let is_fuel_amount = self.settings.overlay.fuel_hero_metric == FuelHeroMetric::FuelAmount;
                    if ui.selectable_label(is_fuel_amount, "Fuel in Tank (Gallons / L)").clicked() {
                        self.settings.overlay.fuel_hero_metric = FuelHeroMetric::FuelAmount;
                        self.settings.save();
                    }
                    if ui.selectable_label(!is_fuel_amount, "Laps to Empty").clicked() {
                        self.settings.overlay.fuel_hero_metric = FuelHeroMetric::LapsToEmpty;
                        self.settings.save();
                    }
                });
            });

        ui.add_space(8.0);

        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Scale & Visual Opacity").color(white).size(13.0).strong());
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("HUD Scale:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.overlay.hud_scale, 0.75..=1.50).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("HUD Card Opacity:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.overlay.opacity, 0.05..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("HUD Background Opacity:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.overlay.bg_opacity, 0.0..=1.0).custom_formatter(|n, _| if n <= 0.005 { "0% (Transparent)".to_string() } else { format!("{:.0}%", n * 100.0) })).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Pit safety margin:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.overlay.safety_margin_laps, 0.0..=5.0).suffix(" laps")).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Overlay position:").color(muted));
                    let pos_str = match self.settings.overlay.position {
                        Some([x, y]) => format!("({:.0}, {:.0})", x, y),
                        None => "Default (Center)".to_string(),
                    };
                    ui.label(egui::RichText::new(pos_str).color(white));
                    if self.settings.overlay.position.is_some() {
                        if ui.add(egui::Button::new(egui::RichText::new("Reset Position").color(muted).size(11.5)).fill(elevated)).clicked() {
                            self.settings.overlay.position = None;
                            self.settings.save();
                        }
                    }
                });
            });
    }

    fn render_settings_tire_popup_tab(
        &mut self,
        ui: &mut egui::Ui,
        white: egui::Color32,
        muted: egui::Color32,
        elevated: egui::Color32,
    ) {
        ui.label(egui::RichText::new("POST-PIT 4-CORNER TIRE POPUP OVERLAY").color(muted).size(11.0).extra_letter_spacing(0.8));
        ui.add_space(6.0);

        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Tire Card Triggers & Timings").color(white).size(13.0).strong());
                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    if ui.checkbox(&mut self.settings.tire_popup_overlay.enabled, "Auto-popup 4-corner tires after pitting").changed() {
                        self.settings.save();
                    }
                    if ui.checkbox(&mut self.settings.tire_popup_overlay.always_on_top, "Always on top").changed() {
                        self.settings.save();
                    }
                    let tire_prev_text = if self.preview_tire_popup_hud { "Hide Preview" } else { "Show Preview" };
                    if ui.selectable_label(self.preview_tire_popup_hud, tire_prev_text).clicked() {
                        self.preview_tire_popup_hud = !self.preview_tire_popup_hud;
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Popup Duration:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.tire_popup_overlay.duration_secs, 3.0..=60.0).suffix(" sec")).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Popup Scale:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.tire_popup_overlay.scale, 0.75..=1.50).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Tire Card Opacity:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.tire_popup_overlay.opacity, 0.05..=1.0).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Tire Background Opacity:").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.tire_popup_overlay.bg_opacity, 0.0..=1.0).custom_formatter(|n, _| if n <= 0.005 { "0% (Transparent)".to_string() } else { format!("{:.0}%", n * 100.0) })).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Tire popup position:").color(muted));
                    let tire_pos_str = match self.settings.tire_popup_overlay.position {
                        Some([x, y]) => format!("({:.0}, {:.0})", x, y),
                        None => "Default (Center)".to_string(),
                    };
                    ui.label(egui::RichText::new(tire_pos_str).color(white));
                    if self.settings.tire_popup_overlay.position.is_some() {
                        if ui.add(egui::Button::new(egui::RichText::new("Reset Position").color(muted).size(11.5)).fill(elevated)).clicked() {
                            self.settings.tire_popup_overlay.position = None;
                            self.settings.save();
                        }
                    }
                });
            });
    }

    fn render_settings_grid_columns_tab(
        &mut self,
        ui: &mut egui::Ui,
        secondary: egui::Color32,
        white: egui::Color32,
        muted: egui::Color32,
        elevated: egui::Color32,
    ) {
        ui.label(egui::RichText::new("GRID COLUMNS & ROLLING AVERAGES").color(muted).size(11.0).extra_letter_spacing(0.8));
        ui.add_space(6.0);

        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Visible Table Columns").color(white).size(13.0).strong());
                ui.label(egui::RichText::new("Toggle which metrics appear in the standings and stint tables:").color(muted).size(11.0));
                ui.add_space(8.0);

                egui::Grid::new("column_grid").num_columns(3).spacing([24.0, 8.0]).show(ui, |ui| {
                    if ui.checkbox(&mut self.settings.columns.pos, "Position").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    if ui.checkbox(&mut self.settings.columns.car_num, "Car #").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    if ui.checkbox(&mut self.settings.columns.driver, "Driver").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    ui.end_row();

                    if ui.checkbox(&mut self.settings.columns.stint, "Stint #").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    if ui.checkbox(&mut self.settings.columns.laps, "Laps").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    if ui.checkbox(&mut self.settings.columns.last_lap, "Last Lap").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    ui.end_row();

                    if ui.checkbox(&mut self.settings.columns.fastest, "Fastest Lap").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    if ui.checkbox(&mut self.settings.columns.cur_avg, "Full Average").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    if ui.checkbox(&mut self.settings.columns.irating, "iRating").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    ui.end_row();

                    if ui.checkbox(&mut self.settings.columns.setup_name, "Setup Name").changed() { self.settings.layout_preset = LayoutPreset::Custom; self.settings.save(); }
                    ui.end_row();
                });
            });

        ui.add_space(8.0);

        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Rolling Lap Average Buckets").color(white).size(13.0).strong());
                ui.label(egui::RichText::new("Select which lap intervals to compute rolling averages for:").color(muted).size(11.0));
                ui.add_space(8.0);

                let mut to_toggle: Option<usize> = None;
                ui.horizontal_wrapped(|ui| {
                    for &b in &self.settings.custom_avg_buckets {
                        let is_active = self.settings.active_avg_buckets.contains(&b);
                        let chip_text = if is_active {
                            format!("✓ Avg {}", b)
                        } else {
                            format!("Avg {}", b)
                        };

                        let chip_btn = egui::Button::new(
                            egui::RichText::new(chip_text)
                                .color(if is_active { egui::Color32::WHITE } else { muted })
                                .size(11.5)
                        )
                        .fill(if is_active { with_alpha(secondary, 180) } else { elevated })
                        .rounding(egui::Rounding::same(5.0));

                        if ui.add(chip_btn).clicked() {
                            to_toggle = Some(b);
                        }
                    }
                });

                if let Some(b) = to_toggle {
                    if self.settings.active_avg_buckets.contains(&b) {
                        self.settings.active_avg_buckets.retain(|&x| x != b);
                    } else {
                        self.settings.active_avg_buckets.push(b);
                        self.settings.active_avg_buckets.sort();
                    }
                    self.settings.save();
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Custom bucket:").size(11.5).color(muted));
                    let edit = ui.add(egui::TextEdit::singleline(&mut self.custom_bucket_input).desired_width(50.0).hint_text("e.g. 12"));
                    let enter_pressed = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                    if ui.add(egui::Button::new(egui::RichText::new("+ Add").color(white).size(11.5)).fill(elevated)).clicked() || enter_pressed {
                        if let Ok(num) = self.custom_bucket_input.trim().parse::<usize>() {
                            if num > 0 && num <= 500 {
                                if !self.settings.custom_avg_buckets.contains(&num) {
                                    self.settings.custom_avg_buckets.push(num);
                                    self.settings.custom_avg_buckets.sort();
                                }
                                if !self.settings.active_avg_buckets.contains(&num) {
                                    self.settings.active_avg_buckets.push(num);
                                    self.settings.active_avg_buckets.sort();
                                }
                                self.custom_bucket_input.clear();
                                self.settings.save();
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(egui::RichText::new("Reset defaults").color(muted).size(11.0))
                                .fill(elevated)
                        ).clicked() {
                            self.settings.custom_avg_buckets = vec![5, 10, 15, 20, 25, 30, 35, 40, 50, 60];
                            self.settings.active_avg_buckets = vec![5, 10, 15, 20, 25, 30];
                            self.settings.save();
                        }
                    });
                });
            });
    }

    fn render_settings_themes_tab(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        primary: egui::Color32,
        _secondary: egui::Color32,
        white: egui::Color32,
        muted: egui::Color32,
        elevated: egui::Color32,
    ) {
        ui.label(egui::RichText::new("APPEARANCE & COLOR THEMES").color(muted).size(11.0).extra_letter_spacing(0.8));
        ui.add_space(6.0);

        let themes = [
            (
                ThemePreset::S4SGoldBlue,
                "S4S Gold & Blue (Dark)",
                "Official Search For Speed livery: High-contrast racing gold accents with electric cyan highlights and deep dark slate canvas.",
                egui::Color32::from_rgb(255, 200, 59),
                egui::Color32::from_rgb(0, 210, 255),
                egui::Color32::from_rgb(14, 17, 23),
            ),
            (
                ThemePreset::StealthDark,
                "Stealth Monochrome",
                "Ultra-clean night racing aesthetic with muted silver tones and minimal color distraction for pure focus.",
                egui::Color32::from_rgb(212, 216, 224),
                egui::Color32::from_rgb(138, 148, 166),
                egui::Color32::from_rgb(8, 10, 14),
            ),
            (
                ThemePreset::S4SGoldBlueLight,
                "S4S Daylight (Light Mode)",
                "Crisp, sunlight-readable high-contrast daylight theme designed for bright rooms and daytime pit stands.",
                egui::Color32::from_rgb(200, 136, 0),
                egui::Color32::from_rgb(0, 119, 204),
                egui::Color32::from_rgb(244, 246, 249),
            ),
        ];

        for (preset, title, desc, col1, col2, bg_col) in themes {
            let is_sel = self.settings.theme == preset;
            egui::Frame::none()
                .fill(if is_sel { with_alpha(primary, 25) } else { elevated })
                .stroke(if is_sel { egui::Stroke::new(1.5, primary) } else { egui::Stroke::NONE })
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (resp, painter) = ui.allocate_painter(egui::vec2(44.0, 22.0), egui::Sense::hover());
                        let r = resp.rect;
                        painter.rect_filled(r, 4.0, bg_col);
                        painter.rect_stroke(r, 4.0, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30)));
                        painter.circle_filled(egui::pos2(r.min.x + 12.0, r.center().y), 5.0, col1);
                        painter.circle_filled(egui::pos2(r.min.x + 28.0, r.center().y), 5.0, col2);

                        ui.label(egui::RichText::new(title).color(if is_sel { primary } else { white }).size(12.5).strong());

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if is_sel {
                                ui.label(egui::RichText::new("Active ✓").color(primary).strong().size(11.5));
                            } else if ui.add(egui::Button::new(egui::RichText::new("Select").color(white).size(11.0)).fill(elevated)).clicked() {
                                self.settings.theme = preset;
                                self.settings.save();
                                setup_custom_style(ctx, &self.settings);
                                self.logo_texture = load_logo_texture(ctx, self.settings.is_dark_theme());
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(desc).color(muted).size(10.5));
                });
            ui.add_space(6.0);
        }
    }

    fn render_settings_updates_tab(
        &mut self,
        ui: &mut egui::Ui,
        primary: egui::Color32,
        white: egui::Color32,
        muted: egui::Color32,
        elevated: egui::Color32,
    ) {
        ui.label(egui::RichText::new("UPDATES & RELEASE CHANNEL").color(muted).size(11.0).extra_letter_spacing(0.8));
        ui.add_space(6.0);

        egui::Frame::none()
            .fill(elevated)
            .rounding(egui::Rounding::same(8.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let version_label = format!("Installed Version: v{}", updater::CURRENT_VERSION);
                    ui.label(egui::RichText::new(version_label).color(white).size(13.0).strong());
                    if settings::is_beta_mode() {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(110, 45, 160))
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("BETA / STAGING").size(10.5).color(egui::Color32::WHITE).strong());
                            });
                    }
                });

                ui.add_space(6.0);
                if ui.checkbox(&mut self.settings.updates.check_on_startup, "Check for updates automatically on launch").changed() {
                    self.settings.save();
                }
                if ui.checkbox(&mut self.settings.updates.include_prereleases, "Include Beta / Staging pre-releases").changed() {
                    self.settings.save();
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let check_btn = egui::Button::new(egui::RichText::new("Check for Updates Now").color(white).size(11.5))
                        .fill(elevated);
                    if ui.add(check_btn).clicked() {
                        self.updater.trigger_check(self.settings.updates.include_prereleases);
                    }

                    if ui.add(egui::Button::new(egui::RichText::new("📖 What's New / Notes").color(white).size(11.5)).fill(elevated)).clicked() {
                        self.show_release_notes_modal = true;
                    }

                    let updater_status = self.updater.status.clone();
                    match &updater_status {
                        updater::UpdateStatus::Idle => {},
                        updater::UpdateStatus::Checking => {
                            ui.spinner();
                            ui.label(egui::RichText::new("Checking GitHub Releases...").color(muted).size(11.5));
                        },
                        updater::UpdateStatus::UpToDate => {
                            ui.label(egui::RichText::new("✓ Up to date").color(egui::Color32::from_rgb(75, 225, 110)).size(11.5));
                        },
                        updater::UpdateStatus::UpdateAvailable(info) => {
                            let text = format!("Update {} available!", info.tag_name);
                            ui.label(egui::RichText::new(text).color(primary).strong().size(11.5));
                            let info_clone = info.clone();
                            if ui.add(egui::Button::new(egui::RichText::new("Update Now ⚡").color(egui::Color32::BLACK).strong().size(11.5)).fill(primary)).clicked() {
                                self.updater.trigger_download_and_install(&info_clone);
                            }
                            if ui.add(egui::Button::new(egui::RichText::new("View Release ↗").color(white).size(11.5)).fill(elevated)).clicked() {
                                updater::open_browser(&info.html_url);
                            }
                        },
                        updater::UpdateStatus::Downloading { percent, downloaded_bytes, total_bytes, .. } => {
                            let mb_down = *downloaded_bytes as f32 / (1024.0 * 1024.0);
                            let mb_tot = *total_bytes as f32 / (1024.0 * 1024.0);
                            ui.label(egui::RichText::new(format!("Downloading: {:.0}% ({:.1}/{:.1} MB)", percent, mb_down, mb_tot)).color(primary).size(11.5));
                        },
                        updater::UpdateStatus::ReadyToRestart { .. } => {
                            if ui.add(egui::Button::new(egui::RichText::new("Restart to Apply 🔄").color(egui::Color32::BLACK).strong().size(11.5)).fill(self.settings.status_green())).clicked() {
                                if let Err(e) = updater::apply_update_and_restart() { self.updater.status = updater::UpdateStatus::Error(e); }
                            }
                        },
                        updater::UpdateStatus::Error(err) => {
                            ui.label(egui::RichText::new(format!("⚠ {}", err)).color(egui::Color32::from_rgb(255, 120, 120)).size(11.0));
                        }
                    }
                });
            });
    }


        pub(crate) fn render_stint_db_modal(&mut self, ctx: &egui::Context, s: &WarRoomState) {
        if !self.show_stint_db_modal {
            return;
        }

        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let green = self.settings.status_green();
        let is_metric = self.settings.use_metric;
        let fuel_conv = if is_metric { 1.0 } else { 0.264172 };
        let fuel_unit_str = if is_metric { "L" } else { "gal" };

        let has_real_track = is_real_track_name(&s.track_name);
        let filter_current_track = self.stint_db_filter_current_track && has_real_track;

        let total_saved = self.stint_db.records.len();
        let current_track_count = if has_real_track {
            self.stint_db.count_for_track(&s.track_name)
        } else {
            0
        };

        let screen_rect = ctx.screen_rect();
        let win_w = (screen_rect.width() * 0.92).clamp(1240.0, 1600.0);
        let win_h = (screen_rect.height() * 0.85).clamp(560.0, 920.0);

        let mut is_open = self.show_stint_db_modal;
        let mut close_modal = false;

        let mut action_reimport: Option<stint_db::SavedStintRecord> = None;
        let mut action_compare: Option<stint_db::SavedStintRecord> = None;
        let mut action_share: Option<stint_db::SavedStintRecord> = None;
        let mut action_export_as: Option<stint_db::SavedStintRecord> = None;
        let mut action_import_file: Option<std::path::PathBuf> = None;
        let mut action_delete_id: Option<String> = None;

        egui::Window::new("📂 Saved Stints Database · Track Re-Import & Share")
            .id(egui::Id::new("stint_database_modal_window_v4"))
            .open(&mut is_open)
            .default_size([win_w, win_h])
            .min_width(1200.0)
            .min_height(520.0)
            .default_pos([screen_rect.min.x + 30.0, screen_rect.min.y + 40.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_modal = true;
                }

                // Top Filter & Action Bar
                ui.horizontal(|ui| {
                    if has_real_track {
                        let cur_btn = egui::Button::new(
                            egui::RichText::new(format!("🏁 Current Track ({})", current_track_count))
                                .color(if filter_current_track { egui::Color32::WHITE } else { muted })
                                .size(12.0)
                                .strong(),
                        )
                        .fill(if filter_current_track { with_alpha(primary, 190) } else { elevated })
                        .rounding(egui::Rounding::same(6.0));
                        if ui
                            .add(cur_btn)
                            .on_hover_text(format!("Show saved stints matching {}", s.track_name))
                            .clicked()
                        {
                            self.stint_db_filter_current_track = true;
                        }
                    }

                    let all_btn = egui::Button::new(
                        egui::RichText::new(format!("🌎 All Saved Stints ({})", total_saved))
                            .color(if !filter_current_track { egui::Color32::WHITE } else { muted })
                            .size(12.0)
                            .strong(),
                    )
                    .fill(if !filter_current_track { with_alpha(secondary, 190) } else { elevated })
                    .rounding(egui::Rounding::same(6.0));
                    if ui.add(all_btn).clicked() {
                        self.stint_db_filter_current_track = false;
                    }

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.label(egui::RichText::new("SEARCH").color(muted).size(11.0).extra_letter_spacing(0.8));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.stint_db_search_query)
                            .hint_text("Track, Car, Setup, Driver, Notes...")
                            .desired_width(210.0),
                    );
                    if !self.stint_db_search_query.is_empty() {
                        if ui
                            .add(egui::Button::new(egui::RichText::new("Clear").color(muted).size(11.0)).frame(false))
                            .clicked()
                        {
                            self.stint_db_search_query.clear();
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_sized(
                                egui::vec2(72.0, 26.0),
                                egui::Button::new(egui::RichText::new("× Close").color(white).size(12.0)).fill(elevated),
                            )
                            .clicked()
                        {
                            close_modal = true;
                        }

                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("📂 Open Stints Folder").color(white).size(11.5))
                                    .fill(elevated)
                                    .rounding(egui::Rounding::same(6.0)),
                            )
                            .on_hover_text("Open the local .s4s.json stints folder in Windows Explorer to share or drop stint files")
                            .clicked()
                        {
                            let folder = self.stint_db.stints_folder_path();
                            let _ = std::fs::create_dir_all(&folder);
                            stint_db::open_folder_in_explorer(&folder);
                        }

                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("🔄 Sync Folder").color(white).size(11.5))
                                    .fill(elevated)
                                    .rounding(egui::Rounding::same(6.0)),
                            )
                            .on_hover_text("Scan the stints folder for any new .s4s.json stint files dropped from teammates")
                            .clicked()
                        {
                            let added = self.stint_db.scan_stints_folder();
                            if added > 0 {
                                self.toast_message = Some((
                                    format!("📂 Imported {} new stint file(s) from folder!", added),
                                    std::time::Instant::now(),
                                ));
                            } else {
                                self.toast_message = Some((
                                    "📂 Stints folder is up to date.".to_string(),
                                    std::time::Instant::now(),
                                ));
                            }
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("📥 Import Stint File...")
                                        .color(egui::Color32::BLACK)
                                        .size(11.5)
                                        .strong(),
                                )
                                .fill(primary)
                                .rounding(egui::Rounding::same(6.0)),
                            )
                            .on_hover_text("Open file picker to import a teammate's .s4s.json stint file and load it into your session")
                            .clicked()
                        {
                            if let Some(path) = stint_db::pick_open_stint_file_dialog() {
                                action_import_file = Some(path);
                            }
                        }

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("📋 Paste from Clipboard")
                                        .color(white)
                                        .size(11.5),
                                )
                                .fill(elevated)
                                .rounding(egui::Rounding::same(6.0)),
                            )
                            .on_hover_text("Import a stint from clipboard: supports files copied in Windows Explorer (.s4s.json) or text share codes")
                            .clicked()
                        {
                            let clip_files = stint_db::read_clipboard_file_paths();
                            if let Some(first_file) = clip_files.first() {
                                action_import_file = Some(first_file.clone());
                            } else {
                                match stint_db::read_clipboard_text().and_then(|raw| stint_db::import_from_str(&raw)) {
                                    Ok(rec) => {
                                        self.stint_db.upsert_stint(rec.clone());
                                        action_reimport = Some(rec);
                                    }
                                    Err(err) => {
                                        self.toast_message = Some((
                                            format!("⚠ Clipboard import failed: {}", err),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                }
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                // Info Banner
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(16, 22, 32))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 14)))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new("💾 Auto-Save Active:")
                                    .color(green)
                                    .strong()
                                    .size(11.5),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Completed stints (1+ laps) are automatically saved locally with lap times, fuel burn, pit tire wear/temps, and setup. Click '📎 Share File' on any stint to copy the .s4s.json file to your clipboard (press Ctrl+V in Discord to attach), or click '📥 Import Stint File...' to load a teammate's stint directly.",
                                )
                                .color(muted)
                                .size(11.5),
                            );
                        });
                    });

                ui.add_space(8.0);

                let query = self.stint_db_search_query.trim().to_lowercase();
                let filtered_records: Vec<stint_db::SavedStintRecord> = self
                    .stint_db
                    .records
                    .iter()
                    .filter(|r| {
                        if filter_current_track && !stint_db::tracks_match(&r.track_name, &s.track_name) {
                            return false;
                        }
                        if !query.is_empty() {
                            let m_track = r.track_name.to_lowercase().contains(&query);
                            let m_car = r.car_name.to_lowercase().contains(&query);
                            let m_setup = r.setup_name.to_lowercase().contains(&query);
                            let m_drv = r.driver_name.to_lowercase().contains(&query);
                            let m_notes = r.stint.notes.to_lowercase().contains(&query);
                            if !(m_track || m_car || m_setup || m_drv || m_notes) {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect();

                if filtered_records.is_empty() {
                    egui::Frame::none()
                        .fill(elevated)
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(24.0)
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                if filter_current_track {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "No saved stints found for current track ({}) yet.",
                                            s.track_name
                                        ))
                                        .color(white)
                                        .strong()
                                        .size(14.0),
                                    );
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Complete at least 1 green-flag lap on track to auto-save your first stint here, or switch to 'All Saved Stints'.",
                                        )
                                        .color(muted)
                                        .size(12.0),
                                    );
                                    if total_saved > 0 {
                                        ui.add_space(10.0);
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    egui::RichText::new(format!(
                                                        "🌎 Show All Saved Stints ({})",
                                                        total_saved
                                                    ))
                                                    .color(white)
                                                    .size(12.0),
                                                )
                                                .fill(with_alpha(secondary, 160)),
                                            )
                                            .clicked()
                                        {
                                            self.stint_db_filter_current_track = false;
                                        }
                                    }
                                } else {
                                    ui.label(
                                        egui::RichText::new("No saved stints match your filter yet.")
                                            .color(white)
                                            .strong()
                                            .size(14.0),
                                    );
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "Complete laps in iRacing (auto-saved automatically), click '📎 Share Stint File' on any stint in Stint Analyzer, or click '📥 Import Stint File...' above.",
                                        )
                                        .color(muted)
                                        .size(12.0),
                                    );
                                }
                            });
                        });
                } else {
                    let player_drv = s.drivers.get(&s.player_car_idx);
                    let avail_h = (ui.available_height() - 10.0).max(200.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(14, 17, 23))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)))
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            egui::ScrollArea::both()
                                .auto_shrink([false, false])
                                .max_height(avail_h)
                                .show(ui, |ui| {
                                    TableBuilder::new(ui)
                                        .striped(true)
                                        .resizable(true)
                                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                        .column(Column::initial(115.0).at_least(95.0))  // Saved Date
                                        .column(Column::initial(180.0).at_least(130.0)) // Track & Car
                                        .column(Column::initial(160.0).at_least(120.0)) // Setup & Driver
                                        .column(Column::initial(46.0).at_least(38.0))   // Laps
                                        .column(Column::initial(72.0).at_least(60.0))   // Fastest
                                        .column(Column::initial(72.0).at_least(60.0))   // Full Avg
                                        .column(Column::initial(68.0).at_least(56.0))   // Fuel/Lap
                                        .column(Column::initial(74.0).at_least(60.0))   // Pit Tires
                                        .column(Column::remainder().at_least(380.0))    // Actions
                                        .header(28.0, |mut header| {
                                            header.col(|ui| { ui.label(egui::RichText::new("Saved").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Track & Car").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Setup & Driver").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Laps").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Fastest").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Full Avg").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Fuel/Lap").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Pit Tires").color(muted).size(11.5)); });
                                            header.col(|ui| { ui.label(egui::RichText::new("Actions (Re-Import / Compare / Share File / Show / Export)").color(muted).size(11.5)); });
                                        })
                                        .body(|mut body| {
                                            for (row_idx, rec) in filtered_records.iter().enumerate() {
                                                let is_track_match = has_real_track && stint_db::tracks_match(&rec.track_name, &s.track_name);
                                                let already_imported_num = player_drv.and_then(|d| {
                                                    d.stints
                                                        .iter()
                                                        .find(|st| st.is_imported && st.saved_id == rec.id)
                                                        .map(|st| st.stint_number)
                                                });

                                                body.row(34.0, |mut row| {
                                                    // 1. Saved Date
                                                    row.col(|ui| {
                                                        ui.label(egui::RichText::new(&rec.saved_at_display).color(muted).size(11.5));
                                                    });

                                                    // 2. Track & Car
                                                    row.col(|ui| {
                                                        ui.vertical(|ui| {
                                                            ui.spacing_mut().item_spacing.y = 1.0;
                                                            ui.horizontal(|ui| {
                                                                ui.label(egui::RichText::new(&rec.track_name).color(white).strong().size(12.0));
                                                                if is_track_match {
                                                                    egui::Frame::none()
                                                                        .fill(with_alpha(green, 34))
                                                                        .rounding(egui::Rounding::same(4.0))
                                                                        .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                                        .show(ui, |ui| {
                                                                            ui.label(egui::RichText::new("MATCH").color(green).strong().size(9.5));
                                                                        });
                                                                }
                                                            });
                                                            ui.label(egui::RichText::new(&rec.car_name).color(muted).size(10.5));
                                                        });
                                                    });

                                                    // 3. Setup & Driver
                                                    row.col(|ui| {
                                                        ui.vertical(|ui| {
                                                            ui.spacing_mut().item_spacing.y = 1.0;
                                                            ui.label(egui::RichText::new(&rec.setup_name).color(primary).strong().size(11.5));
                                                            let drv_txt = format!("#{} {} · {}", rec.car_num, rec.driver_name, rec.session_type);
                                                            let resp = ui.label(egui::RichText::new(drv_txt).color(muted).size(10.5));
                                                            if !rec.stint.notes.is_empty() {
                                                                resp.on_hover_text(format!("Notes: {}", rec.stint.notes));
                                                            }
                                                        });
                                                    });

                                                    // 4. Laps
                                                    row.col(|ui| {
                                                        ui.label(egui::RichText::new(format!("{}", rec.stint.laps_completed)).color(white).strong().size(12.5));
                                                    });

                                                    // 5. Fastest
                                                    row.col(|ui| {
                                                        ui.label(egui::RichText::new(format_time(rec.stint.fast_lap)).color(green).strong().size(12.5));
                                                    });

                                                    // 6. Full Avg
                                                    row.col(|ui| {
                                                        ui.label(egui::RichText::new(format_time(rec.stint.full_average())).color(white).size(12.5));
                                                    });

                                                    // 7. Fuel/Lap
                                                    row.col(|ui| {
                                                        let f_rate = rec.stint.fuel_per_lap() * fuel_conv;
                                                        let f_str = if f_rate > 0.0005 {
                                                            format!("{:.3} {}", f_rate, fuel_unit_str)
                                                        } else {
                                                            "—".to_string()
                                                        };
                                                        ui.label(egui::RichText::new(f_str).color(white).size(11.5));
                                                    });

                                                    // 8. Pit Tires
                                                    row.col(|ui| {
                                                        if rec.stint.tires.has_data {
                                                            let lf_w = (rec.stint.tires.lf.wear_l + rec.stint.tires.lf.wear_m + rec.stint.tires.lf.wear_r) / 3.0;
                                                            let rf_w = (rec.stint.tires.rf.wear_l + rec.stint.tires.rf.wear_m + rec.stint.tires.rf.wear_r) / 3.0;
                                                            egui::Frame::none()
                                                                .fill(with_alpha(secondary, 36))
                                                                .rounding(egui::Rounding::same(4.0))
                                                                .inner_margin(egui::Margin::symmetric(5.0, 1.5))
                                                                .show(ui, |ui| {
                                                                    ui.label(egui::RichText::new(format!("✓ {:.0}/{:.0}%", lf_w, rf_w)).color(secondary).strong().size(10.5));
                                                                });
                                                        } else {
                                                            ui.label(egui::RichText::new("—").color(muted).size(11.5));
                                                        }
                                                    });

                                                    // 9. Actions
                                                    row.col(|ui| {
                                                        ui.push_id(("stint_db_row_actions", row_idx, &rec.id), |ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.spacing_mut().item_spacing.x = 4.0;
                                                                let reimp_label = if let Some(imp_n) = already_imported_num {
                                                                    format!("✓ IMP {}", (imp_n - 100).max(1))
                                                                } else {
                                                                    "📥 Re-Import".to_string()
                                                                };
                                                                let reimp_btn = egui::Button::new(
                                                                    egui::RichText::new(reimp_label)
                                                                        .color(if already_imported_num.is_some() { white } else { egui::Color32::BLACK })
                                                                        .size(11.5)
                                                                        .strong(),
                                                                )
                                                                .fill(if already_imported_num.is_some() { with_alpha(green, 150) } else { primary })
                                                                .rounding(egui::Rounding::same(5.0));

                                                                if ui
                                                                    .add(reimp_btn)
                                                                    .on_hover_text("Load this saved stint into the active session (Cockpit & Grid) as a reference stint")
                                                                    .clicked()
                                                                {
                                                                    action_reimport = Some(rec.clone());
                                                                }

                                                                if ui
                                                                    .add(
                                                                        egui::Button::new(egui::RichText::new("⚖ Compare").color(white).size(11.5))
                                                                            .fill(elevated)
                                                                            .rounding(egui::Rounding::same(5.0)),
                                                                    )
                                                                    .on_hover_text("Re-import and immediately open in Side-by-Side Stint Comparison")
                                                                    .clicked()
                                                                {
                                                                    action_compare = Some(rec.clone());
                                                                }

                                                                if ui
                                                                    .add(
                                                                        egui::Button::new(egui::RichText::new("📎 Share File").color(white).size(11.5))
                                                                            .fill(elevated)
                                                                            .rounding(egui::Rounding::same(5.0)),
                                                                    )
                                                                    .on_hover_text("Copy the .s4s.json stint file to Windows Clipboard (press Ctrl+V in Discord or Explorer to attach the file)")
                                                                    .clicked()
                                                                {
                                                                    action_share = Some(rec.clone());
                                                                }

                                                                let file_path = self.stint_db.record_file_path(&rec);
                                                                if ui
                                                                    .add(
                                                                        egui::Button::new(egui::RichText::new("📂 Show").color(white).size(11.5))
                                                                            .fill(elevated)
                                                                            .rounding(egui::Rounding::same(5.0)),
                                                                    )
                                                                    .on_hover_text("Locate and highlight this stint file (.s4s.json) in Windows Explorer to drag-and-drop into Discord")
                                                                    .clicked()
                                                                {
                                                                    stint_db::reveal_file_in_explorer(&file_path);
                                                                }

                                                                if ui
                                                                    .add(
                                                                        egui::Button::new(egui::RichText::new("💾").color(muted).size(11.5))
                                                                            .fill(elevated)
                                                                            .rounding(egui::Rounding::same(5.0)),
                                                                    )
                                                                    .on_hover_text("Export a copy of this .s4s.json stint file to any folder on your PC")
                                                                    .clicked()
                                                                {
                                                                    action_export_as = Some(rec.clone());
                                                                }

                                                                if ui
                                                                    .add(
                                                                        egui::Button::new(egui::RichText::new("🗑").color(muted).size(11.5))
                                                                            .fill(egui::Color32::TRANSPARENT),
                                                                    )
                                                                    .on_hover_text("Delete this stint from local database")
                                                                    .clicked()
                                                                {
                                                                    action_delete_id = Some(rec.id.clone());
                                                                }
                                                            });
                                                        });
                                                    });
                                                });
                                            }
                                        });
                                });
                        });
                }
            });

        if let Some(path) = action_import_file {
            match stint_db::import_stint_from_file(&path) {
                Ok(rec) => {
                    self.stint_db.upsert_stint(rec.clone());
                    let dest_path = self.stint_db.record_file_path(&rec);
                    if path != dest_path && path.exists() {
                        let _ = std::fs::copy(&path, &dest_path);
                    }
                    action_reimport = Some(rec);
                }
                Err(err) => {
                    self.toast_message = Some((
                        format!("⚠ Could not import file: {}", err),
                        std::time::Instant::now(),
                    ));
                }
            }
        }

        if let Some(rec) = action_reimport {
            let (car_idx, imp_num) = if let Ok(mut state_guard) = self.state.lock() {
                let num = state_guard.import_saved_stint(&rec);
                (state_guard.player_car_idx, num)
            } else {
                (s.player_car_idx, 101)
            };
            self.selected_driver_car_idx = Some(car_idx);
            self.selected_stint_num = Some(imp_num);
            let imp_label = format!("IMP {}", (imp_num - 100).max(1));
            self.toast_message = Some((
                format!(
                    "📥 Re-imported {} ({} · {} laps) into session!",
                    imp_label, rec.setup_name, rec.stint.laps_completed
                ),
                std::time::Instant::now(),
            ));
        }

        if let Some(rec) = action_compare {
            let (car_idx, imp_num, cloned_state) = if let Ok(mut state_guard) = self.state.lock() {
                let num = state_guard.import_saved_stint(&rec);
                let p_idx = state_guard.player_car_idx;
                (p_idx, num, state_guard.clone_state_for_ui())
            } else {
                (s.player_car_idx, 101, s.clone_state_for_ui())
            };
            self.ensure_compare_slots_populated(&cloned_state);
            let slot_entry = StintCompareSlot {
                car_idx,
                stint_number: imp_num,
            };
            if !self.stint_compare_slots.contains(&slot_entry) {
                if self.stint_compare_slots.len() >= 3 {
                    self.stint_compare_slots.pop();
                }
                self.stint_compare_slots.push(slot_entry);
            }
            self.selected_driver_car_idx = Some(car_idx);
            self.selected_stint_num = Some(imp_num);
            self.stint_analyzer_tab = StintAnalyzerTab::Compare;
            self.show_stint_analyzer_modal = true;
            self.show_stint_db_modal = false;
            is_open = false;
            let imp_label = format!("IMP {}", (imp_num - 100).max(1));
            self.toast_message = Some((
                format!("⚖ Loaded {} ({}) into Stint Comparison!", imp_label, rec.setup_name),
                std::time::Instant::now(),
            ));
        }

        if let Some(rec) = action_share {
            match self.stint_db.write_individual_stint_file(&rec) {
                Ok(file_path) => {
                    match stint_db::copy_stint_file_to_clipboard(&file_path) {
                        Ok(()) => {
                            self.toast_message = Some((
                                "📎 Copied Stint file (.s4s.json) to clipboard — press Ctrl+V in Discord or Explorer to attach!".to_string(),
                                std::time::Instant::now(),
                            ));
                        }
                        Err(_) => {
                            if let Ok(code) = stint_db::export_share_string(&rec) {
                                ctx.output_mut(|o| o.copied_text = code);
                                self.toast_message = Some((
                                    "📋 Copied stint share code to clipboard (ready to paste in Discord).".to_string(),
                                    std::time::Instant::now(),
                                ));
                            }
                        }
                    }
                }
                Err(e) => {
                    self.toast_message = Some((format!("⚠ Failed to write stint file: {}", e), std::time::Instant::now()));
                }
            }
        }

        if let Some(rec) = action_export_as {
            let default_name = self.stint_db.record_file_path(&rec)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("stint.s4s.json")
                .to_string();
            if let Some(dest_path) = stint_db::pick_save_stint_file_dialog(&default_name) {
                if let Ok(json) = serde_json::to_string_pretty(&rec) {
                    match std::fs::write(&dest_path, json) {
                        Ok(_) => {
                            self.toast_message = Some((
                                format!("💾 Exported stint file to '{}'!", dest_path.display()),
                                std::time::Instant::now(),
                            ));
                        }
                        Err(e) => {
                            self.toast_message = Some((
                                format!("⚠ Export failed: {}", e),
                                std::time::Instant::now(),
                            ));
                        }
                    }
                }
            }
        }

        if let Some(del_id) = action_delete_id {
            if self.stint_db.delete_stint(&del_id) {
                self.toast_message = Some(("🗑 Removed stint from local database".to_string(), std::time::Instant::now()));
            }
        }

        if !is_open || close_modal {
            self.show_stint_db_modal = false;
        }
    }
}
