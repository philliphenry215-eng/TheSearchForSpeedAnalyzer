use crate::ibt::{
    build_demo_ibt_session, check_ibt_comparison_compatibility, compare_laps, default_telemetry_dir,
    normalize_lap, open_directory_in_explorer, parse_ibt_file, scan_ibt_directory, DamperRegime,
    IbtFileEntry, IbtSession, LapAnalysis, LapComparisonReport, NormalizedLapTrace, ShockCornerData,
    TireCornerAnalysis, TireTempSource, KPA_TO_PSI, MM_TO_INCHES,
};
use crate::settings::{AppSettings, CompareLineStyle};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadTarget {
    Primary,
    Reference,
}

enum LoadWorkerState {
    Idle,
    Loading {
        target: LoadTarget,
        file_name: String,
        rx: Receiver<Result<IbtSession, String>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TireChartMode {
    AllCorners, // LF, RF, LR, RR Center Temperatures
    FrontAxle,  // LF vs RF 3-Zone (Outside, Mid, Inside)
    RearAxle,   // LR vs RR 3-Zone (Outside, Mid, Inside)
    Pressures,  // LF, RF, LR, RR Dynamic Pressures (PSI / kPa)
}

impl Default for TireChartMode {
    fn default() -> Self {
        Self::AllCorners
    }
}

pub struct IbtTelemetryLabState {
    pub directory_input: String,
    pub scanned_files: Vec<IbtFileEntry>,
    pub file_filter: String,
    pub selected_file_path: Option<PathBuf>,
    pub session: Option<IbtSession>,
    pub reference_file_path: Option<PathBuf>,
    pub reference_session: Option<IbtSession>,
    pub active_tab: IbtAnalysisTab,
    pub selected_lap_a_idx: usize,
    pub compare_enabled: bool,
    pub selected_lap_b_idx: usize,
    pub x_axis_mode: XAxisMode,
    pub hover_dist_pct: Option<f32>,
    pub channel_search_filter: String,
    pub status_error: Option<String>,
    pub tire_chart_mode: TireChartMode,
    pub include_subfolders: bool,
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
            reference_file_path: None,
            reference_session: None,
            active_tab: IbtAnalysisTab::InputsAndSpeed,
            selected_lap_a_idx: 0,
            compare_enabled: false,
            selected_lap_b_idx: 0,
            x_axis_mode: XAxisMode::LapDistPct,
            hover_dist_pct: None,
            channel_search_filter: String::new(),
            status_error: None,
            tire_chart_mode: TireChartMode::AllCorners,
            include_subfolders: true,
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
        self.scanned_files = scan_ibt_directory(&path, self.include_subfolders);
    }

    pub fn trigger_load_file(&mut self, path: PathBuf, target: LoadTarget, ctx: egui::Context) {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("telemetry.ibt")
            .to_string();

        match target {
            LoadTarget::Primary => {
                self.selected_file_path = Some(path.clone());
            }
            LoadTarget::Reference => {
                self.reference_file_path = Some(path.clone());
            }
        }
        self.status_error = None;

        let (tx, rx) = channel();
        self.worker = LoadWorkerState::Loading {
            target,
            file_name,
            rx,
        };

        thread::spawn(move || {
            let res = parse_ibt_file(&path);
            let _ = tx.send(res);
            ctx.request_repaint();
        });
    }

    pub fn trigger_load_demo(&mut self, target: LoadTarget, ctx: egui::Context) {
        match target {
            LoadTarget::Primary => {
                self.selected_file_path = None;
            }
            LoadTarget::Reference => {
                self.reference_file_path = None;
            }
        }
        self.status_error = None;

        let (tx, rx) = channel();
        self.worker = LoadWorkerState::Loading {
            target,
            file_name: "porsche963gtp_watkinsglen_demo.ibt".to_string(),
            rx,
        };

        thread::spawn(move || {
            let res = build_demo_ibt_session();
            let _ = tx.send(res);
            ctx.request_repaint();
        });
    }

    pub fn clear_reference_session(&mut self) {
        self.reference_session = None;
        self.reference_file_path = None;
        if let Some(ref sess) = self.session {
            self.selected_lap_b_idx = sess
                .fastest_lap_idx
                .unwrap_or(0)
                .min(sess.analyzed_laps.len().saturating_sub(1));
        }
    }

    pub fn swap_sessions(&mut self) {
        std::mem::swap(&mut self.session, &mut self.reference_session);
        std::mem::swap(&mut self.selected_file_path, &mut self.reference_file_path);
        std::mem::swap(&mut self.selected_lap_a_idx, &mut self.selected_lap_b_idx);
    }

    fn poll_worker(&mut self) {
        if let LoadWorkerState::Loading { target, ref rx, .. } = self.worker {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(sess) => {
                        match target {
                            LoadTarget::Primary => {
                                let pb_idx = sess.fastest_lap_idx.unwrap_or(0);
                                self.selected_lap_a_idx = pb_idx.min(sess.analyzed_laps.len().saturating_sub(1));

                                // If no reference session is loaded, default Lap B to second best lap of Session A
                                if self.reference_session.is_none() {
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
                                }

                                self.session = Some(sess);
                                self.status_error = None;
                            }
                            LoadTarget::Reference => {
                                let pb_idx = sess.fastest_lap_idx.unwrap_or(0);
                                self.selected_lap_b_idx = pb_idx.min(sess.analyzed_laps.len().saturating_sub(1));
                                self.compare_enabled = true;
                                self.reference_session = Some(sess);
                                self.status_error = None;
                            }
                        }
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
                            (IbtAnalysisTab::TiresAndPressures, "Tires & Pressures"),
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
                                                    egui::RichText::new("▶ Load Demo .IBT")
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
                                                    self.trigger_load_demo(LoadTarget::Primary, ctx.clone());
                                                }
                                            },
                                        );
                                    });

                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        let resp = ui.add(
                                            egui::TextEdit::singleline(&mut self.directory_input)
                                                .desired_width(175.0)
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
                                            .add(egui::Button::new("Reset").fill(elevated))
                                            .on_hover_text("Reset to default iRacing telemetry directory")
                                            .clicked()

                                        {
                                            self.directory_input =
                                                default_telemetry_dir().to_string_lossy().to_string();
                                            self.refresh_directory();
                                        }
                                        if ui
                                            .add(egui::Button::new("📂").fill(elevated))
                                            .on_hover_text("Open folder in Windows File Explorer")
                                            .clicked()

                                        {
                                            open_directory_in_explorer(std::path::Path::new(self.directory_input.trim()));
                                        }
                                    });

                                    ui.add_space(2.0);
                                    ui.horizontal(|ui| {
                                        if ui.checkbox(&mut self.include_subfolders, "Subfolders").on_hover_text("Recursively scan subfolders for .ibt files (e.g. car or event directories)").changed() {
                                            self.refresh_directory();
                                        }
                                        ui.label(egui::RichText::new(format!("({} files found)", self.scanned_files.len())).color(muted).size(10.5));
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
                                                || f.subfolder
                                                    .as_deref()
                                                    .map_or(false, |sub| sub.to_lowercase().contains(&filter_lower))
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
                                                        "No .ibt files found in this folder.\nClick '▶ Load Demo .IBT' above to test with a 60Hz multi-lap dataset, or enter a custom path.",
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
                                                    let is_sel_a = self
                                                        .selected_file_path
                                                        .as_ref()
                                                        .map_or(false, |p| p == &entry.path);
                                                    let is_sel_b = self
                                                        .reference_file_path
                                                        .as_ref()
                                                        .map_or(false, |p| p == &entry.path);

                                                    let row_bg = if is_sel_a && is_sel_b {
                                                        with_alpha(primary, 42)
                                                    } else if is_sel_a {
                                                        with_alpha(primary, 32)
                                                    } else if is_sel_b {
                                                        with_alpha(secondary, 32)
                                                    } else {
                                                        elevated
                                                    };
                                                    let stroke = if is_sel_a {
                                                        egui::Stroke::new(1.0, primary)
                                                    } else if is_sel_b {
                                                        egui::Stroke::new(1.0, secondary)
                                                    } else {
                                                        egui::Stroke::NONE
                                                    };

                                                    let mut click_load_b = false;

                                                    let resp = egui::Frame::none()
                                                        .fill(row_bg)
                                                        .stroke(stroke)
                                                        .rounding(egui::Rounding::same(6.0))
                                                        .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                                                        .show(ui, |ui| {
                                                            ui.horizontal(|ui| {
                                                                ui.vertical(|ui| {
                                                                    ui.horizontal(|ui| {
                                                                        if is_sel_a {
                                                                            egui::Frame::none()
                                                                                .fill(with_alpha(primary, 60))
                                                                                .rounding(egui::Rounding::same(3.0))
                                                                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                                                .show(ui, |ui| {
                                                                                    ui.label(
                                                                                        egui::RichText::new("A")
                                                                                            .color(primary)
                                                                                            .strong()
                                                                                            .size(10.0),
                                                                                    );
                                                                                });
                                                                        }
                                                                        if is_sel_b {
                                                                            egui::Frame::none()
                                                                                .fill(with_alpha(secondary, 60))
                                                                                .rounding(egui::Rounding::same(3.0))
                                                                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                                                .show(ui, |ui| {
                                                                                    ui.label(
                                                                                        egui::RichText::new("B")
                                                                                            .color(secondary)
                                                                                            .strong()
                                                                                            .size(10.0),
                                                                                    );
                                                                                });
                                                                        }
                                                                        ui.label(
                                                                            egui::RichText::new(&entry.file_name)
                                                                                .color(if is_sel_a {
                                                                                    primary
                                                                                } else if is_sel_b {
                                                                                    secondary
                                                                                } else {
                                                                                    white
                                                                                })
                                                                                .size(11.5)
                                                                                .strong(),
                                                                        );
                                                                    });
                                                                    ui.horizontal(|ui| {
                                                                        if let Some(ref sub) = entry.subfolder {
                                                                            ui.label(
                                                                                egui::RichText::new(format!("📁 {}", sub))
                                                                                    .color(with_alpha(primary, 210))
                                                                                    .size(10.0),
                                                                            );
                                                                            ui.label(egui::RichText::new("·").color(muted).size(10.0));
                                                                        }
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
                                                                });

                                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                                    let b_label = if is_sel_b { "Ref B ✓" } else { "Ref B" };
                                                                    let b_btn = egui::Button::new(
                                                                        egui::RichText::new(b_label)
                                                                            .size(10.0)
                                                                            .color(if is_sel_b { white } else { muted })
                                                                            .strong(),
                                                                    )
                                                                    .fill(if is_sel_b {
                                                                        with_alpha(secondary, 180)
                                                                    } else {
                                                                        with_alpha(elevated, 200)
                                                                    })
                                                                    .rounding(egui::Rounding::same(4.0));

                                                                    if ui
                                                                        .add(b_btn)
                                                                        .on_hover_text("Load this file as Reference Session (Lap B comparison)")
                                                                        .clicked()
                            
                                                                    {
                                                                        click_load_b = true;
                                                                    }
                                                                });
                                                            });
                                                        })
                                                        .response
                                                        .interact(egui::Sense::click());

                                                    if click_load_b {
                                                        self.trigger_load_file(entry.path.clone(), LoadTarget::Reference, ctx.clone());
                                                    } else if resp.clicked() {
                                                        self.trigger_load_file(entry.path.clone(), LoadTarget::Primary, ctx.clone());
                                                    }

                                                    resp.context_menu(|ui| {
                                                        if ui.button("Load as Session A (Primary)").clicked() {
                                                            self.trigger_load_file(entry.path.clone(), LoadTarget::Primary, ctx.clone());
                                                            ui.close_menu();
                                                        }
                                                        if ui.button("Load as Reference Session (B)").clicked() {
                                                            self.trigger_load_file(entry.path.clone(), LoadTarget::Reference, ctx.clone());
                                                            ui.close_menu();
                                                        }
                                                    });

                                                    ui.add_space(3.0);
                                                }
                                            });

                                        let mut do_clear_ref = false;
                                        let mut do_swap = false;

                                        if self.reference_session.is_some() {
                                            let ref_name = self
                                                .reference_file_path
                                                .as_ref()
                                                .and_then(|p| p.file_name().and_then(|n| n.to_str()))
                                                .unwrap_or("Demo Reference")
                                                .to_string();

                                            ui.add_space(4.0);
                                            egui::Frame::none()
                                                .fill(with_alpha(secondary, 28))
                                                .stroke(egui::Stroke::new(1.0, with_alpha(secondary, 140)))
                                                .rounding(egui::Rounding::same(6.0))
                                                .inner_margin(egui::Margin::symmetric(8.0, 5.0))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new("Ref B:")
                                                                .color(secondary)
                                                                .strong()
                                                                .size(11.0),
                                                        );
                                                        ui.add(
                                                            egui::Label::new(
                                                                egui::RichText::new(&ref_name)
                                                                    .color(white)
                                                                    .size(11.0),
                                                            )
                                                            .truncate(true),
                                                        );
                                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                            if ui
                                                                .button(egui::RichText::new("×").color(muted).size(12.0))
                                                                .on_hover_text("Unload Reference Session")
                                                                .clicked()
                    
                                                            {
                                                                do_clear_ref = true;
                                                            }
                                                            if ui
                                                                .button(egui::RichText::new("Swap").color(white).size(11.0))
                                                                .on_hover_text("Swap Session A and Reference Session B")
                                                                .clicked()
                    
                                                            {
                                                                do_swap = true;
                                                            }
                                                        });
                                                    });
                                                });
                                        }

                                        if do_clear_ref {
                                            self.clear_reference_session();
                                        }
                                        if do_swap {
                                            self.swap_sessions();
                                        }
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

                                        let is_cross_file = self.reference_session.is_some();
                                        if self.compare_enabled && (is_cross_file || sess.laps.len() >= 2) {
                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                let label = if is_cross_file { "Target (Ref B):" } else { "Target (Lap B):" };
                                                ui.label(
                                                    egui::RichText::new(label)
                                                        .color(secondary)
                                                        .size(11.5)
                                                        .strong(),
                                                );
                                                let cur_b_label = if let Some(ref ref_sess) = self.reference_session {
                                                    ref_sess
                                                        .laps
                                                        .get(self.selected_lap_b_idx)
                                                        .map(|l| {
                                                            format!(
                                                                "[Ref] L{} ({}) [{}]",
                                                                l.lap_number,
                                                                format_time(l.duration_secs),
                                                                l.status_badge()
                                                            )
                                                        })
                                                        .unwrap_or_else(|| "Select Ref Lap B".to_string())
                                                } else {
                                                    sess.laps
                                                        .get(self.selected_lap_b_idx)
                                                        .map(|l| {
                                                            format!(
                                                                "Lap {} ({}) [{}]",
                                                                l.lap_number,
                                                                format_time(l.duration_secs),
                                                                l.status_badge()
                                                            )
                                                        })
                                                        .unwrap_or_else(|| "Select Lap B".to_string())
                                                };

                                                egui::ComboBox::from_id_source("ibt_lap_b_combo")
                                                    .selected_text(cur_b_label)
                                                    .width(185.0)
                                                    .show_ui(ui, |ui| {
                                                        if let Some(ref ref_sess) = self.reference_session {
                                                            for l in &ref_sess.laps {
                                                                let text = format!(
                                                                    "[Ref] Lap {} · {} ({})",
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
                                                        } else {
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
                                            egui::RichText::new("▶ Load 3-Lap 60Hz Demo .IBT Telemetry Session")
                                                .color(egui::Color32::BLACK)
                                                .size(14.0)
                                                .strong(),
                                        )
                                        .fill(primary)
                                        .min_size(egui::vec2(340.0, 38.0));
                                        if ui.add(btn).clicked() {
                                            self.trigger_load_demo(LoadTarget::Primary, ctx.clone());
                                        }
                                        ui.add_space(40.0);
                                    });
                                });
                            return;
                        }
                    };

                    let ref_sess_opt = self.reference_session.clone();
                    let mut do_clear_ref = false;
                    let mut do_swap = false;

                    let lap_a_idx = self.selected_lap_a_idx.min(sess.analyzed_laps.len() - 1);
                    let lap_a = &sess.analyzed_laps[lap_a_idx];

                    let (lap_b, compatibility_warning) = if let Some(ref ref_sess) = ref_sess_opt {
                        let compat = check_ibt_comparison_compatibility(&sess, ref_sess);
                        if let Some(warn) = compat.warning_message() {
                            (None, Some(warn))
                        } else if self.compare_enabled && !ref_sess.analyzed_laps.is_empty() {
                            let b_idx = self.selected_lap_b_idx.min(ref_sess.analyzed_laps.len() - 1);
                            (Some(&ref_sess.analyzed_laps[b_idx]), None)
                        } else {
                            (None, None)
                        }
                    } else if self.compare_enabled && sess.analyzed_laps.len() >= 2 {
                        let b_idx = self.selected_lap_b_idx.min(sess.analyzed_laps.len() - 1);
                        (Some(&sess.analyzed_laps[b_idx]), None)
                    } else {
                        (None, None)
                    };

                    let norm_a = normalize_lap(lap_a, 300);
                    let comparison = lap_b.map(|lb| compare_laps(lap_a, lb, 300));

                    egui::ScrollArea::vertical()
                        .id_source("ibt_right_canvas_scroll")
                        .max_height(avail_h)
                        .show(ui, |ui| {
                            if let Some(ref warn) = compatibility_warning {
                                egui::Frame::none()
                                    .fill(with_alpha(amber, 24))
                                    .stroke(egui::Stroke::new(1.0, amber))
                                    .rounding(egui::Rounding::same(8.0))
                                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("⚠").size(16.0));
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new(warn).color(white).size(12.0).strong());
                                                ui.label(
                                                    egui::RichText::new("Telemetry comparison overlay disabled until compatible files are loaded. Primary Session A is still displayed below.")
                                                        .color(muted)
                                                        .size(11.0),
                                                );
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button(egui::RichText::new("× Unload Ref").color(white).size(11.5)).clicked() {
                                                    do_clear_ref = true;
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(8.0);
                            } else if let Some(ref ref_sess) = ref_sess_opt {
                                if let Some(lb) = lap_b {
                                    let ref_meta = &ref_sess.metadata;
                                    egui::Frame::none()
                                        .fill(with_alpha(secondary, 22))
                                        .stroke(egui::Stroke::new(1.0, with_alpha(secondary, 120)))
                                        .rounding(egui::Rounding::same(8.0))
                                        .inner_margin(egui::Margin::symmetric(12.0, 7.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new("⚖ CROSS-FILE COMPARISON ACTIVE:").color(secondary).strong().size(11.5));
                                                ui.label(egui::RichText::new(format!(
                                                    "Session A [Gold]: {} (L{} · {}) vs Ref B [Blue]: {} (L{} · {})",
                                                    sess.metadata.setup_name, lap_a.descriptor.lap_number, format_time(lap_a.descriptor.duration_secs),
                                                    ref_meta.setup_name, lb.descriptor.lap_number, format_time(lb.descriptor.duration_secs),
                                                )).color(white).size(11.5));
                                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                    if ui.button(egui::RichText::new("× Unload Ref").color(muted).size(11.0)).clicked() {
                                                        do_clear_ref = true;
                                                    }
                                                    if ui.button(egui::RichText::new("Swap A / B").color(white).size(11.0)).clicked() {
                                                        do_swap = true;
                                                    }
                                                });
                                            });
                                        });
                                    ui.add_space(8.0);
                                }
                            }
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
                                        &sess,
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
                                    self.render_tires_and_pressures_tab(
                                        ui,
                                        settings,
                                        &sess,
                                        lap_a,
                                        lap_b,
                                        &norm_a,
                                    );
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
        let colors = &settings.telemetry.trace_colors;
        let compare_style = settings.telemetry.compare_style;
        let is_dashed = compare_style == CompareLineStyle::DashedLine;
        let base_sw = settings.telemetry.trace_stroke_width;

        let primary = colors.lap_a();
        let secondary = colors.lap_b();
        let white = settings.text_color();
        let muted = settings.text_muted();
        let green = colors.thr_a();
        let red = colors.brk_a();
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
                stroke_width: base_sw + 0.2,
                is_dashed: false,
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
                stroke_width: base_sw + 0.2,
                is_dashed: false,
            }];
            let b_label = comparison
                .map(|c| format!("Lap {} [B] Speed", c.lap_b.lap_number))
                .unwrap_or_default();
            if let Some(sb) = spd_b {
                series.push(TraceSeries {
                    label: &b_label,
                    data: sb,
                    color: secondary,
                    stroke_width: if is_dashed { base_sw + 0.2 } else { base_sw },
                    is_dashed,
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
                    color: colors.thr_a(),
                    stroke_width: base_sw + 0.1,
                    is_dashed: false,
                });
            }
            if let Some(ref brk) = norm_a.brake_pct {
                series.push(TraceSeries {
                    label: "Brake [A]",
                    data: brk,
                    color: colors.brk_a(),
                    stroke_width: base_sw + 0.1,
                    is_dashed: false,
                });
            }
            if let Some(comp) = comparison {
                if let Some(ref thr_b) = comp.lap_b.throttle_pct {
                    let thr_b_col = match compare_style {
                        CompareLineStyle::ShadedAlpha => with_alpha(colors.thr_a(), 125),
                        _ => colors.thr_b(),
                    };
                    series.push(TraceSeries {
                        label: "Throttle [B]",
                        data: thr_b,
                        color: thr_b_col,
                        stroke_width: if is_dashed { base_sw + 0.1 } else { base_sw * 0.9 },
                        is_dashed,
                    });
                }
                if let Some(ref brk_b) = comp.lap_b.brake_pct {
                    let brk_b_col = match compare_style {
                        CompareLineStyle::ShadedAlpha => with_alpha(colors.brk_a(), 125),
                        _ => colors.brk_b(),
                    };
                    series.push(TraceSeries {
                        label: "Brake [B]",
                        data: brk_b,
                        color: brk_b_col,
                        stroke_width: if is_dashed { base_sw + 0.1 } else { base_sw * 0.9 },
                        is_dashed,
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
                color: colors.steer_a(),
                stroke_width: base_sw,
                is_dashed: false,
            }];
            if let Some(comp) = comparison {
                if let Some(ref steer_b) = comp.lap_b.steering_deg {
                    let steer_b_col = match compare_style {
                        CompareLineStyle::ShadedAlpha => with_alpha(colors.steer_a(), 125),
                        _ => colors.steer_b(),
                    };
                    series.push(TraceSeries {
                        label: "Steering Angle [B]",
                        data: steer_b,
                        color: steer_b_col,
                        stroke_width: if is_dashed { base_sw } else { base_sw * 0.9 },
                        is_dashed,
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
                stroke_width: base_sw,
                is_dashed: false,
            }];
            if let Some(comp) = comparison {
                if let Some(ref rpm_b) = comp.lap_b.rpm {
                    series.push(TraceSeries {
                        label: "RPM [B]",
                        data: rpm_b,
                        color: secondary,
                        stroke_width: if is_dashed { base_sw } else { base_sw * 0.9 },
                        is_dashed,
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
        sess: &IbtSession,
        lap_a: &LapAnalysis,
        _lap_b: Option<&LapAnalysis>,
        norm_a: &NormalizedLapTrace,
        comparison: Option<&LapComparisonReport>,
    ) {
        let colors = &settings.telemetry.trace_colors;
        let compare_style = settings.telemetry.compare_style;
        let is_dashed = compare_style == CompareLineStyle::DashedLine;
        let base_sw = settings.telemetry.trace_stroke_width;

        let primary = colors.lap_a();
        let secondary = colors.lap_b();
        let white = settings.text_color();
        let green = settings.status_green();
        let amber = settings.status_amber();
        let purple = settings.status_purple();
        let use_metric = settings.use_metric;

        let scale = if use_metric { 1.0 } else { MM_TO_INCHES };
        let unit = if use_metric { "mm" } else { "in" };
        let susp = &lap_a.suspension;

        if sess.metadata.is_nextgen_lr_offset_applied {
            egui::Frame::none()
                .fill(settings.card_elevated_color())
                .stroke(egui::Stroke::new(1.0, primary))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(12.0, 7.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🏷 NASCAR NextGen Calibration Active:")
                                .color(primary)
                                .size(11.5)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(
                                "-0.50\" (-12.7 mm) datum calibration applied to Left Rear (LR) ride height, dynamic rake, side rake, and rear split.",
                            )
                            .color(white)
                            .size(11.0),
                        );
                    });
                });
            ui.add_space(8.0);
        }

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

            let rake_card_title = if susp.cfsr_stats_mm.is_some() {
                "DYNAMIC PITCH RAKE (REAR - SPLITTER)"
            } else {
                "DYNAMIC PITCH RAKE (REAR - FRONT)"
            };
            draw_kpi_card(
                &mut cols[2],
                settings,
                rake_card_title,
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
                stroke_width: base_sw + 0.1,
                is_dashed: false,
            });
        }
        if let Some(ref v) = rf_a_s {
            front_rh_series.push(TraceSeries {
                label: "RF Ride Height [A]",
                data: v,
                color: primary,
                stroke_width: base_sw + 0.1,
                is_dashed: false,
            });
        }
        if let Some(ref v) = cfsr_a_s {
            front_rh_series.push(TraceSeries {
                label: "Center Splitter (CFSR) [A]",
                data: v,
                color: purple,
                stroke_width: base_sw,
                is_dashed: false,
            });
        }
        if let Some(ref v) = lf_b_s {
            let lf_b_col = match compare_style {
                CompareLineStyle::ShadedAlpha => with_alpha(egui::Color32::from_rgb(34, 211, 238), 120),
                _ => egui::Color32::from_rgb(59, 130, 246), // High-contrast Electric Blue
            };
            front_rh_series.push(TraceSeries {
                label: "LF [B]",
                data: v,
                color: lf_b_col,
                stroke_width: if is_dashed { base_sw + 0.1 } else { base_sw * 0.85 },
                is_dashed,
            });
        }
        if let Some(ref v) = rf_b_s {
            let rf_b_col = match compare_style {
                CompareLineStyle::ShadedAlpha => with_alpha(primary, 120),
                _ => egui::Color32::from_rgb(249, 115, 22), // High-contrast Vivid Orange
            };
            front_rh_series.push(TraceSeries {
                label: "RF [B]",
                data: v,
                color: rf_b_col,
                stroke_width: if is_dashed { base_sw + 0.1 } else { base_sw * 0.85 },
                is_dashed,
            });
        }
        if let Some(ref v) = cfsr_b_s {
            let cfsr_b_col = match compare_style {
                CompareLineStyle::ShadedAlpha => with_alpha(purple, 120),
                _ => egui::Color32::from_rgb(236, 72, 153), // High-contrast Fuchsia/Pink
            };
            front_rh_series.push(TraceSeries {
                label: "CFSR [B]",
                data: v,
                color: cfsr_b_col,
                stroke_width: if is_dashed { base_sw } else { base_sw * 0.85 },
                is_dashed,
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
                stroke_width: base_sw + 0.1,
                is_dashed: false,
            });
        }
        if let Some(ref v) = rr_a_s {
            rear_rh_series.push(TraceSeries {
                label: "RR Ride Height [A]",
                data: v,
                color: egui::Color32::from_rgb(255, 115, 145),
                stroke_width: base_sw + 0.1,
                is_dashed: false,
            });
        }
        if let Some(ref v) = lr_b_s {
            let lr_b_col = match compare_style {
                CompareLineStyle::ShadedAlpha => with_alpha(green, 120),
                _ => egui::Color32::from_rgb(16, 185, 129), // High-contrast Emerald Teal
            };
            rear_rh_series.push(TraceSeries {
                label: "LR [B]",
                data: v,
                color: lr_b_col,
                stroke_width: if is_dashed { base_sw + 0.1 } else { base_sw * 0.85 },
                is_dashed,
            });
        }
        if let Some(ref v) = rr_b_s {
            let rr_b_col = match compare_style {
                CompareLineStyle::ShadedAlpha => with_alpha(egui::Color32::from_rgb(255, 115, 145), 120),
                _ => egui::Color32::from_rgb(225, 29, 72), // High-contrast Crimson
            };
            rear_rh_series.push(TraceSeries {
                label: "RR [B]",
                data: v,
                color: rr_b_col,
                stroke_width: if is_dashed { base_sw + 0.1 } else { base_sw * 0.85 },
                is_dashed,
            });
        }

        if !rear_rh_series.is_empty() {
            let rear_title = if sess.metadata.is_nextgen_lr_offset_applied {
                format!("REAR RIDE HEIGHTS (LR*, RR) ({}) · *LR includes -0.50\" NextGen offset", unit.to_uppercase())
            } else {
                format!("REAR RIDE HEIGHTS (LR, RR) ({})", unit.to_uppercase())
            };
            draw_synchronized_trace_lane(
                ui,
                settings,
                &rear_title,
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

        // Lane 3: Dynamic Pitch / Longitudinal Rake = ((LR + RR)/2) - Center Splitter
        let rake_a_s = scale_vec(&norm_a.rake_mm);
        let rake_b_s = comparison.and_then(|c| scale_vec(&c.lap_b.rake_mm));
        if let Some(ref ra) = rake_a_s {
            let trace_label_a = if norm_a.cfsr_rh_mm.is_some() {
                "Longitudinal Rake [A] (Rear - Splitter)"
            } else {
                "Longitudinal Rake [A] (Rear - Front)"
            };
            let mut series = vec![TraceSeries {
                label: trace_label_a,
                data: ra,
                color: primary,
                stroke_width: base_sw + 0.2,
                is_dashed: false,
            }];
            if let Some(ref rb) = rake_b_s {
                let trace_label_b = if comparison.as_ref().and_then(|c| c.lap_b.cfsr_rh_mm.as_ref()).is_some() {
                    "Longitudinal Rake [B] (Rear - Splitter)"
                } else {
                    "Longitudinal Rake [B] (Rear - Front)"
                };
                let rake_b_col = match compare_style {
                    CompareLineStyle::ShadedAlpha => with_alpha(primary, 120),
                    _ => secondary,
                };
                series.push(TraceSeries {
                    label: trace_label_b,
                    data: rb,
                    color: rake_b_col,
                    stroke_width: if is_dashed { base_sw + 0.2 } else { base_sw * 0.85 },
                    is_dashed,
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
                stroke_width: base_sw + 0.2,
                is_dashed: false,
            }];
            if let Some(ref sb) = side_b_s {
                let side_b_col = match compare_style {
                    CompareLineStyle::ShadedAlpha => with_alpha(amber, 120),
                    _ => secondary,
                };
                side_series.push(TraceSeries {
                    label: "Side Rake [B]",
                    data: sb,
                    color: side_b_col,
                    stroke_width: if is_dashed { base_sw + 0.2 } else { base_sw * 0.85 },
                    is_dashed,
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
                stroke_width: base_sw,
                is_dashed: false,
            });
        }
        if let Some(ref rs) = rsplit_s {
            split_series.push(TraceSeries {
                label: "Rear Split [A] (LR - RR)",
                data: rs,
                color: green,
                stroke_width: base_sw,
                is_dashed: false,
            });
        }

        if !split_series.is_empty() {
            let split_title = if sess.metadata.is_nextgen_lr_offset_applied {
                format!(
                    "LATERAL CORNER SPLITS · FRONT (LF - RF) & REAR (LR* - RR) ({}) · *LR calibrated",
                    unit.to_uppercase()
                )
            } else {
                format!(
                    "LATERAL CORNER SPLITS · FRONT (LF - RF) & REAR (LR - RR) ({})",
                    unit.to_uppercase()
                )
            };
            draw_synchronized_trace_lane(
                ui,
                settings,
                &split_title,
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
        sess: &IbtSession,
        lap_a: &LapAnalysis,
        lap_b: Option<&LapAnalysis>,
        norm_a: &NormalizedLapTrace,
    ) {
        let primary = settings.primary_accent();
        let white = settings.text_color();
        let muted = settings.text_muted();
        let amber = settings.status_amber();
        let green = settings.status_green();
        let panel_bg = settings.panel_color();
        let elevated = settings.card_elevated_color();
        let divider = settings.divider_color();
        let use_metric = settings.use_metric;

        let fmt_temp = |f_val: f32| -> String {
            if use_metric {
                format!("{:.1}°C", (f_val - 32.0) / 1.8)
            } else {
                format!("{:.1}°F", f_val)
            }
        };

        // 1. Header Frame: Title, Source, Chart Mode Selector
        egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, divider))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "4-CORNER TIRE THERMAL ANALYSIS & PRESSURES — LAP {} ({})",
                            lap_a.descriptor.lap_number,
                            lap_a.descriptor.status_badge()
                        ))
                        .color(primary)
                        .size(12.5)
                        .strong(),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("Source: {}", lap_a.tires.lf.temp_source.label()))
                            .color(muted)
                            .size(11.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mode_btn = |ui: &mut egui::Ui, mode: TireChartMode, label: &str, current: TireChartMode| -> bool {
                            let is_active = mode == current;
                            let btn = egui::Button::new(
                                egui::RichText::new(label)
                                    .color(if is_active { egui::Color32::BLACK } else { white })
                                    .size(11.0)
                                    .strong(),
                            )
                            .fill(if is_active { primary } else { elevated });
                            ui.add(btn).clicked()
                        };

                        if mode_btn(ui, TireChartMode::Pressures, "📊 Pressures (PSI)", self.tire_chart_mode) {
                            self.tire_chart_mode = TireChartMode::Pressures;
                        }
                        if mode_btn(ui, TireChartMode::RearAxle, "🛡 Rear Axle", self.tire_chart_mode) {
                            self.tire_chart_mode = TireChartMode::RearAxle;
                        }
                        if mode_btn(ui, TireChartMode::FrontAxle, "⚡ Front Axle", self.tire_chart_mode) {
                            self.tire_chart_mode = TireChartMode::FrontAxle;
                        }
                        if mode_btn(ui, TireChartMode::AllCorners, "4-Corners", self.tire_chart_mode) {
                            self.tire_chart_mode = TireChartMode::AllCorners;
                        }
                    });
                });

                let lf_static = (lap_a.tires.lf.center_stats_f.max - lap_a.tires.lf.center_stats_f.min).abs() < 0.08
                    && lap_a.tires.lf.wear_center_pct >= 99.9
                    && lap_a.tires.lf.temp_source == TireTempSource::Carcass;
                if lf_static {
                    ui.add_space(3.0);
                    ui.label(
                        egui::RichText::new(
                            "ℹ iRacing Telemetry Note: On-track carcass channels (*tempCL/CM/CR) remain at starting values while driving; definitive hot readings are latched at pit stops and garage exit.",
                        )
                        .color(amber)
                        .size(10.5),
                    );
                }
            });

        ui.add_space(8.0);

        // 2. Synchronized Color Telemetry Trace Lane Across Track Distance (0% - 100%)
        let (chart_title, chart_unit, series_list): (&str, &str, Vec<TraceSeries>) = match self.tire_chart_mode {
            TireChartMode::AllCorners => {
                let mut list = Vec::new();
                if let Some(ref v) = norm_a.lf_temp_f {
                    list.push(TraceSeries::solid("LF Center", v, egui::Color32::from_rgb(56, 189, 248), 1.8));
                }
                if let Some(ref v) = norm_a.rf_temp_f {
                    list.push(TraceSeries::solid("RF Center", v, egui::Color32::from_rgb(251, 191, 36), 2.2));
                }
                if let Some(ref v) = norm_a.lr_temp_f {
                    list.push(TraceSeries::solid("LR Center", v, egui::Color32::from_rgb(74, 222, 128), 1.8));
                }
                if let Some(ref v) = norm_a.rr_temp_f {
                    list.push(TraceSeries::solid("RR Center", v, egui::Color32::from_rgb(248, 113, 113), 2.0));
                }
                ("TIRE SURFACE / CARCASS TEMPERATURES ACROSS LAP (LF, RF, LR, RR)", if use_metric { "°C" } else { "°F" }, list)
            }
            TireChartMode::FrontAxle => {
                let mut list = Vec::new();
                if let Some(ref v) = norm_a.lf_outside_temp_f {
                    list.push(TraceSeries::solid("LF Out", v, egui::Color32::from_rgb(96, 165, 250), 1.4));
                }
                if let Some(ref v) = norm_a.lf_temp_f {
                    list.push(TraceSeries::solid("LF Mid", v, egui::Color32::from_rgb(56, 189, 248), 2.0));
                }
                if let Some(ref v) = norm_a.lf_inside_temp_f {
                    list.push(TraceSeries::solid("LF In", v, egui::Color32::from_rgb(45, 212, 191), 1.4));
                }
                if let Some(ref v) = norm_a.rf_inside_temp_f {
                    list.push(TraceSeries::solid("RF In", v, egui::Color32::from_rgb(251, 146, 60), 1.4));
                }
                if let Some(ref v) = norm_a.rf_temp_f {
                    list.push(TraceSeries::solid("RF Mid", v, egui::Color32::from_rgb(251, 191, 36), 2.2));
                }
                if let Some(ref v) = norm_a.rf_outside_temp_f {
                    list.push(TraceSeries::solid("RF Out", v, egui::Color32::from_rgb(234, 179, 8), 1.4));
                }
                ("FRONT AXLE TIRE TEMPERATURE DISTRIBUTION (LF vs. RF — 3-ZONE)", if use_metric { "°C" } else { "°F" }, list)
            }
            TireChartMode::RearAxle => {
                let mut list = Vec::new();
                if let Some(ref v) = norm_a.lr_outside_temp_f {
                    list.push(TraceSeries::solid("LR Out", v, egui::Color32::from_rgb(52, 211, 153), 1.4));
                }
                if let Some(ref v) = norm_a.lr_temp_f {
                    list.push(TraceSeries::solid("LR Mid", v, egui::Color32::from_rgb(74, 222, 128), 2.0));
                }
                if let Some(ref v) = norm_a.lr_inside_temp_f {
                    list.push(TraceSeries::solid("LR In", v, egui::Color32::from_rgb(167, 243, 208), 1.4));
                }
                if let Some(ref v) = norm_a.rr_inside_temp_f {
                    list.push(TraceSeries::solid("RR In", v, egui::Color32::from_rgb(244, 63, 94), 1.4));
                }
                if let Some(ref v) = norm_a.rr_temp_f {
                    list.push(TraceSeries::solid("RR Mid", v, egui::Color32::from_rgb(248, 113, 113), 2.2));
                }
                if let Some(ref v) = norm_a.rr_outside_temp_f {
                    list.push(TraceSeries::solid("RR Out", v, egui::Color32::from_rgb(251, 113, 133), 1.4));
                }
                ("REAR AXLE TIRE TEMPERATURE DISTRIBUTION (LR vs. RR — 3-ZONE)", if use_metric { "°C" } else { "°F" }, list)
            }
            TireChartMode::Pressures => {
                let mut list = Vec::new();
                if let Some(ref v) = norm_a.lf_press_psi {
                    list.push(TraceSeries::solid("LF Pressure", v, egui::Color32::from_rgb(56, 189, 248), 1.8));
                }
                if let Some(ref v) = norm_a.rf_press_psi {
                    list.push(TraceSeries::solid("RF Pressure", v, egui::Color32::from_rgb(251, 191, 36), 2.2));
                }
                if let Some(ref v) = norm_a.lr_press_psi {
                    list.push(TraceSeries::solid("LR Pressure", v, egui::Color32::from_rgb(74, 222, 128), 1.8));
                }
                if let Some(ref v) = norm_a.rr_press_psi {
                    list.push(TraceSeries::solid("RR Pressure", v, egui::Color32::from_rgb(248, 113, 113), 2.0));
                }
                ("DYNAMIC TIRE PRESSURES ACROSS LAP (LF, RF, LR, RR)", if use_metric { "kPa" } else { "psi" }, list)
            }
        };

        if !series_list.is_empty() {
            draw_synchronized_trace_lane(
                ui,
                settings,
                chart_title,
                chart_unit,
                &norm_a.grid_dist_pct,
                &norm_a.grid_dist_m,
                self.x_axis_mode,
                &series_list,
                160.0,
                false,
                &mut self.hover_dist_pct,
            );

            // Track Sector Reference Timeline Bar
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(panel_bg)
                .rounding(egui::Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Track Sectors:").color(muted).size(10.5));
                        draw_legend_pill(ui, egui::Color32::from_rgb(71, 85, 105), "🏁 Frontstretch (0-15%)", white);
                        draw_legend_pill(ui, egui::Color32::from_rgb(30, 64, 175), "↩ Turn 1 / 2 (15-35%)", white);
                        draw_legend_pill(ui, egui::Color32::from_rgb(71, 85, 105), "➡ Backstretch (35-65%)", white);
                        draw_legend_pill(ui, egui::Color32::from_rgb(180, 83, 9), "↩ Turn 3 / 4 (65-85%)", white);
                        draw_legend_pill(ui, egui::Color32::from_rgb(71, 85, 105), "🏁 Tri-Oval (85-100%)", white);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let rf = &lap_a.tires.rf;
                            ui.label(
                                egui::RichText::new(format!(
                                    "🔥 Hottest Tire: RF {} · 📍 {}",
                                    fmt_temp(rf.peak_temp_f),
                                    rf.peak_track_loc
                                ))
                                .color(temp_heat_color(rf.peak_temp_f))
                                .size(11.0)
                                .strong(),
                            );
                        });
                    });
                });
            ui.add_space(8.0);
        }

        // 3. Visual 4-Wheel Top-Down Chassis Heatmap Cards
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("4-CORNER CHASSIS HEATMAP & TREAD WEAR")
                    .color(primary)
                    .size(12.0)
                    .strong(),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("▲ FRONT OF CAR (DIRECTION OF TRAVEL) ▲")
                    .color(muted)
                    .size(11.0),
            );
        });
        ui.add_space(4.0);

        // Front Row: LF & RF
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

        ui.add_space(6.0);

        // Rear Row: LR & RR
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

        ui.add_space(10.0);

        // 4. Stint / Run Progression Table: Peak Tire Temps & Pressures Per Lap Across the Entire Session
        egui::Frame::none()
            .fill(panel_bg)
            .stroke(egui::Stroke::new(1.0, divider))
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "🏁 RUN PROGRESSION — PEAK TIRE TEMPERATURES & PRESSURES ({} LAPS)",
                            sess.analyzed_laps.len()
                        ))
                        .color(primary)
                        .size(12.0)
                        .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Click any row to inspect that lap's telemetry and tire footprint")
                            .color(muted)
                            .size(11.0),
                    );
                });

                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .id_source("ibt_tire_run_progression_scroll")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        egui::Grid::new("ibt_tire_progression_grid")
                            .num_columns(9)
                            .spacing([14.0, 5.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Lap #").color(muted).strong().size(11.0));
                                ui.label(egui::RichText::new("Time").color(muted).strong().size(11.0));
                                ui.label(egui::RichText::new("Status").color(muted).strong().size(11.0));
                                ui.label(egui::RichText::new("LF Peak").color(muted).strong().size(11.0));
                                ui.label(egui::RichText::new("RF Peak").color(primary).strong().size(11.0));
                                ui.label(egui::RichText::new("LR Peak").color(muted).strong().size(11.0));
                                ui.label(egui::RichText::new("RR Peak").color(muted).strong().size(11.0));
                                ui.label(egui::RichText::new("Hottest Tire & Location").color(muted).strong().size(11.0));
                                ui.label(egui::RichText::new("Hot Press (RF)").color(muted).strong().size(11.0));
                                ui.end_row();

                                for (idx, lap) in sess.analyzed_laps.iter().enumerate() {
                                    let is_selected = idx == self.selected_lap_a_idx;
                                    let row_col = if is_selected { primary } else { white };

                                    let lap_label = format!("Lap {}", lap.descriptor.lap_number);
                                    let lap_btn = ui.selectable_label(is_selected, egui::RichText::new(lap_label).color(row_col).strong());
                                    if lap_btn.clicked() {
                                        self.selected_lap_a_idx = idx;
                                    }

                                    ui.label(egui::RichText::new(format_time(lap.descriptor.duration_secs)).color(row_col));
                                    let badge = lap.descriptor.status_badge();
                                    let badge_col = if badge.contains("PB") {
                                        primary
                                    } else if badge == "VALID" {
                                        green
                                    } else {
                                        amber
                                    };
                                    ui.label(egui::RichText::new(badge).color(badge_col).strong());

                                    let t = &lap.tires;
                                    ui.label(egui::RichText::new(fmt_temp(t.lf.peak_temp_f)).color(temp_heat_color(t.lf.peak_temp_f)));
                                    ui.label(egui::RichText::new(fmt_temp(t.rf.peak_temp_f)).color(temp_heat_color(t.rf.peak_temp_f)).strong());
                                    ui.label(egui::RichText::new(fmt_temp(t.lr.peak_temp_f)).color(temp_heat_color(t.lr.peak_temp_f)));
                                    ui.label(egui::RichText::new(fmt_temp(t.rr.peak_temp_f)).color(temp_heat_color(t.rr.peak_temp_f)));

                                    let corners = [
                                        ("LF", t.lf.peak_temp_f, &t.lf.peak_track_loc),
                                        ("RF", t.rf.peak_temp_f, &t.rf.peak_track_loc),
                                        ("LR", t.lr.peak_temp_f, &t.lr.peak_track_loc),
                                        ("RR", t.rr.peak_temp_f, &t.rr.peak_track_loc),
                                    ];
                                    let hottest = corners.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
                                    ui.label(
                                        egui::RichText::new(format!("{} ({} · {})", hottest.0, fmt_temp(hottest.1), hottest.2))
                                            .color(temp_heat_color(hottest.1)),
                                    );

                                    let rf_press_str = if use_metric {
                                        format!("{:.1} kPa", t.rf.end_pressure_psi / KPA_TO_PSI)
                                    } else {
                                        format!("{:.1} psi", t.rf.end_pressure_psi)
                                    };
                                    ui.label(egui::RichText::new(rf_press_str).color(primary));
                                    ui.end_row();
                                }
                            });
                    });
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
    pub is_dashed: bool,
}

impl<'a> TraceSeries<'a> {
    pub fn solid(label: &'a str, data: &'a [f32], color: egui::Color32, stroke_width: f32) -> Self {
        Self {
            label,
            data,
            color,
            stroke_width,
            is_dashed: false,
        }
    }

    pub fn dashed(label: &'a str, data: &'a [f32], color: egui::Color32, stroke_width: f32) -> Self {
        Self {
            label,
            data,
            color,
            stroke_width,
            is_dashed: true,
        }
    }
}

fn draw_dashed_polyline(
    painter: &egui::Painter,
    pts: &[egui::Pos2],
    stroke: egui::Stroke,
    dash_len: f32,
    gap_len: f32,
) {
    if pts.len() < 2 {
        return;
    }
    let period = dash_len + gap_len;
    let mut dist = 0.0;
    for w in pts.windows(2) {
        let p1 = w[0];
        let p2 = w[1];
        let seg_len = (p2 - p1).length();
        if seg_len < 1e-4 {
            continue;
        }

        let mut t = 0.0;
        while t < seg_len {
            let cur_dist = dist + t;
            let phase = cur_dist % period;
            let remaining_in_phase = if phase < dash_len {
                dash_len - phase
            } else {
                period - phase
            };
            let step = (seg_len - t).min(remaining_in_phase);
            if phase < dash_len {
                let start_frac = t / seg_len;
                let end_frac = (t + step) / seg_len;
                let sp = p1 + (p2 - p1) * start_frac;
                let ep = p1 + (p2 - p1) * end_frac;
                painter.line_segment([sp, ep], stroke);
            }
            t += step;
        }
        dist += seg_len;
    }
}

fn downsample_min_max(data: &[f32], target_count: usize) -> (Vec<f32>, Vec<usize>) {
    let n = data.len();
    if n <= target_count || target_count < 4 {
        let indices = (0..n).collect();
        return (data.to_vec(), indices);
    }
    let bucket_size = n as f32 / (target_count / 2) as f32;
    let mut out_vals = Vec::with_capacity(target_count);
    let mut out_indices = Vec::with_capacity(target_count);

    let mut b_start = 0.0;
    while (b_start as usize) < n {
        let start_idx = b_start as usize;
        let end_idx = ((b_start + bucket_size) as usize).min(n);
        if start_idx >= end_idx {
            break;
        }

        let mut min_val = f32::INFINITY;
        let mut min_idx = start_idx;
        let mut max_val = f32::NEG_INFINITY;
        let mut max_idx = start_idx;

        for (offset, &v) in data[start_idx..end_idx].iter().enumerate() {
            let actual_idx = start_idx + offset;
            if v < min_val {
                min_val = v;
                min_idx = actual_idx;
            }
            if v > max_val {
                max_val = v;
                max_idx = actual_idx;
            }
        }

        if min_idx <= max_idx {
            out_vals.push(min_val);
            out_indices.push(min_idx);
            if max_idx != min_idx {
                out_vals.push(max_val);
                out_indices.push(max_idx);
            }
        } else {
            out_vals.push(max_val);
            out_indices.push(max_idx);
            out_vals.push(min_val);
            out_indices.push(min_idx);
        }

        b_start += bucket_size;
    }
    (out_vals, out_indices)
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
                    if let Some(idx) = hover_idx {
                        if series.len() == 2 {
                            let val_a = series[0].data.get(idx).copied();
                            let val_b = series[1].data.get(idx).copied();
                            if let (Some(va), Some(vb)) = (val_a, val_b) {
                                let delta = vb - va;
                                let delta_col = if delta.abs() < 1e-4 {
                                    muted
                                } else if delta > 0.0 {
                                    settings.telemetry.trace_colors.d_ahead()
                                } else {
                                    settings.telemetry.trace_colors.d_behind()
                                };
                                let delta_txt = format!("Δ (B - A): {:+.2} {}", delta, unit);
                                draw_legend_pill(ui, delta_col, &delta_txt, white);
                                ui.add_space(6.0);
                            }
                        }
                    }

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

            // Draw each trace polyline (with adaptive LOD downsampling & dashed support)
            for s in series {
                let n = s.data.len();
                if n < 2 {
                    continue;
                }

                let target_pts = (plot_rect.width() * 2.0).clamp(500.0, 2400.0) as usize;
                let (sampled_vals, sampled_indices) = if settings.telemetry.enable_downsampling_lod && n > target_pts * 2 {
                    downsample_min_max(s.data, target_pts)
                } else {
                    (s.data.to_vec(), (0..n).collect())
                };

                let m = sampled_vals.len();
                if m < 2 {
                    continue;
                }

                let mut pts = Vec::with_capacity(m);
                for (&idx, &val) in sampled_indices.iter().zip(sampled_vals.iter()) {
                    let x_frac = idx as f32 / ((n - 1) as f32);
                    let y_frac = ((y_max - val) / (y_max - y_min)).clamp(0.0, 1.0);
                    let px = plot_rect.left() + x_frac * plot_rect.width();
                    let py = plot_rect.top() + y_frac * plot_rect.height();
                    pts.push(egui::pos2(px, py));
                }

                if s.is_dashed {
                    draw_dashed_polyline(&painter, &pts, egui::Stroke::new(s.stroke_width, s.color), 7.0, 4.5);
                } else {
                    painter.add(egui::Shape::line(
                        pts,
                        egui::Stroke::new(s.stroke_width, s.color),
                    ));
                }
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

            ui.horizontal(|ui| {
                let peak_col = temp_heat_color(corner.peak_temp_f);
                ui.label(
                    egui::RichText::new(format!("🔥 Peak: {}", fmt_temp(corner.peak_temp_f)))
                        .color(peak_col)
                        .size(12.5)
                        .strong(),
                );
                if !corner.peak_track_loc.is_empty() {
                    ui.label(
                        egui::RichText::new(format!("📍 {}", corner.peak_track_loc))
                            .color(white)
                            .size(11.5),
                    );
                }
            });

            ui.add_space(5.0);

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
    out.push_str("THE SEARCH FOR SPEED — OFFLINE .IBT TELEMETRY ENGINEERING REPORT\n");
    out.push_str(&format!(
        "File\t{}\nTrack\t{}\nCar\t#{} {}\nDriver\t{} ({} iR)\nSetup\t{}\nWeather\tAir {} / Track {} / Wind {} {}\n",
        m.file_name, m.track_display_name, m.car_number, m.car_name, m.driver_name, m.irating, m.setup_name, air_c, trk_c, wnd_c, wdir_c
    ));
    if m.is_nextgen_lr_offset_applied {
        out.push_str("Calibration\tNASCAR NextGen LR -0.50\" (-12.7 mm) datum calibration applied\n");
    }
    out.push('\n');

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
    let rake_metric_name = if s.cfsr_stats_mm.is_some() {
        "Pitch Rake (Rear - Splitter)"
    } else {
        "Pitch Rake (Rear - Front)"
    };
    let mut metrics = vec![
        ("LF Ride Height", s.lf_stats_mm),
        ("RF Ride Height", s.rf_stats_mm),
        ("LR Ride Height", s.lr_stats_mm),
        ("RR Ride Height", s.rr_stats_mm),
    ];
    if let Some(cfsr) = s.cfsr_stats_mm {
        metrics.push(("Center Splitter (CFSR)", cfsr));
    }
    metrics.push(("Front RH Avg", s.front_rh_stats_mm));
    metrics.push(("Rear RH Avg", s.rear_rh_stats_mm));
    metrics.push((rake_metric_name, s.rake_stats_mm));
    metrics.push(("Side Rake (Left - Right)", s.side_rake_stats_mm));

    for (name, st) in metrics {
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
    out.push_str("Corner\tPeak Temp (°F)\tPeak Location\tInside (°F)\tCenter (°F)\tOutside (°F)\tSpread (In-Out °F)\tCenter Bias (°F)\tStart PSI\tHot End PSI\n");
    for t in [&lap_a.tires.lf, &lap_a.tires.rf, &lap_a.tires.lr, &lap_a.tires.rr] {
        out.push_str(&format!(
            "{}\t{:.1}\t{}\t{:.1}\t{:.1}\t{:.1}\t{:+.1}\t{:+.1}\t{:.2}\t{:.2}\n",
            t.corner_name,
            t.peak_temp_f,
            t.peak_track_loc,
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
