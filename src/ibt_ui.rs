use crate::ibt::{
    build_demo_ibt_session, compare_laps, default_telemetry_dir, normalize_lap, parse_ibt_file,
    scan_ibt_directory, DamperRegime, IbtFileEntry, IbtSession, LapAnalysis, LapComparisonReport,
    NormalizedLapTrace, ShockCornerData, TireCornerAnalysis, TireTempSource, KPA_TO_PSI,
    MM_TO_INCHES,
};
use crate::settings::AppSettings;
use crate::{display_font, format_time, with_alpha};
use eframe::egui;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::thread;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IbtAnalysisTab {
    InputsAndSpeed,
    RideHeightAndRake,
    ShockHistograms,
    TiresAndPressures,
    ChannelsAndExport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XAxisMode {
    LapDistPct, // 0.0 - 100.0%
    LapDistMeters,
}

enum LoadWorkerState {
    Idle,
    Loading {
        file_name: String,
        rx: Receiver<Result<IbtSession, String>>,
    },
}

pub struct IbtTelemetryLabState {
    pub directory_input: String,
    pub scanned_files: Vec<IbtFileEntry>,
    pub file_filter: String,
    pub selected_file_path: Option<PathBuf>,
    pub session: Option<IbtSession>,
    pub active_tab: IbtAnalysisTab,
    pub selected_lap_a_idx: usize,
    pub compare_enabled: bool,
    pub selected_lap_b_idx: usize,
    pub x_axis_mode: XAxisMode,
    pub hover_dist_pct: Option<f32>,
    pub channel_search_filter: String,
    pub status_error: Option<String>,
    worker: LoadWorkerState,
    has_initialized_dir: bool,
}

impl Default for IbtTelemetryLabState {
    fn default() -> Self {
        let default_dir = default_telemetry_dir();
        Self {
            directory_input: default_dir.to_string_lossy().to_string(),
            scanned_files: Vec::new(),
            file_filter: String::new(),
            selected_file_path: None,
            session: None,
            active_tab: IbtAnalysisTab::InputsAndSpeed,
            selected_lap_a_idx: 0,
            compare_enabled: false,
            selected_lap_b_idx: 0,
            x_axis_mode: XAxisMode::LapDistPct,
            hover_dist_pct: None,
            channel_search_filter: String::new(),
            status_error: None,
            worker: LoadWorkerState::Idle,
            has_initialized_dir: false,
        }
    }
}

impl IbtTelemetryLabState {
    pub fn ensure_initialized(&mut self) {
        if !self.has_initialized_dir {
            self.has_initialized_dir = true;
            self.refresh_directory();
        }
    }

    pub fn refresh_directory(&mut self) {
        let path = PathBuf::from(self.directory_input.trim());
        self.scanned_files = scan_ibt_directory(&path);
    }

    pub fn trigger_load_file(&mut self, path: PathBuf, ctx: egui::Context) {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("telemetry.ibt")
            .to_string();

        self.selected_file_path = Some(path.clone());
        self.status_error = None;

        let (tx, rx) = channel();
        self.worker = LoadWorkerState::Loading { file_name, rx };

        thread::spawn(move || {
            let res = parse_ibt_file(&path);
            let _ = tx.send(res);
            ctx.request_repaint();
        });
    }

    pub fn trigger_load_demo(&mut self, ctx: egui::Context) {
        self.selected_file_path = None;
        self.status_error = None;

        let (tx, rx) = channel();
        self.worker = LoadWorkerState::Loading {
            file_name: "porsche963gtp_watkinsglen_demo.ibt".to_string(),
            rx,
        };

        thread::spawn(move || {
            let res = build_demo_ibt_session();
            let _ = tx.send(res);
            ctx.request_repaint();
        });
    }

    fn poll_worker(&mut self) {
        if let LoadWorkerState::Loading { ref rx, .. } = self.worker {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(sess) => {
                        let pb_idx = sess.fastest_lap_idx.unwrap_or(0);
                        self.selected_lap_a_idx = pb_idx.min(sess.analyzed_laps.len().saturating_sub(1));

                        // Pick a different valid lap for Lap B comparison if possible
                        let second_idx = sess
                            .laps
                            .iter()
                            .filter(|l| l.lap_index != self.selected_lap_a_idx && l.is_complete && !l.is_out_lap)
                            .min_by(|a, b| {
                                a.duration_secs
                                    .partial_cmp(&b.duration_secs)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|l| l.lap_index)
                            .unwrap_or_else(|| {
                                if self.selected_lap_a_idx > 0 {
                                    self.selected_lap_a_idx - 1
                                } else {
                                    (self.selected_lap_a_idx + 1)
                                        .min(sess.analyzed_laps.len().saturating_sub(1))
                                }
                            });

                        self.selected_lap_b_idx = second_idx;
                        self.compare_enabled = sess.analyzed_laps.len() >= 2 && second_idx != self.selected_lap_a_idx;
                        self.session = Some(sess);
                        self.status_error = None;
                    }
                    Err(e) => {
                        self.status_error = Some(e);
                    }
                }
                self.worker = LoadWorkerState::Idle;
            }
        }
    }

    pub fn render(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        toast_out: &mut Option<(String, std::time::Instant)>,
    ) {
        self.ensure_initialized();
        self.poll_worker();

        let primary = settings.primary_accent();
        let secondary = settings.secondary_accent();
        let panel_bg = settings.panel_color();
        let elevated = settings.card_elevated_color();
        let white = settings.text_color();
        let muted = settings.text_muted();
        let divider = settings.divider_color();
        let green = settings.status_green();
        let amber = settings.status_amber();
        let red = settings.status_red();
        let purple = settings.status_purple();
        let use_metric = settings.use_metric;

        // 1. TOP PROMINENT SECTION BANNER & SUB-NAVIGATION TABS
        egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, with_alpha(secondary, 130)))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(14.0, 9.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Distinct Mode Badge
                    egui::Frame::none()
                        .fill(with_alpha(secondary, 45))
                        .stroke(egui::Stroke::new(1.0, secondary))
                        .rounding(egui::Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("OFFLINE .IBT LAB")
                                    .color(white)
                                    .size(11.0)
                                    .strong(),
                            );
                        });

                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("60Hz Binary Telemetry & Vehicle Dynamics Engine")
                            .font(display_font(16.5))
                            .color(white),
                    );
                    ui.label(
                        egui::RichText::new("· Standalone Post-Run Analysis (Separate from Live Stint Tracker)")
                            .color(muted)
                            .size(11.5),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let tabs = [
                            (IbtAnalysisTab::ChannelsAndExport, "📋 Channels & Export"),
                            (IbtAnalysisTab::TiresAndPressures, "🛞 Tires & Pressures"),
                            (IbtAnalysisTab::ShockHistograms, "📊 Shock Histograms"),
                            (IbtAnalysisTab::RideHeightAndRake, "📐 Ride Height & Rake"),
                            (IbtAnalysisTab::InputsAndSpeed, "📈 Inputs & Speed"),
                        ];
                        for (tab_id, label) in tabs {
                            let active = self.active_tab == tab_id;
                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .size(12.0)
                                    .color(if active { egui::Color32::BLACK } else { white })
                                    .strong(),
                            )
                            .fill(if active { primary } else { elevated })
                            .rounding(egui::Rounding::same(6.0));
                            if ui.add(btn).clicked() {
                                self.active_tab = tab_id;
                            }
                        }
                    });
                });
            });

        ui.add_space(8.0);

        // 2. MAIN 2-COLUMN LAYOUT: LEFT FILE/LAP SIDEBAR + RIGHT TELEMETRY CANVAS
        let avail_h = ui.available_height().max(400.0);
        ui.horizontal_top(|ui| {
            // --- LEFT SIDEBAR: FILE BROWSER, METADATA & LAP SLICING ---
            ui.allocate_ui_with_layout(
                egui::vec2(315.0, avail_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_source("ibt_left_sidebar_scroll")
                        .max_height(avail_h)
                        .show(ui, |ui| {
                            // Card 1: Telemetry Folder & File Ingestion
                            egui::Frame::none()
                                .fill(panel_bg)
                                .stroke(egui::Stroke::new(1.0, divider))
                                .rounding(egui::Rounding::same(10.0))
                                .inner_margin(10.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("📂 TELEMETRY (.IBT) FILES")
                                                .color(primary)
                                                .size(11.5)
                                                .strong(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                let demo_btn = egui::Button::new(
                                                    egui::RichText::new("🧪 Load Demo .IBT")
                                                        .size(11.0)
                                                        .color(white)
                                                        .strong(),
                                                )
                                                .fill(with_alpha(secondary, 80));
                                                if ui
                                                    .add(demo_btn)
                                                    .on_hover_text("Generate and load a complete 3-lap 60Hz .ibt telemetry dataset in memory")
                                                    .clicked()
                                                {
                                                    self.trigger_load_demo(ctx.clone());
                                                }
                                            },
                                        );
                                    });

                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        let resp = ui.add(
                                            egui::TextEdit::singleline(&mut self.directory_input)
                                                .desired_width(220.0)
                                                .hint_text("%USERPROFILE%\\Documents\\iRacing\\telemetry"),
                                        );
                                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            self.refresh_directory();
                                        }
                                        if ui
                                            .add(egui::Button::new("🔄").fill(elevated))
                                            .on_hover_text("Rescan telemetry directory for .ibt files")
                                            .clicked()
                                        {
                                            self.refresh_directory();
                                        }
                                        if ui
                                            .add(egui::Button::new("⌂").fill(elevated))
                                            .on_hover_text("Reset to default iRacing telemetry directory")
                                            .clicked()
                                        {
                                            self.directory_input =
                                                default_telemetry_dir().to_string_lossy().to_string();
                                            self.refresh_directory();
                                        }
                                    });

                                    ui.add_space(4.0);
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.file_filter)
                                            .desired_width(f32::INFINITY)
                                            .hint_text("🔍 Filter .ibt files by car or track..."),
                                    );

                                    if let LoadWorkerState::Loading { ref file_name, .. } = self.worker {
                                        ui.add_space(6.0);
                                        ui.horizontal(|ui| {
                                            ui.spinner();
                                            ui.label(
                                                egui::RichText::new(format!("Ingesting {}...", file_name))
                                                    .color(primary)
                                                    .size(11.5),
                                            );
                                        });
                                    }

                                    if let Some(ref err) = self.status_error {
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new(format!("⚠ {}", err)).color(red).size(11.5));
                                    }

                                    ui.add_space(4.0);
                                    let filter_lower = self.file_filter.trim().to_lowercase();
                                    let matching_files: Vec<IbtFileEntry> = self
                                        .scanned_files
                                        .iter()
                                        .filter(|f| {
                                            filter_lower.is_empty()
                                                || f.file_name.to_lowercase().contains(&filter_lower)
                                        })
                                        .cloned()
                                        .collect();

                                    if matching_files.is_empty() {
                                        egui::Frame::none()
                                            .fill(elevated)
                                            .rounding(egui::Rounding::same(6.0))
                                            .inner_margin(8.0)
                                            .show(ui, |ui| {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "No .ibt files found in this folder.\nClick '🧪 Load Demo .IBT' above to test with a 60Hz multi-lap dataset, or enter a custom path.",
                                                    )
                                                    .color(muted)
                                                    .size(11.0),
                                                );
                                            });
                                    } else {
                                        egui::ScrollArea::vertical()
                                            .id_source("ibt_file_list_scroll")
                                            .max_height(155.0)
                                            .show(ui, |ui| {
                                                for entry in matching_files {
                                                    let is_sel = self
                                                        .selected_file_path
                                                        .as_ref()
                                                        .map_or(false, |p| p == &entry.path);
                                                    let row_bg = if is_sel {
                                                        with_alpha(primary, 32)
                                                    } else {
                                                        elevated
                                                    };
                                                    let stroke = if is_sel {
                                                        egui::Stroke::new(1.0, primary)
                                                    } else {
                                                        egui::Stroke::NONE
                                                    };

                                                    let resp = egui::Frame::none()
                                                        .fill(row_bg)
                                                        .stroke(stroke)
                                                        .rounding(egui::Rounding::same(6.0))
                                                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                                                        .show(ui, |ui| {
                                                            ui.set_width(ui.available_width());
                                                            ui.label(
                                                                egui::RichText::new(&entry.file_name)
                                                                    .color(if is_sel { primary } else { white })
                                                                    .size(11.5)
                                                                    .strong(),
                                                            );
                                                            ui.horizontal(|ui| {
                                                                ui.label(
                                                                    egui::RichText::new(format!(
                                                                        "{:.1} MB · {}",
                                                                        entry.size_mb(),
                                                                        entry.modified_display
                                                                    ))
                                                                    .color(muted)
                                                                    .size(10.5),
                                                                );
                                                            });
                                                        })
                                                        .response
                                                        .interact(egui::Sense::click());

                                                    if resp.clicked() {
                                                        self.trigger_load_file(entry.path.clone(), ctx.clone());
                                                    }
                                                    ui.add_space(3.0);
                                                }
                                            });
                                    }
                                });

                            ui.add_space(8.0);

                            // Card 2 & 3: Session Metadata & Lap Slicing (if session is loaded)
                            if let Some(ref sess) = self.session {
                                let meta = &sess.metadata;
                                egui::Frame::none()
                                    .fill(panel_bg)
                                    .stroke(egui::Stroke::new(1.0, divider))
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(10.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("📋 SESSION YAML HEADER")
                                                .color(primary)
                                                .size(11.5)
                                                .strong(),
                                        );
                                        ui.add_space(4.0);
                                        egui::Grid::new("ibt_meta_grid")
                                            .num_columns(2)
                                            .spacing([10.0, 4.0])
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new("Track:").color(muted).size(11.5));
                                                let trk_len_conv = crate::convert_unit(&meta.track_length, use_metric);
                                                let trk_base = if meta.track_config_name.is_empty() {
                                                    meta.track_display_name.clone()
                                                } else {
                                                    format!("{} ({})", meta.track_display_name, meta.track_config_name)
                                                };
                                                let trk = if !trk_len_conv.is_empty() && trk_len_conv != "—" {
                                                    format!("{} · {}", trk_base, trk_len_conv)
                                                } else {
                                                    trk_base
                                                };
                                                ui.label(egui::RichText::new(trk).color(white).size(11.5).strong());
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Car:").color(muted).size(11.5));
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "#{} {}",
                                                        meta.car_number, meta.car_name
                                                    ))
                                                    .color(white)
                                                    .size(11.5),
                                                );
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Driver:").color(muted).size(11.5));
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} ({} iR)",
                                                        meta.driver_name, meta.irating
                                                    ))
                                                    .color(white)
                                                    .size(11.5),
                                                );
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Setup:").color(muted).size(11.5));
                                                ui.label(
                                                    egui::RichText::new(&meta.setup_name)
                                                        .color(primary)
                                                        .size(11.5)
                                                        .strong(),
                                                );
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Weather:").color(muted).size(11.5));
                                                let air_conv = crate::convert_unit(&meta.air_temp, use_metric);
                                                let trk_conv = crate::convert_unit(&meta.track_temp, use_metric);
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "Air {} · Trk {}",
                                                        air_conv, trk_conv
                                                    ))
                                                    .color(white)
                                                    .size(11.5),
                                                );
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Wind / RH:").color(muted).size(11.5));
                                                let wind_spd_conv = crate::convert_unit(&meta.wind_speed, use_metric);
                                                let wind_dir_conv = crate::format_wind_dir(&meta.wind_dir);
                                                let rh_conv = crate::convert_unit(&meta.relative_humidity, use_metric);
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} {} · RH {}",
                                                        wind_spd_conv, wind_dir_conv, rh_conv
                                                    ))
                                                    .color(white)
                                                    .size(11.5),
                                                );
                                                ui.end_row();

                                                ui.label(egui::RichText::new("Date / Rate:").color(muted).size(11.5));
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} · {} Hz ({} ticks)",
                                                        meta.session_date, meta.tick_rate, meta.total_ticks
                                                    ))
                                                    .color(muted)
                                                    .size(11.0),
                                                );
                                                ui.end_row();
                                            });
                                    });

                                ui.add_space(8.0);

                                // Card 3: Lap Slicing, Indexing & Alignment
                                egui::Frame::none()
                                    .fill(panel_bg)
                                    .stroke(egui::Stroke::new(1.0, divider))
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(10.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("⏱ LAP SLICING & COMPARISON")
                                                    .color(primary)
                                                    .size(11.5)
                                                    .strong(),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.checkbox(&mut self.compare_enabled, "Compare (A vs B)");
                                                },
                                            );
                                        });

                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("X-Axis:").color(muted).size(11.5));
                                            if ui
                                                .selectable_label(
                                                    self.x_axis_mode == XAxisMode::LapDistPct,
                                                    "LapDistPct (%)",
                                                )
                                                .clicked()
                                            {
                                                self.x_axis_mode = XAxisMode::LapDistPct;
                                            }
                                            if ui
                                                .selectable_label(
                                                    self.x_axis_mode == XAxisMode::LapDistMeters,
                                                    if use_metric { "LapDist (m)" } else { "LapDist (ft)" },
                                                )
                                                .clicked()
                                            {
                                                self.x_axis_mode = XAxisMode::LapDistMeters;
                                            }
                                        });

                                        if self.compare_enabled && sess.laps.len() >= 2 {
                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new("Target (Lap B):")
                                                        .color(secondary)
                                                        .size(11.5)
                                                        .strong(),
                                                );
                                                let cur_b_label = sess
                                                    .laps
                                                    .get(self.selected_lap_b_idx)
                                                    .map(|l| {
                                                        format!(
                                                            "Lap {} ({}) [{}]",
                                                            l.lap_number,
                                                            format_time(l.duration_secs),
                                                            l.status_badge()
                                                        )
                                                    })
                                                    .unwrap_or_else(|| "Select Lap B".to_string());

                                                egui::ComboBox::from_id_source("ibt_lap_b_combo")
                                                    .selected_text(cur_b_label)
                                                    .width(185.0)
                                                    .show_ui(ui, |ui| {
                                                        for l in &sess.laps {
                                                            let text = format!(
                                                                "Lap {} · {} ({})",
                                                                l.lap_number,
                                                                format_time(l.duration_secs),
                                                                l.status_badge()
                                                            );
                                                            ui.selectable_value(
                                                                &mut self.selected_lap_b_idx,
                                                                l.lap_index,
                                                                text,
                                                            );
                                                        }
                                                    });
                                            });
                                        }

                                        ui.add_space(6.0);
                                        ui.label(
                                            egui::RichText::new("Click a row to select Primary Lap A:")
                                                .color(muted)
                                                .size(10.5),
                                        );
                                        ui.add_space(2.0);

                                        egui::ScrollArea::vertical()
                                            .id_source("ibt_laps_table_scroll")
                                            .max_height(220.0)
                                            .show(ui, |ui| {
                                                for l in &sess.laps {
                                                    let is_a = l.lap_index == self.selected_lap_a_idx;
                                                    let is_b = self.compare_enabled
                                                        && l.lap_index == self.selected_lap_b_idx;

                                                    let row_fill = if is_a {
                                                        with_alpha(primary, 36)
                                                    } else if is_b {
                                                        with_alpha(secondary, 36)
                                                    } else {
                                                        elevated
                                                    };

                                                    let border = if is_a {
                                                        egui::Stroke::new(1.2, primary)
                                                    } else if is_b {
                                                        egui::Stroke::new(1.2, secondary)
                                                    } else {
                                                        egui::Stroke::NONE
                                                    };

                                                    let badge_col = if l.is_personal_best {
                                                        purple
                                                    } else if l.is_valid_flying {
                                                        green
                                                    } else if l.is_out_lap || l.is_in_lap {
                                                        amber
                                                    } else {
                                                        red
                                                    };

                                                    let resp = egui::Frame::none()
                                                        .fill(row_fill)
                                                        .stroke(border)
                                                        .rounding(egui::Rounding::same(6.0))
                                                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                                                        .show(ui, |ui| {
                                                            ui.horizontal(|ui| {
                                                                let tag = if is_a {
                                                                    format!("L{} [A]", l.lap_number)
                                                                } else if is_b {
                                                                    format!("L{} [B]", l.lap_number)
                                                                } else {
                                                                    format!("L{}", l.lap_number)
                                                                };
                                                                ui.label(
                                                                    egui::RichText::new(tag)
                                                                        .color(if is_a {
                                                                            primary
                                                                        } else if is_b {
                                                                            secondary
                                                                        } else {
                                                                            white
                                                                        })
                                                                        .size(12.0)
                                                                        .strong(),
                                                                );
                                                                ui.label(
                                                                    egui::RichText::new(format_time(
                                                                        l.duration_secs,
                                                                    ))
                                                                    .color(white)
                                                                    .monospace()
                                                                    .size(12.0),
                                                                );
                                                                ui.with_layout(
                                                                    egui::Layout::right_to_left(
                                                                        egui::Align::Center,
                                                                    ),
                                                                    |ui| {
                                                                        ui.label(
                                                                            egui::RichText::new(
                                                                                l.status_badge(),
                                                                            )
                                                                            .color(badge_col)
                                                                            .size(10.5)
                                                                            .strong(),
                                                                        );
                                                                    },
                                                                );
                                                            });
                                                        })
                                                        .response
                                                        .interact(egui::Sense::click());

                                                    if resp.clicked() {
                                                        self.selected_lap_a_idx = l.lap_index;
                                                    }
                                                    ui.add_space(3.0);
                                                }
                                            });
                                    });
                            }
                        });
                },
            );

            ui.add_space(8.0);

            // --- RIGHT MAIN WORKSPACE: TELEMETRY CHARTS, HISTOGRAMS & CARDS ---
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), avail_h),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let sess = match self.session.clone() {
                        Some(s) if !s.analyzed_laps.is_empty() => s,
                        _ => {
                            // Empty State Hero Card with 1-Click Demo Loader
                            egui::Frame::none()
                                .fill(panel_bg)
                                .stroke(egui::Stroke::new(1.0, divider))
                                .rounding(egui::Rounding::same(12.0))
                                .inner_margin(32.0)
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(40.0);
                                        ui.label(
                                            egui::RichText::new("📊 OFFLINE .IBT TELEMETRY & VEHICLE DYNAMICS LAB")
                                                .font(display_font(22.0))
                                                .color(white),
                                        );
                                        ui.add_space(8.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "Select an iRacing .ibt telemetry file from the left sidebar (%USERPROFILE%\\Documents\\iRacing\\telemetry)\nor load the built-in 60Hz Porsche 963 GTP @ Watkins Glen dataset to inspect:",
                                            )
                                            .color(muted)
                                            .size(13.0),
                                        );
                                        ui.add_space(16.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "• Distance-Normalized Driver Inputs (Speed, Throttle, Brake, Steering Angle, Gear, RPM & Lap Time Delta)\n\
                                                 • Dynamic Suspension Ride Heights (LF, RF, LR, RR, Center Floor CFSR), Pitch Rake & Lateral Side Rake\n\
                                                 • 4-Corner Shock / Damper Velocity Distribution Histograms (-150 to +150 mm/s Low/High-Speed Bump & Rebound)\n\
                                                 • 4-Corner Tire Carcass/Surface Temps (°F/°C), Inside-to-Outside Spreads (ΔT), Center Wear Bias & Pressures (PSI)",
                                            )
                                            .color(white)
                                            .size(12.5),
                                        );
                                        ui.add_space(22.0);
                                        let btn = egui::Button::new(
                                            egui::RichText::new("🧪 Load 3-Lap 60Hz Demo .IBT Telemetry Session")
                                                .color(egui::Color32::BLACK)
                                                .size(14.0)
                                                .strong(),
                                        )
                                        .fill(primary)
                                        .min_size(egui::vec2(340.0, 38.0));
                                        if ui.add(btn).clicked() {
                                            self.trigger_load_demo(ctx.clone());
                                        }
                                        ui.add_space(40.0);
                                    });
                                });
                            return;
                        }
                    };

                    let lap_a_idx = self.selected_lap_a_idx.min(sess.analyzed_laps.len() - 1);
                    let lap_a = &sess.analyzed_laps[lap_a_idx];
                    let lap_b = if self.compare_enabled && sess.analyzed_laps.len() >= 2 {
                        let b_idx = self.selected_lap_b_idx.min(sess.analyzed_laps.len() - 1);
                        Some(&sess.analyzed_laps[b_idx])
                    } else {
                        None
                    };

                    let norm_a = normalize_lap(lap_a, 300);
                    let comparison = lap_b.map(|lb| compare_laps(lap_a, lb, 300));

                    egui::ScrollArea::vertical()
                        .id_source("ibt_right_canvas_scroll")
                        .max_height(avail_h)
                        .show(ui, |ui| {
                            match self.active_tab {
                                IbtAnalysisTab::InputsAndSpeed => {
                                    self.render_inputs_and_speed_tab(
                                        ui,
                                        settings,
                                        lap_a,
                                        lap_b,
                                        &norm_a,
                                        comparison.as_ref(),
                                    );
                                }
                                IbtAnalysisTab::RideHeightAndRake => {
                                    self.render_ride_height_and_rake_tab(
                                        ui,
                                        settings,
                                        lap_a,
                                        lap_b,
                                        &norm_a,
                                        comparison.as_ref(),
                                    );
                                }
                                IbtAnalysisTab::ShockHistograms => {
                                    self.render_shock_histograms_tab(ui, settings, lap_a, lap_b);
                                }
                                IbtAnalysisTab::TiresAndPressures => {
                                    self.render_tires_and_pressures_tab(ui, settings, lap_a, lap_b);
                                }
                                IbtAnalysisTab::ChannelsAndExport => {
                                    self.render_channels_and_export_tab(
                                        ui,
                                        settings,
                                        &sess,
                                        lap_a,
                                        lap_b,
                                        toast_out,
                                    );
                                }
                            }
                        });
                },
            );
        });
    }

    // ========================================================================
    // TAB 1: DRIVER INPUTS & SPEED TRACES
    // ========================================================================
    fn render_inputs_and_speed_tab(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        lap_a: &LapAnalysis,
        lap_b: Option<&LapAnalysis>,
        norm_a: &NormalizedLapTrace,
        comparison: Option<&LapComparisonReport>,
    ) {
        let primary = settings.primary_accent();
        let secondary = settings.secondary_accent();
        let white = settings.text_color();
        let muted = settings.text_muted();
        let green = settings.status_green();
        let red = settings.status_red();
        let use_metric = settings.use_metric;

        // Top KPI Summary Strip
        ui.columns(5, |cols| {
            let (spd_max, spd_min, spd_avg, spd_unit) = if use_metric {
                (
                    lap_a.inputs.speed_kph_stats.max,
                    lap_a.inputs.speed_kph_stats.min,
                    lap_a.inputs.speed_kph_stats.avg,
                    "km/h",
                )
            } else {
                (
                    lap_a.inputs.speed_mph_stats.max,
                    lap_a.inputs.speed_mph_stats.min,
                    lap_a.inputs.speed_mph_stats.avg,
                    "mph",
                )
            };

            draw_kpi_card(
                &mut cols[0],
                settings,
                &format!("LAP {} TIME [A]", lap_a.descriptor.lap_number),
                &format_time(lap_a.descriptor.duration_secs),
                &if let Some(lb) = lap_b {
                    let diff = lb.descriptor.duration_secs - lap_a.descriptor.duration_secs;
                    format!("vs L{} [B]: {:+.3}s", lb.descriptor.lap_number, diff)
                } else {
                    format!("Status: {}", lap_a.descriptor.status_badge())
                },
                primary,
            );

            draw_kpi_card(
                &mut cols[1],
                settings,
                &format!("PEAK / MIN SPEED ({})", spd_unit.to_uppercase()),
                &format!("{:.1} / {:.1}", spd_max, spd_min),
                &format!("Lap Avg: {:.1} {}", spd_avg, spd_unit),
                white,
            );

            draw_kpi_card(
                &mut cols[2],
                settings,
                "FULL THROTTLE TIME",
                &format!("{:.1}%", lap_a.inputs.full_throttle_pct_of_lap),
                &format!("Coasting: {:.1}%", lap_a.inputs.coasting_pct_of_lap),
                green,
            );

            draw_kpi_card(
                &mut cols[3],
                settings,
                "BRAKING ZONE TIME",
                &format!("{:.1}%", lap_a.inputs.braking_pct_of_lap),
                &format!("Peak Brake: {:.0}%", lap_a.inputs.brake_stats.max),
                red,
            );

            draw_kpi_card(
                &mut cols[4],
                settings,
                "STEERING & MAX RPM",
                &format!(
                    "{:+.0}° / {:+.0}°",
                    lap_a.inputs.steering_deg_stats.min, lap_a.inputs.steering_deg_stats.max
                ),
                &format!("Peak RPM: {:.0}", lap_a.inputs.rpm_stats.max),
                secondary,
            );
        });

        ui.add_space(8.0);

        // If comparing two laps, show Time Delta (B - A) lane first
        if let Some(comp) = comparison {
            let delta_label = format!(
                "Time Delta Δt (L{} [B] - L{} [A])",
                comp.lap_b.lap_number, comp.lap_a.lap_number
            );
            let series = vec![TraceSeries {
                label: &delta_label,
                data: &comp.time_delta_s,
                color: secondary,
                stroke_width: 2.0,
            }];
            draw_synchronized_trace_lane(
                ui,
                settings,
                "TIME DELTA (Δs) · NEGATIVE = LAP B AHEAD",
                "s",
                &comp.grid_dist_pct,
                &comp.grid_dist_m,
                self.x_axis_mode,
                &series,
                110.0,
                true,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 1: Speed (mph or km/h)
        let spd_a = if use_metric {
            norm_a.speed_kph.as_deref()
        } else {
            norm_a.speed_mph.as_deref()
        };
        let spd_b = comparison.and_then(|c| {
            if use_metric {
                c.lap_b.speed_kph.as_deref()
            } else {
                c.lap_b.speed_mph.as_deref()
            }
        });
        if let Some(sa) = spd_a {
            let a_label = format!("Lap {} [A] Speed", norm_a.lap_number);
            let mut series = vec![TraceSeries {
                label: &a_label,
                data: sa,
                color: primary,
                stroke_width: 2.0,
            }];
            let b_label = comparison
                .map(|c| format!("Lap {} [B] Speed", c.lap_b.lap_number))
                .unwrap_or_default();
            if let Some(sb) = spd_b {
                series.push(TraceSeries {
                    label: &b_label,
                    data: sb,
                    color: secondary,
                    stroke_width: 1.6,
                });
            }
            draw_synchronized_trace_lane(
                ui,
                settings,
                if use_metric { "VEHICLE SPEED (KM/H)" } else { "VEHICLE SPEED (MPH)" },
                if use_metric { "km/h" } else { "mph" },
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &series,
                155.0,
                false,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 2: Throttle & Brake (0 - 100%)
        if norm_a.throttle_pct.is_some() || norm_a.brake_pct.is_some() {
            let mut series = Vec::new();
            if let Some(ref thr) = norm_a.throttle_pct {
                series.push(TraceSeries {
                    label: "Throttle [A]",
                    data: thr,
                    color: green,
                    stroke_width: 1.9,
                });
            }
            if let Some(ref brk) = norm_a.brake_pct {
                series.push(TraceSeries {
                    label: "Brake [A]",
                    data: brk,
                    color: red,
                    stroke_width: 1.9,
                });
            }
            if let Some(comp) = comparison {
                if let Some(ref thr_b) = comp.lap_b.throttle_pct {
                    series.push(TraceSeries {
                        label: "Throttle [B]",
                        data: thr_b,
                        color: with_alpha(green, 125),
                        stroke_width: 1.2,
                    });
                }
                if let Some(ref brk_b) = comp.lap_b.brake_pct {
                    series.push(TraceSeries {
                        label: "Brake [B]",
                        data: brk_b,
                        color: with_alpha(red, 125),
                        stroke_width: 1.2,
                    });
                }
            }
            draw_synchronized_trace_lane(
                ui,
                settings,
                "DRIVER PEDAL INPUTS · THROTTLE & BRAKE (0–100%)",
                "%",
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &series,
                145.0,
                false,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 3: Steering Wheel Angle (deg)
        if let Some(ref steer_a) = norm_a.steering_deg {
            let mut series = vec![TraceSeries {
                label: "Steering Angle [A]",
                data: steer_a,
                color: egui::Color32::from_rgb(0, 229, 200),
                stroke_width: 1.8,
            }];
            if let Some(comp) = comparison {
                if let Some(ref steer_b) = comp.lap_b.steering_deg {
                    series.push(TraceSeries {
                        label: "Steering Angle [B]",
                        data: steer_b,
                        color: secondary,
                        stroke_width: 1.3,
                    });
                }
            }
            draw_synchronized_trace_lane(
                ui,
                settings,
                "STEERING WHEEL ANGLE (DEGREES)",
                "°",
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &series,
                130.0,
                true,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 4: Engine RPM & Gear
        if let Some(ref rpm_a) = norm_a.rpm {
            let mut series = vec![TraceSeries {
                label: "RPM [A]",
                data: rpm_a,
                color: settings.status_amber(),
                stroke_width: 1.7,
            }];
            if let Some(comp) = comparison {
                if let Some(ref rpm_b) = comp.lap_b.rpm {
                    series.push(TraceSeries {
                        label: "RPM [B]",
                        data: rpm_b,
                        color: secondary,
                        stroke_width: 1.3,
                    });
                }
            }
            draw_synchronized_trace_lane(
                ui,
                settings,
                "DRIVETRAIN · ENGINE RPM",
                "rpm",
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &series,
                115.0,
                false,
                &mut self.hover_dist_pct,
            );
        }

        let _ = muted;
    }

    // ========================================================================
    // TAB 2: SUSPENSION DYNAMICS, RIDE HEIGHTS, PITCH RAKE & SIDE RAKE
    // ========================================================================
    fn render_ride_height_and_rake_tab(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        lap_a: &LapAnalysis,
        _lap_b: Option<&LapAnalysis>,
        norm_a: &NormalizedLapTrace,
        comparison: Option<&LapComparisonReport>,
    ) {
        let primary = settings.primary_accent();
        let secondary = settings.secondary_accent();
        let white = settings.text_color();
        let green = settings.status_green();
        let amber = settings.status_amber();
        let purple = settings.status_purple();
        let use_metric = settings.use_metric;

        let scale = if use_metric { 1.0 } else { MM_TO_INCHES };
        let unit = if use_metric { "mm" } else { "in" };
        let susp = &lap_a.suspension;

        // Top KPI Row: Front RH, Rear RH, Dynamic Longitudinal Rake, Dynamic Side Rake, Center Floor CFSR
        ui.columns(5, |cols| {
            draw_kpi_card(
                &mut cols[0],
                settings,
                &format!("FRONT RIDE HEIGHT ({})", unit.to_uppercase()),
                &format!("{:.2} {}", susp.front_rh_stats_mm.avg * scale, unit),
                &format!(
                    "Min {:.2} · Max {:.2} {}",
                    susp.front_rh_stats_mm.min * scale,
                    susp.front_rh_stats_mm.max * scale,
                    unit
                ),
                egui::Color32::from_rgb(34, 211, 238),
            );

            draw_kpi_card(
                &mut cols[1],
                settings,
                &format!("REAR RIDE HEIGHT ({})", unit.to_uppercase()),
                &format!("{:.2} {}", susp.rear_rh_stats_mm.avg * scale, unit),
                &format!(
                    "Min {:.2} · Max {:.2} {}",
                    susp.rear_rh_stats_mm.min * scale,
                    susp.rear_rh_stats_mm.max * scale,
                    unit
                ),
                green,
            );

            draw_kpi_card(
                &mut cols[2],
                settings,
                "DYNAMIC PITCH RAKE (REAR - FRONT)",
                &format!("{:+.2} {}", susp.rake_stats_mm.avg * scale, unit),
                &format!(
                    "Range: {:+.2} to {:+.2} {} (+Nose-Down)",
                    susp.rake_stats_mm.min * scale,
                    susp.rake_stats_mm.max * scale,
                    unit
                ),
                primary,
            );

            draw_kpi_card(
                &mut cols[3],
                settings,
                "SIDE RAKE / LATERAL ROLL (L - R)",
                &format!("{:+.2} {}", susp.side_rake_stats_mm.avg * scale, unit),
                &format!(
                    "F-Split {:+.2} · R-Split {:+.2} {}",
                    susp.front_split_stats_mm.avg * scale,
                    susp.rear_split_stats_mm.avg * scale,
                    unit
                ),
                amber,
            );

            if let Some(cfsr) = susp.cfsr_stats_mm {
                draw_kpi_card(
                    &mut cols[4],
                    settings,
                    &format!("CENTER FLOOR CFSR ({})", unit.to_uppercase()),
                    &format!("{:.2} {}", cfsr.avg * scale, unit),
                    &format!("Min {:.2} · Max {:.2} {}", cfsr.min * scale, cfsr.max * scale, unit),
                    purple,
                );
            } else {
                draw_kpi_card(
                    &mut cols[4],
                    settings,
                    "CENTER FLOOR CFSR",
                    "—",
                    "Not logged by this vehicle",
                    white,
                );
            }
        });

        ui.add_space(8.0);

        let scale_vec = |opt: &Option<Vec<f32>>| -> Option<Vec<f32>> {
            opt.as_ref()
                .map(|v| v.iter().map(|&x| x * scale).collect::<Vec<f32>>())
        };

        // Lane 1: Front Ride Heights & Center Splitter (LF, RF, CFSR)
        let lf_a_s = scale_vec(&norm_a.lf_rh_mm);
        let rf_a_s = scale_vec(&norm_a.rf_rh_mm);
        let cfsr_a_s = scale_vec(&norm_a.cfsr_rh_mm);
        let lf_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.lf_rh_mm));
        let rf_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.rf_rh_mm));
        let cfsr_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.cfsr_rh_mm));

        let mut front_rh_series = Vec::new();
        if let Some(ref v) = lf_a_s {
            front_rh_series.push(TraceSeries {
                label: "LF Ride Height [A]",
                data: v,
                color: egui::Color32::from_rgb(34, 211, 238),
                stroke_width: 1.9,
            });
        }
        if let Some(ref v) = rf_a_s {
            front_rh_series.push(TraceSeries {
                label: "RF Ride Height [A]",
                data: v,
                color: primary,
                stroke_width: 1.9,
            });
        }
        if let Some(ref v) = cfsr_a_s {
            front_rh_series.push(TraceSeries {
                label: "Center Splitter (CFSR) [A]",
                data: v,
                color: purple,
                stroke_width: 1.8,
            });
        }
        if let Some(ref v) = lf_b_s {
            front_rh_series.push(TraceSeries {
                label: "LF [B]",
                data: v,
                color: with_alpha(egui::Color32::from_rgb(34, 211, 238), 120),
                stroke_width: 1.2,
            });
        }
        if let Some(ref v) = rf_b_s {
            front_rh_series.push(TraceSeries {
                label: "RF [B]",
                data: v,
                color: with_alpha(primary, 120),
                stroke_width: 1.2,
            });
        }
        if let Some(ref v) = cfsr_b_s {
            front_rh_series.push(TraceSeries {
                label: "CFSR [B]",
                data: v,
                color: with_alpha(purple, 120),
                stroke_width: 1.2,
            });
        }

        if !front_rh_series.is_empty() {
            draw_synchronized_trace_lane(
                ui,
                settings,
                &format!("FRONT RIDE HEIGHTS & CENTER SPLITTER (LF, RF, CFSR) ({})", unit.to_uppercase()),
                unit,
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &front_rh_series,
                160.0,
                false,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 2: Rear Ride Heights (LR, RR)
        let lr_a_s = scale_vec(&norm_a.lr_rh_mm);
        let rr_a_s = scale_vec(&norm_a.rr_rh_mm);
        let lr_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.lr_rh_mm));
        let rr_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.rr_rh_mm));

        let mut rear_rh_series = Vec::new();
        if let Some(ref v) = lr_a_s {
            rear_rh_series.push(TraceSeries {
                label: "LR Ride Height [A]",
                data: v,
                color: green,
                stroke_width: 1.9,
            });
        }
        if let Some(ref v) = rr_a_s {
            rear_rh_series.push(TraceSeries {
                label: "RR Ride Height [A]",
                data: v,
                color: egui::Color32::from_rgb(255, 115, 145),
                stroke_width: 1.9,
            });
        }
        if let Some(ref v) = lr_b_s {
            rear_rh_series.push(TraceSeries {
                label: "LR [B]",
                data: v,
                color: with_alpha(green, 120),
                stroke_width: 1.2,
            });
        }
        if let Some(ref v) = rr_b_s {
            rear_rh_series.push(TraceSeries {
                label: "RR [B]",
                data: v,
                color: with_alpha(egui::Color32::from_rgb(255, 115, 145), 120),
                stroke_width: 1.2,
            });
        }

        if !rear_rh_series.is_empty() {
            draw_synchronized_trace_lane(
                ui,
                settings,
                &format!("REAR RIDE HEIGHTS (LR, RR) ({})", unit.to_uppercase()),
                unit,
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &rear_rh_series,
                160.0,
                false,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 3: Dynamic Pitch / Longitudinal Rake = Rear RH - Front RH
        let rake_a_s = scale_vec(&norm_a.rake_mm);
        let rake_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.rake_mm));
        if let Some(ref ra) = rake_a_s {
            let mut series = vec![TraceSeries {
                label: "Longitudinal Rake [A] (Rear - Front)",
                data: ra,
                color: primary,
                stroke_width: 2.0,
            }];
            if let Some(ref rb) = rake_b_s {
                series.push(TraceSeries {
                    label: "Longitudinal Rake [B]",
                    data: rb,
                    color: secondary,
                    stroke_width: 1.5,
                });
            }
            draw_synchronized_trace_lane(
                ui,
                settings,
                &format!(
                    "DYNAMIC LONGITUDINAL PITCH RAKE ({}) · +VALUES = TAIL-UP / NOSE-DOWN",
                    unit.to_uppercase()
                ),
                unit,
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &series,
                150.0,
                true,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 4: Dynamic Roll / Side Rake (Left RH - Right RH)
        let side_a_s = scale_vec(&norm_a.side_rake_mm);
        let side_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.side_rake_mm));
        if let Some(ref sa) = side_a_s {
            let mut side_series = vec![TraceSeries {
                label: "Side Rake [A] (Left - Right)",
                data: sa,
                color: amber,
                stroke_width: 2.0,
            }];
            if let Some(ref sb) = side_b_s {
                side_series.push(TraceSeries {
                    label: "Side Rake [B]",
                    data: sb,
                    color: secondary,
                    stroke_width: 1.5,
                });
            }
            draw_synchronized_trace_lane(
                ui,
                settings,
                &format!(
                    "DYNAMIC ROLL / SIDE RAKE (LEFT RH - RIGHT RH) ({})",
                    unit.to_uppercase()
                ),
                unit,
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &side_series,
                145.0,
                true,
                &mut self.hover_dist_pct,
            );
            ui.add_space(6.0);
        }

        // Lane 5: Lateral Corner Splits (Front LF - RF & Rear LR - RR)
        let fsplit_s = scale_vec(&norm_a.front_split_mm);
        let rsplit_s = scale_vec(&norm_a.rear_split_mm);

        let mut split_series = Vec::new();
        if let Some(ref fs) = fsplit_s {
            split_series.push(TraceSeries {
                label: "Front Split [A] (LF - RF)",
                data: fs,
                color: egui::Color32::from_rgb(34, 211, 238),
                stroke_width: 1.8,
            });
        }
        if let Some(ref rs) = rsplit_s {
            split_series.push(TraceSeries {
                label: "Rear Split [A] (LR - RR)",
                data: rs,
                color: green,
                stroke_width: 1.8,
            });
        }

        if !split_series.is_empty() {
            draw_synchronized_trace_lane(
                ui,
                settings,
                &format!(
                    "LATERAL CORNER SPLITS · FRONT (LF - RF) & REAR (LR - RR) ({})",
                    unit.to_uppercase()
                ),
                unit,
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &split_series,
                145.0,
                true,
                &mut self.hover_dist_pct,
            );
        }
    }

    // ========================================================================
    // TAB 3: SHOCK / DAMPER VELOCITY DISTRIBUTION HISTOGRAMS
    // ========================================================================
    fn render_shock_histograms_tab(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        lap_a: &LapAnalysis,
        _lap_b: Option<&LapAnalysis>,
    ) {
        let panel_bg = settings.panel_color();
        let divider = settings.divider_color();
        let white = settings.text_color();
        let muted = settings.text_muted();
        let primary = settings.primary_accent();

        if !lap_a.shocks.has_any_shock_data {
            egui::Frame::none()
                .fill(panel_bg)
                .stroke(egui::Stroke::new(1.0, divider))
                .rounding(egui::Rounding::same(10.0))
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("No shock velocity or shock deflection channels found in this .ibt file.")
                            .color(muted)
                            .size(13.0),
                    );
                });
            return;
        }

        // Legend & Engineering Header
        egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, divider))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "4-CORNER DAMPER VELOCITY DISTRIBUTION — LAP {} ({})",
                            lap_a.descriptor.lap_number,
                            lap_a.shocks.lf.source.label()
                        ))
                        .color(primary)
                        .size(12.0)
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        draw_legend_pill(
                            ui,
                            egui::Color32::from_rgb(245, 158, 11),
                            "Hi-Spd Bump (> +50 mm/s)",
                            white,
                        );
                        draw_legend_pill(
                            ui,
                            egui::Color32::from_rgb(34, 211, 238),
                            "Lo-Spd Bump (0..+50 mm/s)",
                            white,
                        );
                        draw_legend_pill(
                            ui,
                            egui::Color32::from_rgb(52, 211, 153),
                            "Lo-Spd Rebound (-50..0 mm/s)",
                            white,
                        );
                        draw_legend_pill(
                            ui,
                            egui::Color32::from_rgb(168, 85, 247),
                            "Hi-Spd Rebound (< -50 mm/s)",
                            white,
                        );
                    });
                });
            });

        ui.add_space(8.0);

        // 2x2 Grid for LF / RF (top row) and LR / RR (bottom row)
        ui.columns(2, |cols| {
            draw_shock_corner_histogram_card(&mut cols[0], settings, "LEFT FRONT (LF)", &lap_a.shocks.lf);
            draw_shock_corner_histogram_card(&mut cols[1], settings, "RIGHT FRONT (RF)", &lap_a.shocks.rf);
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            draw_shock_corner_histogram_card(&mut cols[0], settings, "LEFT REAR (LR)", &lap_a.shocks.lr);
            draw_shock_corner_histogram_card(&mut cols[1], settings, "RIGHT REAR (RR)", &lap_a.shocks.rr);
        });
    }

    // ========================================================================
    // TAB 4: TIRE SURFACE / GARAGE HOT TEMPERATURES, SPREADS & PRESSURES
    // ========================================================================
    fn render_tires_and_pressures_tab(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        lap_a: &LapAnalysis,
        lap_b: Option<&LapAnalysis>,
    ) {
        let primary = settings.primary_accent();
        let white = settings.text_color();
        let muted = settings.text_muted();
        let amber = settings.status_amber();
        let panel_bg = settings.panel_color();
        let divider = settings.divider_color();

        egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, divider))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "4-CORNER TIRE TEMPERATURES, TREAD WEAR, SPREAD & HOT PRESSURE — LAP {} ({})",
                            lap_a.descriptor.lap_number,
                            lap_a.tires.lf.temp_source.label()
                        ))
                        .color(primary)
                        .size(12.0)
                        .strong(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(
                                "Spread ΔT = T_inside - T_outside · Center Bias = T_center - (T_in + T_out)/2",
                            )
                            .color(muted)
                            .size(11.5),
                        );
                    });
                });

                let lf_static = (lap_a.tires.lf.center_stats_f.max - lap_a.tires.lf.center_stats_f.min).abs() < 0.08
                    && lap_a.tires.lf.wear_center_pct >= 99.9
                    && lap_a.tires.lf.temp_source == TireTempSource::Carcass;
                if lf_static {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "ℹ iRacing Telemetry Note: On-track tire channels (*tempCL/CM/CR) remain at starting temps while driving and latch hot pit/garage readings when stopping in the pit box or exiting the car to the garage.",
                        )
                        .color(amber)
                        .size(11.0),
                    );
                }
            });

        ui.add_space(8.0);

        let _ = white;
        ui.columns(2, |cols| {
            draw_tire_corner_analysis_card(
                &mut cols[0],
                settings,
                "LEFT FRONT (LF) · O · M · I",
                &lap_a.tires.lf,
                lap_b.map(|lb| &lb.tires.lf),
            );
            draw_tire_corner_analysis_card(
                &mut cols[1],
                settings,
                "RIGHT FRONT (RF) · I · M · O",
                &lap_a.tires.rf,
                lap_b.map(|lb| &lb.tires.rf),
            );
        });

        ui.add_space(8.0);

        ui.columns(2, |cols| {
            draw_tire_corner_analysis_card(
                &mut cols[0],
                settings,
                "LEFT REAR (LR) · O · M · I",
                &lap_a.tires.lr,
                lap_b.map(|lb| &lb.tires.lr),
            );
            draw_tire_corner_analysis_card(
                &mut cols[1],
                settings,
                "RIGHT REAR (RR) · I · M · O",
                &lap_a.tires.rr,
                lap_b.map(|lb| &lb.tires.rr),
            );
        });
    }

    // ========================================================================
    // TAB 5: CHANNEL SCHEMA INSPECTOR & TSV CLIPBOARD EXPORT
    // ========================================================================
    fn render_channels_and_export_tab(
        &mut self,
        ui: &mut egui::Ui,
        settings: &AppSettings,
        sess: &IbtSession,
        lap_a: &LapAnalysis,
        lap_b: Option<&LapAnalysis>,
        toast_out: &mut Option<(String, std::time::Instant)>,
    ) {
        let panel_bg = settings.panel_color();
        let elevated = settings.card_elevated_color();
        let divider = settings.divider_color();
        let primary = settings.primary_accent();
        let white = settings.text_color();
        let muted = settings.text_muted();

        egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, divider))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "DISCOVERED .IBT CHANNEL SCHEMA ({} CHANNELS)",
                            sess.channels.len()
                        ))
                        .color(primary)
                        .size(12.0)
                        .strong(),
                    );
                    ui.add_space(12.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.channel_search_filter)
                            .desired_width(260.0)
                            .hint_text("🔍 Search channel name or description..."),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let copy_btn = egui::Button::new(
                            egui::RichText::new("📋 Copy Lap Engineering Report (TSV)")
                                .color(egui::Color32::BLACK)
                                .size(12.0)
                                .strong(),
                        )
                        .fill(primary);
                        if ui.add(copy_btn).clicked() {
                            let tsv = format_ibt_engineering_report_tsv(sess, lap_a, lap_b, settings.use_metric);
                            ui.output_mut(|o| o.copied_text = tsv);
                            *toast_out = Some((
                                "Copied .IBT Lap Engineering Report to Clipboard!".to_string(),
                                std::time::Instant::now(),
                            ));
                        }
                    });
                });

                ui.add_space(8.0);
                let q = self.channel_search_filter.trim().to_lowercase();
                egui::ScrollArea::vertical()
                    .id_source("ibt_channels_schema_scroll")
                    .max_height(460.0)
                    .show(ui, |ui| {
                        egui::Grid::new("ibt_channels_grid")
                            .num_columns(5)
                            .striped(true)
                            .spacing([18.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Channel").color(muted).strong());
                                ui.label(egui::RichText::new("Unit").color(muted).strong());
                                ui.label(egui::RichText::new("Type").color(muted).strong());
                                ui.label(egui::RichText::new("Offset").color(muted).strong());
                                ui.label(egui::RichText::new("Description").color(muted).strong());
                                ui.end_row();

                                for ch in &sess.channels {
                                    if !q.is_empty()
                                        && !ch.name.to_lowercase().contains(&q)
                                        && !ch.desc.to_lowercase().contains(&q)
                                    {
                                        continue;
                                    }
                                    ui.label(
                                        egui::RichText::new(&ch.name)
                                            .color(white)
                                            .monospace()
                                            .strong(),
                                    );
                                    ui.label(egui::RichText::new(&ch.unit).color(primary).monospace());
                                    ui.label(
                                        egui::RichText::new(format!("{:?}[{}]", ch.var_type, ch.count))
                                            .color(muted)
                                            .size(11.5),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("+0x{:04X}", ch.offset))
                                            .color(muted)
                                            .monospace()
                                            .size(11.5),
                                    );
                                    ui.label(egui::RichText::new(&ch.desc).color(white).size(12.0));
                                    ui.end_row();
                                }
                            });
                    });
            });
        let _ = elevated;
    }
}

// ============================================================================
// UI HELPER WIDGETS: KPI CARDS, SYNCHRONIZED TRACE LANES & HISTOGRAMS
// ============================================================================

fn draw_kpi_card(
    ui: &mut egui::Ui,
    settings: &AppSettings,
    title: &str,
    hero_val: &str,
    sub_val: &str,
    accent: egui::Color32,
) {
    egui::Frame::none()
        .fill(settings.panel_color())
        .stroke(egui::Stroke::new(1.0, settings.divider_color()))
        .rounding(egui::Rounding::same(9.0))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(title)
                    .color(settings.text_muted())
                    .size(10.5)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(hero_val)
                    .font(display_font(18.0))
                    .color(accent),
            );
            ui.label(
                egui::RichText::new(sub_val)
                    .color(settings.text_muted())
                    .size(11.0),
            );
        });
}

fn draw_legend_pill(ui: &mut egui::Ui, color: egui::Color32, text: &str, text_col: egui::Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 2.0, color);
        ui.label(egui::RichText::new(text).color(text_col).size(11.0));
    });
}

pub struct TraceSeries<'a> {
    pub label: &'a str,
    pub data: &'a [f32],
    pub color: egui::Color32,
    pub stroke_width: f32,
}

#[allow(clippy::too_many_arguments)]
fn draw_synchronized_trace_lane(
    ui: &mut egui::Ui,
    settings: &AppSettings,
    title: &str,
    unit: &str,
    grid_dist_pct: &[f32],
    grid_dist_m: &[f32],
    x_axis_mode: XAxisMode,
    series: &[TraceSeries<'_>],
    height: f32,
    draw_zero_line: bool,
    shared_hover_pct: &mut Option<f32>,
) {
    let panel_bg = settings.panel_color();
    let elevated = settings.card_elevated_color();
    let divider = settings.divider_color();
    let white = settings.text_color();
    let muted = settings.text_muted();

    egui::Frame::none()
        .fill(panel_bg)
        .stroke(egui::Stroke::new(1.0, divider))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            // Header with title and series hover/summary values
            let hover_idx = shared_hover_pct.and_then(|pct| {
                if grid_dist_pct.is_empty() {
                    None
                } else {
                    let i = ((pct.clamp(0.0, 1.0)) * ((grid_dist_pct.len() - 1) as f32)).round() as usize;
                    Some(i.min(grid_dist_pct.len() - 1))
                }
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).color(white).size(11.5).strong());
                if let Some(idx) = hover_idx {
                    let pct = grid_dist_pct.get(idx).copied().unwrap_or(0.0) * 100.0;
                    let dist_m = grid_dist_m.get(idx).copied().unwrap_or(0.0);
                    let dist_str = if x_axis_mode == XAxisMode::LapDistPct {
                        format!("@ {:.1}% ({:.0} m)", pct, dist_m)
                    } else if settings.use_metric {
                        format!("@ {:.0} m ({:.1}%)", dist_m, pct)
                    } else {
                        format!("@ {:.0} ft ({:.1}%)", dist_m * 3.28084, pct)
                    };
                    ui.label(egui::RichText::new(dist_str).color(muted).monospace().size(11.0));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for s in series.iter().rev() {
                        let val_txt = if let Some(idx) = hover_idx {
                            if let Some(&v) = s.data.get(idx) {
                                format!("{}: {:.2} {}", s.label, v, unit)
                            } else {
                                s.label.to_string()
                            }
                        } else if !s.data.is_empty() {
                            let st = crate::ibt::ChannelStats::from_slice(s.data);
                            format!("{}: [{:.1} .. {:.1} {}]", s.label, st.min, st.max, unit)
                        } else {
                            s.label.to_string()
                        };
                        draw_legend_pill(ui, s.color, &val_txt, white);
                        ui.add_space(6.0);
                    }
                });
            });

            ui.add_space(4.0);

            let avail_w = ui.available_width().max(200.0);
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(avail_w, height), egui::Sense::hover());
            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, 6.0, elevated);
            painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, divider));

            // Compute global y_min and y_max across all series
            let mut y_min = f32::INFINITY;
            let mut y_max = f32::NEG_INFINITY;
            for s in series {
                for &v in s.data {
                    if v.is_finite() {
                        if v < y_min {
                            y_min = v;
                        }
                        if v > y_max {
                            y_max = v;
                        }
                    }
                }
            }
            if !y_min.is_finite() || !y_max.is_finite() {
                y_min = 0.0;
                y_max = 1.0;
            }
            if draw_zero_line {
                y_min = y_min.min(0.0);
                y_max = y_max.max(0.0);
            }
            if (y_max - y_min).abs() < 1e-4 {
                y_min -= 1.0;
                y_max += 1.0;
            } else {
                let pad = (y_max - y_min) * 0.08;
                y_min -= pad;
                y_max += pad;
            }

            let plot_rect = rect.shrink2(egui::vec2(44.0, 10.0));
            if plot_rect.width() <= 10.0 || plot_rect.height() <= 10.0 {
                return;
            }

            // Horizontal grid lines & Y-axis labels
            for step in 0..=4 {
                let frac = step as f32 / 4.0;
                let y = plot_rect.top() + frac * plot_rect.height();
                let val = y_max - frac * (y_max - y_min);
                painter.line_segment(
                    [egui::pos2(plot_rect.left(), y), egui::pos2(plot_rect.right(), y)],
                    egui::Stroke::new(0.8, with_alpha(divider, 140)),
                );
                painter.text(
                    egui::pos2(plot_rect.left() - 5.0, y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.1}", val),
                    egui::FontId::monospace(9.5),
                    muted,
                );
            }

            // Vertical distance grid lines (25%, 50%, 75%)
            for step in 1..4 {
                let frac = step as f32 * 0.25;
                let x = plot_rect.left() + frac * plot_rect.width();
                painter.line_segment(
                    [egui::pos2(x, plot_rect.top()), egui::pos2(x, plot_rect.bottom())],
                    egui::Stroke::new(0.8, with_alpha(divider, 110)),
                );
            }

            // Optional Zero Reference Line
            if draw_zero_line && y_min <= 0.0 && y_max >= 0.0 {
                let zero_frac = (y_max - 0.0) / (y_max - y_min);
                let y0 = plot_rect.top() + zero_frac * plot_rect.height();
                painter.line_segment(
                    [egui::pos2(plot_rect.left(), y0), egui::pos2(plot_rect.right(), y0)],
                    egui::Stroke::new(1.2, with_alpha(white, 90)),
                );
            }

            // Draw each trace polyline
            for s in series {
                let n = s.data.len();
                if n < 2 {
                    continue;
                }
                let mut pts = Vec::with_capacity(n);
                for (i, &val) in s.data.iter().enumerate() {
                    let x_frac = i as f32 / ((n - 1) as f32);
                    let y_frac = ((y_max - val) / (y_max - y_min)).clamp(0.0, 1.0);
                    let px = plot_rect.left() + x_frac * plot_rect.width();
                    let py = plot_rect.top() + y_frac * plot_rect.height();
                    pts.push(egui::pos2(px, py));
                }
                painter.add(egui::Shape::line(
                    pts,
                    egui::Stroke::new(s.stroke_width, s.color),
                ));
            }

            // Update shared hover cursor
            if let Some(mouse_pos) = response.hover_pos() {
                if plot_rect.contains(mouse_pos) {
                    let pct = ((mouse_pos.x - plot_rect.left()) / plot_rect.width()).clamp(0.0, 1.0);
                    *shared_hover_pct = Some(pct);
                }
            }

            // Draw synchronized vertical cursor line across all lanes
            if let Some(pct) = *shared_hover_pct {
                let cx = plot_rect.left() + pct.clamp(0.0, 1.0) * plot_rect.width();
                painter.line_segment(
                    [egui::pos2(cx, plot_rect.top()), egui::pos2(cx, plot_rect.bottom())],
                    egui::Stroke::new(1.2, settings.primary_accent()),
                );

                for s in series {
                    if !s.data.is_empty() {
                        let idx = (pct * ((s.data.len() - 1) as f32)).round() as usize;
                        if let Some(&v) = s.data.get(idx.min(s.data.len() - 1)) {
                            let y_frac = ((y_max - v) / (y_max - y_min)).clamp(0.0, 1.0);
                            let py = plot_rect.top() + y_frac * plot_rect.height();
                            painter.circle_filled(egui::pos2(cx, py), 3.5, s.color);
                        }
                    }
                }
            }
        });
}

fn draw_shock_corner_histogram_card(
    ui: &mut egui::Ui,
    settings: &AppSettings,
    title: &str,
    corner: &ShockCornerData,
) {
    let panel_bg = settings.panel_color();
    let elevated = settings.card_elevated_color();
    let divider = settings.divider_color();
    let white = settings.text_color();
    let muted = settings.text_muted();
    let primary = settings.primary_accent();

    egui::Frame::none()
        .fill(panel_bg)
        .stroke(egui::Stroke::new(1.0, divider))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).font(display_font(15.0)).color(white));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("Σ = {:.1}%", corner.total_binned_pct))
                            .color(primary)
                            .monospace()
                            .size(11.5)
                            .strong(),
                    );
                });
            });

            ui.add_space(4.0);

            // Regime breakdown pills
            ui.columns(4, |cols| {
                cols[0].label(
                    egui::RichText::new(format!(
                        "Lo Bump\n{:.1}%",
                        corner.low_speed_compression_pct
                    ))
                    .color(egui::Color32::from_rgb(34, 211, 238))
                    .size(11.0)
                    .strong(),
                );
                cols[1].label(
                    egui::RichText::new(format!(
                        "Lo Reb\n{:.1}%",
                        corner.low_speed_rebound_pct
                    ))
                    .color(egui::Color32::from_rgb(52, 211, 153))
                    .size(11.0)
                    .strong(),
                );
                cols[2].label(
                    egui::RichText::new(format!(
                        "Hi Bump\n{:.1}%",
                        corner.high_speed_compression_pct
                    ))
                    .color(egui::Color32::from_rgb(245, 158, 11))
                    .size(11.0)
                    .strong(),
                );
                cols[3].label(
                    egui::RichText::new(format!(
                        "Hi Reb\n{:.1}%",
                        corner.high_speed_rebound_pct
                    ))
                    .color(egui::Color32::from_rgb(168, 85, 247))
                    .size(11.0)
                    .strong(),
                );
            });

            ui.add_space(6.0);

            // Bar chart canvas
            let avail_w = ui.available_width().max(180.0);
            let chart_h = 165.0;
            let (rect, response) =
                ui.allocate_exact_size(egui::vec2(avail_w, chart_h), egui::Sense::hover());
            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, 6.0, elevated);
            painter.rect_stroke(rect, 6.0, egui::Stroke::new(1.0, divider));

            let plot_rect = rect.shrink2(egui::vec2(10.0, 18.0));
            let max_bin_pct = corner
                .bins
                .iter()
                .map(|b| b.time_pct)
                .fold(10.0_f32, f32::max)
                * 1.12;

            let n_bins = corner.bins.len().max(1);
            let bar_step = plot_rect.width() / (n_bins as f32);
            let bar_w = (bar_step - 1.5).max(2.0);

            let mut hovered_bin_info: Option<String> = None;
            let mouse_pos = response.hover_pos();

            for (i, bin) in corner.bins.iter().enumerate() {
                let x_left = plot_rect.left() + (i as f32) * bar_step;
                let h_frac = (bin.time_pct / max_bin_pct).clamp(0.0, 1.0);
                let bar_h = h_frac * plot_rect.height();
                let bar_rect = egui::Rect::from_min_max(
                    egui::pos2(x_left, plot_rect.bottom() - bar_h),
                    egui::pos2(x_left + bar_w, plot_rect.bottom()),
                );

                let bar_col = match bin.regime {
                    DamperRegime::HighSpeedRebound => egui::Color32::from_rgb(168, 85, 247),
                    DamperRegime::LowSpeedRebound => egui::Color32::from_rgb(52, 211, 153),
                    DamperRegime::LowSpeedCompression => egui::Color32::from_rgb(34, 211, 238),
                    DamperRegime::HighSpeedCompression => egui::Color32::from_rgb(245, 158, 11),
                };

                painter.rect_filled(bar_rect, 2.0, bar_col);

                if let Some(mp) = mouse_pos {
                    let hit_rect = egui::Rect::from_min_max(
                        egui::pos2(x_left, plot_rect.top()),
                        egui::pos2(x_left + bar_step, plot_rect.bottom()),
                    );
                    if hit_rect.contains(mp) {
                        painter.rect_stroke(bar_rect, 2.0, egui::Stroke::new(1.2, white));
                        hovered_bin_info = Some(format!(
                            "Bin [{:+.0} .. {:+.0} mm/s]: {:.2}% ({} ticks)",
                            bin.min_mms, bin.max_mms, bin.time_pct, bin.count
                        ));
                    }
                }
            }

            // Center 0 mm/s vertical axis line & labels
            let x_zero = plot_rect.center().x;
            painter.line_segment(
                [
                    egui::pos2(x_zero, plot_rect.top()),
                    egui::pos2(x_zero, plot_rect.bottom()),
                ],
                egui::Stroke::new(1.2, with_alpha(white, 150)),
            );
            painter.text(
                egui::pos2(plot_rect.left(), rect.bottom() - 2.0),
                egui::Align2::LEFT_BOTTOM,
                "-150 mm/s (Rebound)",
                egui::FontId::monospace(9.5),
                muted,
            );
            painter.text(
                egui::pos2(x_zero, rect.bottom() - 2.0),
                egui::Align2::CENTER_BOTTOM,
                "0",
                egui::FontId::monospace(9.5),
                white,
            );
            painter.text(
                egui::pos2(plot_rect.right(), rect.bottom() - 2.0),
                egui::Align2::RIGHT_BOTTOM,
                "+150 mm/s (Bump)",
                egui::FontId::monospace(9.5),
                muted,
            );

            ui.add_space(4.0);
            if let Some(info) = hovered_bin_info {
                ui.label(egui::RichText::new(info).color(primary).monospace().size(11.0));
            } else {
                ui.label(
                    egui::RichText::new(format!(
                        "Avg Bump: {:.1} mm/s (Peak {:.0}) · Avg Reb: {:.1} mm/s (Peak {:.0})",
                        corner.avg_compression_mms,
                        corner.peak_compression_mms,
                        corner.avg_rebound_mms,
                        corner.peak_rebound_mms
                    ))
                    .color(muted)
                    .size(11.0),
                );
            }
        });
}

fn draw_tire_corner_analysis_card(
    ui: &mut egui::Ui,
    settings: &AppSettings,
    title: &str,
    corner: &TireCornerAnalysis,
    corner_b: Option<&TireCornerAnalysis>,
) {
    let panel_bg = settings.panel_color();
    let elevated = settings.card_elevated_color();
    let divider = settings.divider_color();
    let white = settings.text_color();
    let muted = settings.text_muted();
    let primary = settings.primary_accent();
    let green = settings.status_green();
    let amber = settings.status_amber();
    let use_metric = settings.use_metric;

    let fmt_temp = |f_val: f32| -> String {
        if use_metric {
            format!("{:.1}°C", (f_val - 32.0) / 1.8)
        } else {
            format!("{:.1}°F", f_val)
        }
    };
    let fmt_diff_temp = |f_diff: f32| -> String {
        if use_metric {
            format!("{:+.1}°C", f_diff / 1.8)
        } else {
            format!("{:+.1}°F", f_diff)
        }
    };
    let fmt_press = |psi: f32| -> String {
        if use_metric {
            format!("{:.1} kPa", psi / KPA_TO_PSI)
        } else {
            format!("{:.2} psi", psi)
        }
    };

    let avg_wear = (corner.wear_outside_pct + corner.wear_center_pct + corner.wear_inside_pct) / 3.0;
    let wear_col = if avg_wear >= 90.0 {
        green
    } else if avg_wear >= 80.0 {
        primary
    } else if avg_wear >= 70.0 {
        amber
    } else {
        settings.status_red()
    };

    egui::Frame::none()
        .fill(panel_bg)
        .stroke(egui::Stroke::new(1.0, divider))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(title).font(display_font(15.0)).color(white));
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("{:.0}% Tread", avg_wear))
                        .color(wear_col)
                        .size(11.5)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Hot: {} ({:+.2} psi build)",
                            fmt_press(corner.end_pressure_psi),
                            corner.pressure_build_psi
                        ))
                        .color(primary)
                        .size(12.0)
                        .strong(),
                    );
                });
            });

            ui.add_space(6.0);

            // 3-Band Temperature & Tread Wear Boxes (Displayed in Pit/Garage Order: O·M·I for Left, I·M·O for Right)
            let (
                col1_lbl,
                col1_stat,
                col1_wear,
                col2_lbl,
                col2_stat,
                col2_wear,
                col3_lbl,
                col3_stat,
                col3_wear,
            ) = if corner.is_left_side {
                (
                    "OUTSIDE (O)",
                    corner.outside_stats_f,
                    corner.wear_outside_pct,
                    "MIDDLE (M)",
                    corner.center_stats_f,
                    corner.wear_center_pct,
                    "INSIDE (I)",
                    corner.inside_stats_f,
                    corner.wear_inside_pct,
                )
            } else {
                (
                    "INSIDE (I)",
                    corner.inside_stats_f,
                    corner.wear_inside_pct,
                    "MIDDLE (M)",
                    corner.center_stats_f,
                    corner.wear_center_pct,
                    "OUTSIDE (O)",
                    corner.outside_stats_f,
                    corner.wear_outside_pct,
                )
            };

            ui.columns(3, |cols| {
                let items = [
                    (col1_lbl, col1_stat, col1_wear),
                    (col2_lbl, col2_stat, col2_wear),
                    (col3_lbl, col3_stat, col3_wear),
                ];
                for (col_ui, (lbl, stat, wear_pct)) in cols.iter_mut().zip(items.into_iter()) {
                    egui::Frame::none()
                        .fill(elevated)
                        .rounding(egui::Rounding::same(7.0))
                        .inner_margin(8.0)
                        .show(col_ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new(lbl).color(muted).size(10.0).strong());
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new(fmt_temp(stat.avg))
                                        .font(display_font(16.5))
                                        .color(temp_heat_color(stat.avg)),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "Tread {:.0}% · Peak {}",
                                        wear_pct,
                                        fmt_temp(stat.max)
                                    ))
                                    .color(muted)
                                    .size(10.5),
                                );
                            });
                        });
                }
            });

            ui.add_space(8.0);

            // Engineering Metrics Grid: Inside-to-Outside Spread, Center Bias, Cold/Start -> Hot Pressure
            egui::Grid::new(format!("tire_eng_grid_{}", corner.corner_name))
                .num_columns(2)
                .spacing([16.0, 5.0])
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("Inside-to-Outside Spread (ΔT = T_in - T_out):")
                            .color(muted)
                            .size(11.5),
                    );
                    let spread_col = if corner.spread_stats_f.avg >= 5.0 && corner.spread_stats_f.avg <= 25.0 {
                        green
                    } else {
                        amber
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} (Peak {})",
                            fmt_diff_temp(corner.spread_stats_f.avg),
                            fmt_diff_temp(corner.spread_stats_f.max)
                        ))
                        .color(spread_col)
                        .size(12.0)
                        .strong(),
                    );
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Center Wear Bias (T_center - Avg(In,Out)):")
                            .color(muted)
                            .size(11.5),
                    );
                    let bias_desc = if corner.center_bias_stats_f.avg > 4.0 {
                        "Crown Hot / Over-inflated"
                    } else if corner.center_bias_stats_f.avg < -4.0 {
                        "Shoulders Hot / Under-inflated"
                    } else {
                        "Balanced Footprint"
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {}",
                            fmt_diff_temp(corner.center_bias_stats_f.avg),
                            bias_desc
                        ))
                        .color(white)
                        .size(12.0),
                    );
                    ui.end_row();

                    ui.label(
                        egui::RichText::new("Pressure Progression (Cold/Start → Hot / Peak):")
                            .color(muted)
                            .size(11.5),
                    );
                    let base_p = if corner.cold_pressure_psi > 0.1 {
                        corner.cold_pressure_psi
                    } else {
                        corner.start_pressure_psi
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{} → {} (Peak {})",
                            fmt_press(base_p),
                            fmt_press(corner.end_pressure_psi),
                            fmt_press(corner.peak_pressure_psi)
                        ))
                        .color(primary)
                        .size(12.0),
                    );
                    ui.end_row();

                    if let Some(cb) = corner_b {
                        ui.label(
                            egui::RichText::new("Comparison vs Lap B (ΔAvg Temp / ΔHot PSI):")
                                .color(muted)
                                .size(11.5),
                        );
                        let dt_mid = cb.center_stats_f.avg - corner.center_stats_f.avg;
                        let dp_end = cb.end_pressure_psi - corner.end_pressure_psi;
                        ui.label(
                            egui::RichText::new(format!(
                                "Center {} · Pressure {:+.2} psi",
                                fmt_diff_temp(dt_mid),
                                dp_end
                            ))
                            .color(settings.secondary_accent())
                            .size(11.5)
                            .strong(),
                        );
                        ui.end_row();
                    }
                });
        });
}

fn temp_heat_color(temp_f: f32) -> egui::Color32 {
    if temp_f < 155.0 {
        egui::Color32::from_rgb(56, 189, 248) // Cool Blue
    } else if temp_f <= 215.0 {
        egui::Color32::from_rgb(74, 222, 128) // Optimal Green
    } else if temp_f <= 240.0 {
        egui::Color32::from_rgb(251, 191, 36) // Warm Amber
    } else {
        egui::Color32::from_rgb(248, 113, 113) // Overheating Red
    }
}

pub fn format_ibt_engineering_report_tsv(
    sess: &IbtSession,
    lap_a: &LapAnalysis,
    lap_b: Option<&LapAnalysis>,
    use_metric: bool,
) -> String {
    let mut out = String::new();
    let m = &sess.metadata;
    let air_c = crate::convert_unit(&m.air_temp, use_metric);
    let trk_c = crate::convert_unit(&m.track_temp, use_metric);
    let wnd_c = crate::convert_unit(&m.wind_speed, use_metric);
    let wdir_c = crate::format_wind_dir(&m.wind_dir);
    out.push_str("SEARCH FOR SPEED — OFFLINE .IBT TELEMETRY ENGINEERING REPORT\n");
    out.push_str(&format!(
        "File\t{}\nTrack\t{}\nCar\t#{} {}\nDriver\t{} ({} iR)\nSetup\t{}\nWeather\tAir {} / Track {} / Wind {} {}\n\n",
        m.file_name, m.track_display_name, m.car_number, m.car_name, m.driver_name, m.irating, m.setup_name, air_c, trk_c, wnd_c, wdir_c
    ));

    out.push_str(&format!(
        "PRIMARY LAP A\tLap {} ({:.3}s) [{}]\n",
        lap_a.descriptor.lap_number,
        lap_a.descriptor.duration_secs,
        lap_a.descriptor.status_badge()
    ));
    if let Some(lb) = lap_b {
        out.push_str(&format!(
            "COMPARISON LAP B\tLap {} ({:.3}s) [{}]\n",
            lb.descriptor.lap_number,
            lb.descriptor.duration_secs,
            lb.descriptor.status_badge()
        ));
    }

    out.push_str("\nSUSPENSION RIDE HEIGHTS & RAKE (MM)\n");
    out.push_str("Metric\tMin (mm)\tAvg (mm)\tMax (mm)\n");
    let s = &lap_a.suspension;
    for (name, st) in [
        ("LF Ride Height", s.lf_stats_mm),
        ("RF Ride Height", s.rf_stats_mm),
        ("LR Ride Height", s.lr_stats_mm),
        ("RR Ride Height", s.rr_stats_mm),
        ("Front RH Avg", s.front_rh_stats_mm),
        ("Rear RH Avg", s.rear_rh_stats_mm),
        ("Pitch Rake (Rear - Front)", s.rake_stats_mm),
        ("Side Rake (Left - Right)", s.side_rake_stats_mm),
    ] {
        out.push_str(&format!("{}\t{:.2}\t{:.2}\t{:.2}\n", name, st.min, st.avg, st.max));
    }

    out.push_str("\n4-CORNER SHOCK VELOCITY DISTRIBUTION (% TIME)\n");
    out.push_str("Corner\tLo-Spd Bump %\tLo-Spd Reb %\tHi-Spd Bump %\tHi-Spd Reb %\tTotal %\n");
    for c in [&lap_a.shocks.lf, &lap_a.shocks.rf, &lap_a.shocks.lr, &lap_a.shocks.rr] {
        out.push_str(&format!(
            "{}\t{:.2}%\t{:.2}%\t{:.2}%\t{:.2}%\t{:.2}%\n",
            c.corner_name,
            c.low_speed_compression_pct,
            c.low_speed_rebound_pct,
            c.high_speed_compression_pct,
            c.high_speed_rebound_pct,
            c.total_binned_pct
        ));
    }

    out.push_str("\n4-CORNER TIRE TEMPERATURES (°F) & PRESSURES (PSI)\n");
    out.push_str("Corner\tInside (°F)\tCenter (°F)\tOutside (°F)\tSpread (In-Out °F)\tCenter Bias (°F)\tStart PSI\tHot End PSI\n");
    for t in [&lap_a.tires.lf, &lap_a.tires.rf, &lap_a.tires.lr, &lap_a.tires.rr] {
        out.push_str(&format!(
            "{}\t{:.1}\t{:.1}\t{:.1}\t{:+.1}\t{:+.1}\t{:.2}\t{:.2}\n",
            t.corner_name,
            t.inside_stats_f.avg,
            t.center_stats_f.avg,
            t.outside_stats_f.avg,
            t.spread_stats_f.avg,
            t.center_bias_stats_f.avg,
            t.start_pressure_psi,
            t.end_pressure_psi
        ));
    }

    out
}
