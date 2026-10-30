use eframe::egui;
use egui_extras::{Column, TableBuilder};
use crate::{with_alpha, WarRoomApp};
use crate::formatters::*;
use crate::models::*;
use crate::views::*;

impl WarRoomApp {
    pub(crate) fn render_grid_table_view(&mut self, ui: &mut egui::Ui, s: &WarRoomState) {
        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let panel_bg = self.settings.panel_color();
        let cols = self.settings.columns.clone();
        let active_buckets = self.settings.active_avg_buckets.clone();

        // 1. Gather all driver stint entries
        struct StintGridEntry<'a> {
            car_idx: usize,
            pos: i32,
            car_num: &'a str,
            driver_name: &'a str,
            irating: &'a str,
            is_player: bool,
            is_on_pit_road: bool,
            setup_name: &'a str,
            stint_num: i32,
            stint_short_label: String,
            stint_is_active: bool,
            stint_is_imported: bool,
            laps_completed: i32,
            last_lap: f32,
            fast_lap: f32,
            full_avg: f32,
            bucket_avgs: Vec<(usize, f32)>,
        }

        let mut entries: Vec<StintGridEntry> = Vec::new();

        for (&car_idx, drv) in &s.drivers {
            let is_player = car_idx == s.player_car_idx;
            if drv.stints.is_empty() {
                if drv.position > 0 || drv.is_in_world {
                    let entry_setup = if !drv.setup_name.is_empty() && drv.setup_name != "—" && drv.setup_name != "Baseline" {
                        &drv.setup_name
                    } else if is_player && !s.player_setup_name.is_empty() && s.player_setup_name != "Baseline" {
                        &s.player_setup_name
                    } else {
                        "Baseline"
                    };
                    entries.push(StintGridEntry {
                        car_idx,
                        pos: drv.position,
                        car_num: &drv.car_num,
                        driver_name: &drv.driver_name,
                        irating: &drv.irating,
                        is_player,
                        is_on_pit_road: drv.is_on_pit_road,
                        setup_name: entry_setup,
                        stint_num: 1,
                        stint_short_label: "S1".to_string(),
                        stint_is_active: false,
                        stint_is_imported: false,
                        laps_completed: 0,
                        last_lap: 0.0,
                        fast_lap: 0.0,
                        full_avg: 0.0,
                        bucket_avgs: active_buckets.iter().map(|&b| (b, 0.0)).collect(),
                    });
                }
            } else {
                for stint in &drv.stints {
                    let bucket_avgs = active_buckets.iter().map(|&b| (b, stint.avg_first_n(b))).collect();
                    let entry_setup = if !stint.setup_name.is_empty() && stint.setup_name != "—" && stint.setup_name != "Baseline" {
                        &stint.setup_name
                    } else if !drv.setup_name.is_empty() && drv.setup_name != "—" && drv.setup_name != "Baseline" {
                        &drv.setup_name
                    } else if is_player && !s.player_setup_name.is_empty() && s.player_setup_name != "Baseline" {
                        &s.player_setup_name
                    } else {
                        "Baseline"
                    };
                    entries.push(StintGridEntry {
                        car_idx,
                        pos: drv.position,
                        car_num: &drv.car_num,
                        driver_name: &drv.driver_name,
                        irating: &drv.irating,
                        is_player,
                        is_on_pit_road: drv.is_on_pit_road,
                        setup_name: entry_setup,
                        stint_num: stint.stint_number,
                        stint_short_label: stint.short_label(),
                        stint_is_active: stint.is_active,
                        stint_is_imported: stint.is_imported,
                        laps_completed: stint.laps_completed,
                        last_lap: stint.last_lap,
                        fast_lap: stint.fast_lap,
                        full_avg: stint.full_average(),
                        bucket_avgs,
                    });
                }
            }
        }

        // 2. Compute Session Benchmark Bests
        let mut session_fastest_lap = 0.0;
        let mut session_fastest_holder = String::new();
        let mut session_fastest_buckets: std::collections::HashMap<usize, f32> = std::collections::HashMap::new();

        for e in &entries {
            if e.fast_lap > 0.0 && (session_fastest_lap == 0.0 || e.fast_lap < session_fastest_lap) {
                session_fastest_lap = e.fast_lap;
                session_fastest_holder = format!("P{} #{} {}", if e.pos > 0 { e.pos.to_string() } else { "—".to_string() }, e.car_num, e.driver_name);
            }

            for &(b, b_val) in &e.bucket_avgs {
                if b_val > 0.0 {
                    let cur = session_fastest_buckets.entry(b).or_insert(b_val);
                    if b_val < *cur {
                        *cur = b_val;
                    }
                }
            }
        }

        // 3. Search Filter
        let search_query = self.driver_search_query.trim().to_lowercase();
        if !search_query.is_empty() {
            entries.retain(|e| {
                e.driver_name.to_lowercase().contains(&search_query) ||
                e.car_num.to_lowercase().contains(&search_query) ||
                e.setup_name.to_lowercase().contains(&search_query)
            });
        }

        // 4. Sorting
        let current_sort_key = self.grid_sort_key;
        let current_sort_dir = self.grid_sort_dir;

        entries.sort_by(|a, b| {
            let cmp = match current_sort_key {
                GridSortKey::Position => {
                    let a_p = if a.pos > 0 { a.pos } else { 9999 };
                    let b_p = if b.pos > 0 { b.pos } else { 9999 };
                    a_p.cmp(&b_p)
                }
                GridSortKey::CarNumber => {
                    let a_num = a.car_num.parse::<i32>().unwrap_or(9999);
                    let b_num = b.car_num.parse::<i32>().unwrap_or(9999);
                    a_num.cmp(&b_num).then_with(|| a.car_num.cmp(b.car_num))
                }
                GridSortKey::DriverName => a.driver_name.cmp(b.driver_name),
                GridSortKey::StintNumber => a.stint_num.cmp(&b.stint_num),
                GridSortKey::LapsCompleted => a.laps_completed.cmp(&b.laps_completed),
                GridSortKey::LastLap => {
                    let a_t = if a.last_lap > 0.0 { a.last_lap } else { f32::MAX };
                    let b_t = if b.last_lap > 0.0 { b.last_lap } else { f32::MAX };
                    a_t.partial_cmp(&b_t).unwrap_or(std::cmp::Ordering::Equal)
                }
                GridSortKey::FastestLap => {
                    let a_t = if a.fast_lap > 0.0 { a.fast_lap } else { f32::MAX };
                    let b_t = if b.fast_lap > 0.0 { b.fast_lap } else { f32::MAX };
                    a_t.partial_cmp(&b_t).unwrap_or(std::cmp::Ordering::Equal)
                }
                GridSortKey::CurrentAverage => {
                    let a_t = if a.full_avg > 0.0 { a.full_avg } else { f32::MAX };
                    let b_t = if b.full_avg > 0.0 { b.full_avg } else { f32::MAX };
                    a_t.partial_cmp(&b_t).unwrap_or(std::cmp::Ordering::Equal)
                }
                GridSortKey::AvgBucket(target_b) => {
                    let a_val = a.bucket_avgs.iter().find(|(b, _)| *b == target_b).map(|(_, v)| *v).unwrap_or(0.0);
                    let b_val = b.bucket_avgs.iter().find(|(b, _)| *b == target_b).map(|(_, v)| *v).unwrap_or(0.0);
                    let a_t = if a_val > 0.0 { a_val } else { f32::MAX };
                    let b_t = if b_val > 0.0 { b_val } else { f32::MAX };
                    a_t.partial_cmp(&b_t).unwrap_or(std::cmp::Ordering::Equal)
                }
            };

            if current_sort_dir == SortDirection::Descending {
                cmp.reverse()
            } else {
                cmp
            }
        });

        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let purple = self.settings.status_purple();
        let card_stroke = egui::Stroke::new(1.0, self.settings.divider_color());

        // 5. Render Top Session Intelligence & Search Bar
        egui::Frame::none()
            .fill(panel_bg)
            .stroke(card_stroke)
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Search Driver
                    ui.label(egui::RichText::new("SEARCH").color(muted).size(11.0).extra_letter_spacing(0.8));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.driver_search_query)
                            .hint_text("Driver or Car #...")
                            .desired_width(160.0)
                    );
                    if !self.driver_search_query.is_empty() {
                        if ui.add(egui::Button::new(egui::RichText::new("Clear").color(muted).size(11.5)).frame(false)).clicked() {
                            self.driver_search_query.clear();
                        }
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // Active Sort indicator
                    let sort_label = match current_sort_key {
                        GridSortKey::Position => "Position".to_string(),
                        GridSortKey::CarNumber => "Car #".to_string(),
                        GridSortKey::DriverName => "Driver".to_string(),
                        GridSortKey::StintNumber => "Stint #".to_string(),
                        GridSortKey::LapsCompleted => "Laps".to_string(),
                        GridSortKey::LastLap => "Last Lap".to_string(),
                        GridSortKey::FastestLap => "Fastest Lap".to_string(),
                        GridSortKey::CurrentAverage => "Full Avg".to_string(),
                        GridSortKey::AvgBucket(b) => format!("Avg {}", b),
                    };
                    let sort_dir_icon = if current_sort_dir == SortDirection::Ascending { "▲" } else { "▼" };

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("SORT").color(muted).size(11.0).extra_letter_spacing(0.8));
                        ui.label(egui::RichText::new(format!("{} {}", sort_label, sort_dir_icon)).color(white).strong().size(12.5));
                        if current_sort_key != GridSortKey::Position {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("Reset").color(muted).size(11.0))
                                    .fill(elevated)
                                    .rounding(egui::Rounding::same(6.0))
                            ).clicked() {
                                self.grid_sort_key = GridSortKey::Position;
                                self.grid_sort_dir = SortDirection::Ascending;
                            }
                        }
                    });

                    if session_fastest_lap > 0.0 {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("SESSION BEST").color(muted).size(11.0).extra_letter_spacing(0.8));
                        egui::Frame::none()
                            .fill(with_alpha(purple, 38))
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(format_time(session_fastest_lap)).color(purple).strong().size(12.5));
                            });
                        ui.label(egui::RichText::new(format!("({})", session_fastest_holder)).color(muted).size(11.5));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(egui::RichText::new("Analyze stint →").color(egui::Color32::from_rgb(20, 18, 10)).size(11.5).strong())
                                .fill(primary)
                                .rounding(egui::Rounding::same(6.0))
                        ).clicked() {
                            self.stint_analyzer_tab = StintAnalyzerTab::SingleStint;
                            self.show_stint_analyzer_modal = true;
                        }

                        if ui.add(
                            egui::Button::new(egui::RichText::new("⚖ Compare stints").color(white).size(11.5))
                                .fill(elevated)
                                .rounding(egui::Rounding::same(6.0))
                        ).on_hover_text("Compare up to 3 stints side by side").clicked() {
                            self.ensure_compare_slots_populated(s);
                            self.stint_analyzer_tab = StintAnalyzerTab::Compare;
                            self.show_stint_analyzer_modal = true;
                        }

                        if ui.add(
                            egui::Button::new(egui::RichText::new("📂 Saved Stints").color(white).size(11.5))
                                .fill(elevated)
                                .rounding(egui::Rounding::same(6.0))
                        ).on_hover_text("Open Saved Stints Database to re-import previous track stints or paste a shared stint").clicked() {
                            self.show_stint_db_modal = true;
                        }

                        if ui.add(
                            egui::Button::new(egui::RichText::new("Buckets ▼").color(white).size(11.5))
                                .fill(elevated)
                                .rounding(egui::Rounding::same(6.0))
                        ).clicked() {
                            self.show_settings_modal = true;
                        }

                        let on_track_count = s.drivers.values().filter(|d| !d.is_on_pit_road && d.is_in_world).count();
                        let in_pit_count = s.drivers.values().filter(|d| d.is_on_pit_road && d.is_in_world).count();
                        ui.label(egui::RichText::new(format!("Track {} · Pit {}", on_track_count, in_pit_count)).color(muted).size(12.0));
                    });
                });
            });

        ui.add_space(8.0);

        // 6. Interactive Clickable Sort Header Helper
        let mut new_sort_action: Option<(GridSortKey, SortDirection)> = None;

        let render_header_btn = |ui: &mut egui::Ui, title: &str, key: GridSortKey, cur_k: GridSortKey, cur_d: SortDirection, primary_col: egui::Color32| -> Option<(GridSortKey, SortDirection)> {
            let is_active = cur_k == key;
            let arrow = if is_active {
                if cur_d == SortDirection::Ascending { " ▲" } else { " ▼" }
            } else {
                ""
            };
            let text = format!("{}{}", title, arrow);
            let color = if is_active { primary_col } else { egui::Color32::from_rgb(139, 147, 163) };

            let resp = ui.add(egui::Button::new(egui::RichText::new(text).color(color).size(11.5)).frame(false));
            if resp.clicked() {
                if is_active {
                    let next_dir = if cur_d == SortDirection::Ascending { SortDirection::Descending } else { SortDirection::Ascending };
                    Some((key, next_dir))
                } else {
                    let default_dir = match key {
                        GridSortKey::LapsCompleted => SortDirection::Descending,
                        _ => SortDirection::Ascending,
                    };
                    Some((key, default_dir))
                }
            } else {
                None
            }
        };

        // 7. Full-Width Table Chassis
        egui::Frame::none()
            .fill(panel_bg)
            .stroke(card_stroke)
            .rounding(egui::Rounding::same(12.0))
            .inner_margin(egui::Margin::symmetric(14.0, 12.0))
            .show(ui, |ui| {
                let avail_h = ui.available_height();

                egui::ScrollArea::both().auto_shrink([false, false]).max_height(avail_h).show(ui, |ui| {
                    let mut table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center));

                    if cols.pos { table = table.column(Column::initial(52.0)); }
                    if cols.car_num { table = table.column(Column::initial(62.0)); }
                    if cols.driver { table = table.column(Column::initial(180.0)); }
                    if cols.irating { table = table.column(Column::initial(65.0)); }
                    if cols.stint { table = table.column(Column::initial(75.0)); }
                    if cols.laps { table = table.column(Column::initial(60.0)); }
                    if cols.last_lap { table = table.column(Column::initial(85.0)); }
                    if cols.fastest { table = table.column(Column::initial(85.0)); }
                    if cols.cur_avg { table = table.column(Column::initial(85.0)); }

                    for _ in &active_buckets {
                        table = table.column(Column::initial(80.0));
                    }

                    if cols.setup_name {
                        table = table.column(Column::initial(130.0).at_least(100.0));
                    }
                    table = table.column(Column::remainder().at_least(85.0));

                    table.header(28.0, |mut header| {
                        if cols.pos {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Pos", GridSortKey::Position, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.car_num {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Car #", GridSortKey::CarNumber, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.driver {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Driver", GridSortKey::DriverName, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.irating {
                            header.col(|ui| {
                                ui.label(egui::RichText::new("iRating").color(muted).size(11.5));
                            });
                        }
                        if cols.stint {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Stint", GridSortKey::StintNumber, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.laps {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Laps", GridSortKey::LapsCompleted, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.last_lap {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Last", GridSortKey::LastLap, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.fastest {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Fastest", GridSortKey::FastestLap, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.cur_avg {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, "Full Avg", GridSortKey::CurrentAverage, current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        for b in &active_buckets {
                            header.col(|ui| {
                                if let Some(act) = render_header_btn(ui, &format!("Avg {}", b), GridSortKey::AvgBucket(*b), current_sort_key, current_sort_dir, primary) {
                                    new_sort_action = Some(act);
                                }
                            });
                        }
                        if cols.setup_name {
                            header.col(|ui| {
                                ui.label(egui::RichText::new("Setup").color(muted).size(11.5));
                            });
                        }
                        header.col(|ui| {
                            ui.label(egui::RichText::new("Action").color(muted).size(11.5));
                        });
                    })
                    .body(|mut body| {
                        for (row_idx, e) in entries.iter().enumerate() {
                            let is_player = e.is_player;

                            let is_dark = self.settings.is_dark_theme();
                            let text_col = if is_player || e.stint_is_active {
                                white
                            } else {
                                muted
                            };

                            body.row(30.0, |mut row| {
                                // POS Badge
                                if cols.pos {
                                    row.col(|ui| {
                                        let pos_str = if e.pos > 0 { format!("P{}", e.pos) } else { "—".to_string() };
                                        let (badge_bg, badge_fg) = match e.pos {
                                            1 => (with_alpha(primary, if is_dark { 42 } else { 55 }), primary),
                                            2 => if is_dark {
                                                (egui::Color32::from_rgba_unmultiplied(220, 225, 235, 34), egui::Color32::from_rgb(220, 225, 235))
                                            } else {
                                                (egui::Color32::from_rgba_unmultiplied(100, 115, 135, 45), egui::Color32::from_rgb(55, 65, 82))
                                            },
                                            3 => if is_dark {
                                                (egui::Color32::from_rgba_unmultiplied(205, 133, 63, 40), egui::Color32::from_rgb(230, 165, 100))
                                            } else {
                                                (egui::Color32::from_rgba_unmultiplied(190, 100, 25, 45), egui::Color32::from_rgb(160, 80, 15))
                                            },
                                            _ => (elevated, muted),
                                        };

                                        egui::Frame::none()
                                            .fill(badge_bg)
                                            .rounding(egui::Rounding::same(5.0))
                                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                            .show(ui, |ui| {
                                                ui.label(egui::RichText::new(pos_str).color(badge_fg).strong().size(11.5));
                                            });
                                    });
                                }

                                // CAR #
                                if cols.car_num {
                                    row.col(|ui| {
                                        ui.label(egui::RichText::new(format!("#{}", e.car_num)).color(muted).size(12.0));
                                    });
                                }

                                // DRIVER (Double-click to analyze stint)
                                if cols.driver {
                                    row.col(|ui| {
                                        ui.push_id(("grid_drv_col", row_idx, e.car_idx, e.stint_num), |ui| {
                                            ui.horizontal(|ui| {
                                                let drv_resp = if is_player {
                                                    ui.selectable_label(false, egui::RichText::new(e.driver_name).color(primary).strong().size(13.0))
                                                } else {
                                                    ui.selectable_label(false, egui::RichText::new(e.driver_name).color(text_col).size(13.0))
                                                };
                                                if drv_resp.double_clicked() {
                                                    self.selected_driver_car_idx = Some(e.car_idx);
                                                    self.selected_stint_num = Some(e.stint_num);
                                                    self.show_stint_analyzer_modal = true;
                                                }
                                                drv_resp.on_hover_text("Double-click to analyze stint");

                                                if is_player {
                                                    egui::Frame::none()
                                                        .fill(with_alpha(primary, 34))
                                                        .rounding(egui::Rounding::same(4.0))
                                                        .inner_margin(egui::Margin::symmetric(5.0, 1.0))
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new("You").color(primary).strong().size(10.0));
                                                        });
                                                }
                                            });
                                        });
                                    });
                                }

                                // iRATING
                                if cols.irating {
                                    row.col(|ui| {
                                        let ir_str = if !e.irating.is_empty() && e.irating != "0" {
                                            let val = e.irating.parse::<f32>().unwrap_or(0.0);
                                            if val >= 1000.0 { format!("{:.1}k", val / 1000.0) } else { e.irating.to_string() }
                                        } else {
                                            "—".to_string()
                                        };
                                        ui.label(egui::RichText::new(ir_str).color(muted).size(12.0));
                                    });
                                }

                                // STINT (Click or Double-click for any driver to open Stint Analyzer)
                                if cols.stint {
                                    row.col(|ui| {
                                        ui.push_id(("grid_stint_col", row_idx, e.car_idx, e.stint_num), |ui| {
                                            let tag = if e.stint_is_active {
                                                " · Live"
                                            } else if e.stint_is_imported {
                                                " · Ref"
                                            } else if e.is_on_pit_road {
                                                " · Pit"
                                            } else {
                                                ""
                                            };
                                            let stint_btn_text = format!("{}{}", e.stint_short_label, tag);

                                            let is_sel = self.selected_driver_car_idx == Some(e.car_idx) && self.selected_stint_num == Some(e.stint_num);
                                            let resp = ui.add(
                                                egui::Button::new(egui::RichText::new(stint_btn_text).color(if is_sel || is_player { primary } else { white }).size(11.5))
                                                    .fill(if is_sel { with_alpha(secondary, 150) } else { elevated })
                                                    .rounding(egui::Rounding::same(6.0))
                                            );
                                            if resp.clicked() || resp.double_clicked() {
                                                self.selected_driver_car_idx = Some(e.car_idx);
                                                self.selected_stint_num = Some(e.stint_num);
                                                self.show_stint_analyzer_modal = true;
                                            }
                                            resp.on_hover_text("Click to analyze this stint");
                                        });
                                    });
                                }

                                // LAPS
                                if cols.laps {
                                    row.col(|ui| {
                                        ui.push_id(("grid_laps_col", row_idx, e.car_idx, e.stint_num), |ui| {
                                            let resp = ui.selectable_label(false, egui::RichText::new(e.laps_completed.to_string()).color(text_col).size(12.5));
                                            if resp.double_clicked() {
                                                self.selected_driver_car_idx = Some(e.car_idx);
                                                self.selected_stint_num = Some(e.stint_num);
                                                self.show_stint_analyzer_modal = true;
                                            }
                                        });
                                    });
                                }

                                // LAST LAP
                                if cols.last_lap {
                                    row.col(|ui| {
                                        ui.push_id(("grid_last_col", row_idx, e.car_idx, e.stint_num), |ui| {
                                            let last_str = if e.last_lap > 0.0 { format_time(e.last_lap) } else { "—".to_string() };
                                            let resp = ui.selectable_label(false, egui::RichText::new(last_str).color(text_col).size(12.5));
                                            if resp.double_clicked() {
                                                self.selected_driver_car_idx = Some(e.car_idx);
                                                self.selected_stint_num = Some(e.stint_num);
                                                self.show_stint_analyzer_modal = true;
                                            }
                                        });
                                    });
                                }

                                // FASTEST LAP
                                if cols.fastest {
                                    row.col(|ui| {
                                        if e.fast_lap > 0.0 {
                                            let is_session_best = (e.fast_lap - session_fastest_lap).abs() < 0.0001;
                                            let fast_str = format_time(e.fast_lap);

                                            if is_session_best {
                                                egui::Frame::none()
                                                    .fill(with_alpha(purple, 42))
                                                    .rounding(egui::Rounding::same(5.0))
                                                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                                    .show(ui, |ui| {
                                                        ui.label(egui::RichText::new(fast_str).color(purple).strong().size(12.0));
                                                    });
                                            } else {
                                                ui.label(egui::RichText::new(fast_str).color(white).strong().size(12.5));
                                            }
                                        } else {
                                            ui.label(egui::RichText::new("—").color(muted));
                                        }
                                    });
                                }

                                // FULL AVG
                                if cols.cur_avg {
                                    row.col(|ui| {
                                        if e.full_avg > 0.0 {
                                            let avg_str = format_time(e.full_avg);
                                            ui.label(egui::RichText::new(avg_str).color(text_col).strong().size(12.5));
                                        } else {
                                            ui.label(egui::RichText::new("—").color(muted));
                                        }
                                    });
                                }

                                // BUCKET COLUMNS
                                for (b_idx, &(b, avg_val)) in e.bucket_avgs.iter().enumerate() {
                                    row.col(|ui| {
                                        ui.push_id(("grid_b_cell", row_idx, b_idx, b), |ui| {
                                            if avg_val > 0.0 {
                                                let is_bucket_best = session_fastest_buckets.get(&b).map_or(false, |&best| (avg_val - best).abs() < 0.0001);
                                                let val_str = format_time(avg_val);

                                                if is_bucket_best {
                                                    egui::Frame::none()
                                                        .fill(with_alpha(purple, 42))
                                                        .rounding(egui::Rounding::same(5.0))
                                                        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                                                        .show(ui, |ui| {
                                                            ui.label(egui::RichText::new(val_str).color(purple).strong().size(12.0));
                                                        });
                                                } else {
                                                    let is_sorted_col = current_sort_key == GridSortKey::AvgBucket(b);
                                                    let cell_color = if is_sorted_col { primary } else { text_col };
                                                    ui.label(egui::RichText::new(val_str).color(cell_color).size(12.5));
                                                }
                                            } else {
                                                ui.label(egui::RichText::new("—").color(muted));
                                            }
                                        });
                                    });
                                }

                                // SETUP NAME
                                if cols.setup_name {
                                    row.col(|ui| {
                                        let setup_str = if !e.setup_name.is_empty() && e.setup_name != "—" { e.setup_name } else { "Baseline" };
                                        ui.label(egui::RichText::new(setup_str).color(muted).size(11.5));
                                    });
                                }

                                // ACTION (Analyze Button)
                                row.col(|ui| {
                                    ui.push_id(("grid_analyze_btn", row_idx, e.car_idx, e.stint_num), |ui| {
                                        if ui.add(
                                            egui::Button::new(egui::RichText::new("Analyze →").color(if is_player { primary } else { white }).size(11.0))
                                                .fill(elevated)
                                                .rounding(egui::Rounding::same(6.0))
                                        ).clicked() {
                                            self.selected_driver_car_idx = Some(e.car_idx);
                                            self.selected_stint_num = Some(e.stint_num);
                                            self.show_stint_analyzer_modal = true;
                                        }
                                    });
                                });
                            });
                        }
                    });
                });
            });

        if let Some((new_k, new_d)) = new_sort_action {
            self.grid_sort_key = new_k;
            self.grid_sort_dir = new_d;
        }
    }
}

