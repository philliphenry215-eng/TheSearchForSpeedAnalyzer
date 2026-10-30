use eframe::egui;
use crate::{display_font, with_alpha, WarRoomApp};
use crate::formatters::*;
use crate::models::*;
use crate::views::*;
use egui_extras::{Column, TableBuilder};
use crate::stint_db;
use crate::telemetry::is_real_track_name;
use crate::views::stint_modal::render_tire_corner_card;

impl WarRoomApp {
    pub(crate) fn render_cockpit_view(&mut self, ui: &mut egui::Ui, s: &WarRoomState) {
        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let panel_bg = self.settings.panel_color();
        let elevated = self.settings.card_elevated_color();
        let green = self.settings.status_green();
        let amber = self.settings.status_amber();

        let is_metric = self.settings.use_metric;
        let fuel_conv = if is_metric { 1.0 } else { 0.264172 };
        let fuel_unit_str = if is_metric { "L" } else { "gal" };

        let player = s.drivers.get(&s.player_car_idx);
        let active_stint = player.and_then(|p| p.stints.iter().rev().find(|st| st.is_active).or_else(|| p.stints.last()));
        let last_completed_stint = player.and_then(|p| p.stints.iter().rev().find(|st| st.laps_completed > 0));
        
        let avg_fuel_burn_ltr = active_stint
            .map(|st| st.fuel_per_lap())
            .filter(|&r| r > 0.001)
            .unwrap_or_else(|| player.map_or(0.0, |d| d.estimated_fuel_burn_ltr()));

        let laps_of_fuel = active_stint
            .filter(|st| st.is_active && !s.is_under_caution && !s.debug_on_pit_road)
            .map(|st| st.laps_of_fuel_crossing)
            .filter(|&l| l > 0.0)
            .unwrap_or_else(|| {
                if avg_fuel_burn_ltr > 0.001 && s.player_fuel_level > 0.01 {
                    (s.player_fuel_level / avg_fuel_burn_ltr).max(0.0)
                } else {
                    0.0
                }
            });

        let display_fuel_level = s.player_fuel_level * fuel_conv;
        let display_fuel_cap = s.player_fuel_capacity * fuel_conv;
        let display_fuel_burn = avg_fuel_burn_ltr * fuel_conv;

        let total_avail_w = ui.available_width();
        let total_avail_h = ui.available_height();

        let col1_w = (total_avail_w * 0.21).clamp(240.0, 280.0);
        let col3_w = (total_avail_w * 0.33).clamp(345.0, 420.0);
        let col2_w = (total_avail_w - col1_w - col3_w - 24.0).max(300.0);

        let card_stroke = egui::Stroke::new(1.0, self.settings.divider_color());

        ui.horizontal_top(|ui| {
            // --- COLUMN 1: FUEL & STINT ---
            ui.allocate_ui_with_layout(egui::vec2(col1_w, total_avail_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::Frame::none()
                    .fill(panel_bg)
                    .stroke(card_stroke)
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            ui.label(egui::RichText::new("FUEL").color(muted).size(11.0).extra_letter_spacing(1.0));
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                let laps_fuel_str = if laps_of_fuel > 0.05 && avg_fuel_burn_ltr > 0.001 {
                                    format!("{:.1}", laps_of_fuel)
                                } else {
                                    "—".to_string()
                                };
                                ui.label(egui::RichText::new(laps_fuel_str).font(display_font(56.0)).color(white));
                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.add_space(18.0);
                                    ui.label(egui::RichText::new("laps left").color(muted).size(13.0));
                                });
                            });

                            ui.add_space(10.0);
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 8.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(30, 35, 45));
                            let pct = (s.player_fuel_level / s.player_fuel_capacity.max(1.0)).clamp(0.0, 1.0);
                            let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * pct, rect.height()));
                            ui.painter().rect_filled(fill_rect, 4.0, primary);

                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("{:.2} {}", display_fuel_level, fuel_unit_str)).size(12.0).color(muted));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format!("of {:.1} {}", display_fuel_cap, fuel_unit_str)).size(12.0).color(muted));
                                });
                            });

                            ui.add_space(16.0);

                            let last_lap_time = active_stint.map_or(0.0, |st| st.last_lap);
                            let best_lap_time = active_stint
                                .map(|st| st.fast_lap)
                                .filter(|&t| t > 0.0)
                                .or_else(|| {
                                    player.and_then(|p| {
                                        p.stints.iter()
                                            .map(|st| st.fast_lap)
                                            .filter(|&t| t > 0.0)
                                            .fold(None, |min, t| match min {
                                                None => Some(t),
                                                Some(m) if t < m => Some(t),
                                                Some(m) => Some(m),
                                            })
                                    })
                                })
                                .unwrap_or(0.0);
                            let avg_5 = active_stint.map_or(0.0, |st| st.avg_5());
                            let full_avg = active_stint.map_or(0.0, |st| st.full_average());
                            let stint_laps = active_stint.map_or(0, |st| st.laps_completed);
                            let std_dev = active_stint.map_or(0.0, |st| st.std_dev());

                            egui::Grid::new("fuel_metrics_grid").num_columns(2).spacing([10.0, 12.0]).show(ui, |ui| {
                                ui.label(egui::RichText::new("Burn / lap").color(muted).size(13.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let burn_str = if display_fuel_burn > 0.0005 {
                                        format!("{:.3} {}", display_fuel_burn, fuel_unit_str)
                                    } else {
                                        "—".to_string()
                                    };
                                    ui.label(egui::RichText::new(burn_str).color(white).size(14.0).strong());
                                });
                                ui.end_row();

                                ui.label(egui::RichText::new("Last lap").color(muted).size(13.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format_time(last_lap_time)).color(primary).size(14.5).strong());
                                });
                                ui.end_row();

                                ui.label(egui::RichText::new("Best lap").color(muted).size(13.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let best_color = if best_lap_time > 0.0 { green } else { muted };
                                    let resp = ui.label(egui::RichText::new(format_time(best_lap_time)).color(best_color).size(14.5).strong());
                                    if best_lap_time > 0.0 {
                                        resp.on_hover_text("Fastest lap completed by driver in this session");
                                    }
                                });
                                ui.end_row();

                                ui.label(egui::RichText::new("5-lap avg").color(muted).size(13.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format_time(avg_5)).color(white).size(14.0).strong());
                                });
                                ui.end_row();

                                ui.label(egui::RichText::new("Stint avg").color(muted).size(13.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format_time(full_avg)).color(white).size(14.0).strong());
                                });
                                ui.end_row();

                                ui.label(egui::RichText::new("Stint laps").color(muted).size(13.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(stint_laps.to_string()).color(white).size(14.0).strong());
                                });
                                ui.end_row();

                                ui.label(egui::RichText::new("Consistency σ").color(muted).size(13.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(egui::RichText::new(format!("{:.3}", std_dev)).color(white).size(14.0).strong());
                                });
                                ui.end_row();
                            });
                        });
                    });
            });

            ui.add_space(4.0);

            // --- COLUMN 2: STINT LOG & ROLLING AVERAGES TABLE ---
            ui.allocate_ui_with_layout(egui::vec2(col2_w, total_avail_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::Frame::none()
                    .fill(panel_bg)
                    .stroke(card_stroke)
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        let mut remove_imported_stint_num: Option<i32> = None;
                        let saved_track_count = if is_real_track_name(&s.track_name) {
                            self.stint_db.records.iter().filter(|r| stint_db::tracks_match(&r.track_name, &s.track_name)).count()
                        } else {
                            self.stint_db.records.len()
                        };
                        let saved_btn_label = if saved_track_count > 0 {
                            format!("📂 Saved Stints ({})", saved_track_count)
                        } else {
                            "📂 Saved Stints".to_string()
                        };

                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("STINT LOG").color(muted).size(11.0).extra_letter_spacing(1.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(
                                    egui::Button::new(egui::RichText::new("Buckets ▼").color(white).size(11.5))
                                        .fill(elevated)
                                        .rounding(egui::Rounding::same(6.0))
                                ).clicked() {
                                    self.show_settings_modal = true;
                                }

                                if ui.add(
                                    egui::Button::new(egui::RichText::new(saved_btn_label).color(white).size(11.5))
                                        .fill(elevated)
                                        .rounding(egui::Rounding::same(6.0))
                                ).on_hover_text("Open Saved Stints Database to re-import previous track stints or paste a shared stint").clicked() {
                                    self.show_stint_db_modal = true;
                                }
                            });
                        });
                        ui.add_space(8.0);

                        let active_buckets = self.settings.active_avg_buckets.clone();
                        let table_avail_h = (ui.available_height() - 48.0).max(100.0);

                        egui::ScrollArea::both().auto_shrink([false, false]).max_height(table_avail_h).show(ui, |ui| {
                            let mut table = TableBuilder::new(ui)
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .column(Column::initial(115.0).at_least(95.0))
                                .column(Column::initial(65.0).at_least(55.0));

                            for _ in &active_buckets {
                                table = table.column(Column::initial(82.0).at_least(72.0));
                            }

                            table = table
                                .column(Column::initial(82.0).at_least(72.0))
                                .column(Column::initial(72.0).at_least(62.0))
                                .column(Column::remainder().at_least(55.0));

                            table.header(26.0, |mut header| {
                                header.col(|ui| { ui.label(egui::RichText::new("Stint").color(muted).size(11.5)); });
                                header.col(|ui| { ui.label(egui::RichText::new("Laps").color(muted).size(11.5)); });
                                for b in &active_buckets {
                                    header.col(|ui| { ui.label(egui::RichText::new(format!("Avg {}", b)).color(muted).size(11.5)); });
                                }
                                header.col(|ui| { ui.label(egui::RichText::new("Full").color(muted).size(11.5)); });
                                header.col(|ui| { ui.label(egui::RichText::new("Fuel/lap").color(muted).size(11.5)); });
                                header.col(|ui| { ui.label(egui::RichText::new("σ").color(muted).size(11.5)); });
                            })
                            .body(|mut body| {
                                if let Some(drv) = player {
                                    for stint in drv.stints.iter().rev() {
                                        let row_color = if stint.is_active { white } else { muted };
                                        let is_selected = self.selected_stint_num == Some(stint.stint_number);
                                        let stint_lbl = stint.short_label();

                                        body.row(30.0, |mut row| {
                                            row.col(|ui| {
                                                ui.push_id(("cockpit_stint_col", stint.stint_number), |ui| {
                                                    ui.horizontal(|ui| {
                                                        let resp = ui.selectable_label(is_selected, egui::RichText::new(&stint_lbl).color(if is_selected { primary } else { white }).strong().size(13.0));
                                                        if resp.double_clicked() {
                                                            let tsv = format_full_stint_excel_tsv(stint, &active_buckets, is_metric);
                                                            ui.output_mut(|o| o.copied_text = tsv);
                                                            self.toast_message = Some((format!("Copied Stint {} for Excel!", stint_lbl), std::time::Instant::now()));
                                                        } else if resp.clicked() {
                                                            self.selected_driver_car_idx = Some(s.player_car_idx);
                                                            self.selected_stint_num = Some(stint.stint_number);
                                                            self.show_stint_analyzer_modal = true;
                                                        }
                                                        if !stint.saved_label.is_empty() {
                                                            resp.on_hover_text(format!("Re-imported stint: {}", stint.saved_label));
                                                        }
                                                        if stint.is_active {
                                                            egui::Frame::none()
                                                                .fill(with_alpha(green, 32))
                                                                .rounding(egui::Rounding::same(4.0))
                                                                .inner_margin(egui::Margin::symmetric(5.0, 1.5))
                                                                .show(ui, |ui| {
                                                                    ui.label(egui::RichText::new("Live").color(green).size(10.5).strong());
                                                                });
                                                        } else if stint.is_imported {
                                                            egui::Frame::none()
                                                                .fill(with_alpha(secondary, 36))
                                                                .rounding(egui::Rounding::same(4.0))
                                                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                                .show(ui, |ui| {
                                                                    ui.label(egui::RichText::new("Ref").color(secondary).size(10.0).strong());
                                                                });
                                                            if ui.add(egui::Button::new(egui::RichText::new("×").color(muted).size(10.0)).frame(false)).on_hover_text("Unload this imported stint from current session (remains saved in your Stint Database)").clicked() {
                                                                remove_imported_stint_num = Some(stint.stint_number);
                                                            }
                                                        }
                                                    });
                                                });
                                            });
                                            row.col(|ui| {
                                                ui.push_id(("cockpit_laps_col", stint.stint_number), |ui| {
                                                    let resp = ui.selectable_label(false, egui::RichText::new(format!("{}", stint.laps_completed)).color(row_color));
                                                    if resp.double_clicked() {
                                                        let tsv = format_stint_laps_tsv(stint);
                                                        ui.output_mut(|o| o.copied_text = tsv);
                                                        self.toast_message = Some((format!("Copied {} Laps for Excel", stint_lbl), std::time::Instant::now()));
                                                    } else if resp.clicked() {
                                                        self.selected_driver_car_idx = Some(s.player_car_idx);
                                                        self.selected_stint_num = Some(stint.stint_number);
                                                        self.show_stint_analyzer_modal = true;
                                                    }
                                                });
                                            });
                                            for (b_idx, b) in active_buckets.iter().enumerate() {
                                                let avg_val = stint.avg_first_n(*b);
                                                let avg_str = if avg_val > 0.0 { format_time(avg_val) } else { "—".to_string() };
                                                row.col(|ui| {
                                                    ui.push_id(("cockpit_b_col", stint.stint_number, b_idx, *b), |ui| {
                                                        let resp = ui.selectable_label(false, egui::RichText::new(avg_str).color(row_color));
                                                        if resp.double_clicked() {
                                                            let tsv = format_full_stint_excel_tsv(stint, &active_buckets, is_metric);
                                                            ui.output_mut(|o| o.copied_text = tsv);
                                                            self.toast_message = Some((format!("Copied Stint {} for Excel", stint_lbl), std::time::Instant::now()));
                                                        }
                                                    });
                                                });
                                            }
                                            row.col(|ui| { ui.label(egui::RichText::new(format_time(stint.full_average())).color(row_color).strong()); });
                                            let f_val = stint.fuel_per_lap() * fuel_conv;
                                            let f_str = if f_val > 0.0005 { format!("{:.3}", f_val) } else { "—".to_string() };
                                            row.col(|ui| { ui.label(egui::RichText::new(f_str).color(row_color)); });
                                            row.col(|ui| { ui.label(egui::RichText::new(format!("{:.3}", stint.std_dev())).color(row_color)); });
                                        });
                                    }
                                }
                            });
                        });

                        if let Some(rem_num) = remove_imported_stint_num {
                            if let Ok(mut state_guard) = self.state.lock() {
                                state_guard.remove_imported_stint(rem_num);
                            }
                            if self.selected_stint_num == Some(rem_num) {
                                self.selected_stint_num = None;
                            }
                        }

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Double-click a row to copy for Excel").color(muted).size(11.5));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(
                                    egui::Button::new(egui::RichText::new("Analyze stint →").color(egui::Color32::from_rgb(20, 18, 10)).size(13.0).strong())
                                        .fill(primary)
                                        .rounding(egui::Rounding::same(8.0))
                                ).clicked() {
                                    self.selected_driver_car_idx = Some(s.player_car_idx);
                                    self.stint_analyzer_tab = StintAnalyzerTab::SingleStint;
                                    self.show_stint_analyzer_modal = true;
                                }

                                if ui.add(
                                    egui::Button::new(egui::RichText::new("⚖ Compare stints").color(white).size(12.5))
                                        .fill(elevated)
                                        .rounding(egui::Rounding::same(8.0))
                                ).on_hover_text("Compare up to 3 stints side by side").clicked() {
                                    self.ensure_compare_slots_populated(s);
                                    self.stint_analyzer_tab = StintAnalyzerTab::Compare;
                                    self.show_stint_analyzer_modal = true;
                                }
                            });
                        });
                    });
            });

            ui.add_space(4.0);

            // --- COLUMN 3: PIT STRATEGY, TIRES & RIVALS ---
            ui.allocate_ui_with_layout(egui::vec2(col3_w, total_avail_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                egui::Frame::none()
                    .fill(panel_bg)
                    .stroke(card_stroke)
                    .rounding(egui::Rounding::same(12.0))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                            ui.label(egui::RichText::new("PIT CALL").color(muted).size(11.0).extra_letter_spacing(1.0));
                            ui.add_space(6.0);

                            // Tinted Pit Call Card with Left Accent Bar (Option A)
                            let pit_card_resp = egui::Frame::none()
                                .fill(with_alpha(secondary, 34))
                                .rounding(egui::Rounding::same(10.0))
                                .inner_margin(egui::Margin { left: 16.0, right: 12.0, top: 10.0, bottom: 10.0 })
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (pit_target_str, window_str) = if laps_of_fuel > 0.5 && avg_fuel_burn_ltr > 0.001 {
                                            let current_lap = player.map_or(1, |p| p.total_laps_completed);
                                            let pit_target = current_lap + (laps_of_fuel as i32).max(1);
                                            (format!("L{}", pit_target), format!("Window L{}–L{}", (pit_target - 8).max(1), pit_target + 4))
                                        } else {
                                            ("—".to_string(), "Awaiting Lap 1".to_string())
                                        };
                                        ui.label(egui::RichText::new(pit_target_str).font(display_font(42.0)).color(white));
                                        ui.add_space(10.0);
                                        ui.vertical(|ui| {
                                            ui.add_space(4.0);
                                            let win_color = if self.settings.is_dark_theme() { egui::Color32::from_rgb(143, 180, 238) } else { secondary };
                                            ui.label(egui::RichText::new(window_str).color(win_color).size(13.0).strong());
                                            ui.label(egui::RichText::new("Full tank + 4 tires").color(muted).size(12.0));
                                        });
                                    });
                                });
                            let r = pit_card_resp.response.rect;
                            let bar_rect = egui::Rect::from_min_max(r.min, egui::pos2(r.min.x + 3.5, r.max.y));
                            ui.painter().rect_filled(bar_rect, egui::Rounding { nw: 10.0, sw: 10.0, ne: 0.0, se: 0.0 }, secondary);

                            ui.add_space(16.0);

                            // --- 4-CORNER POST-STINT TIRE REPORT ---
                            let selected_stint = self.selected_stint_num.and_then(|num| player.and_then(|p| p.stints.iter().find(|st| st.stint_number == num)))
                                .or(last_completed_stint);

                            let stint_has_tires = selected_stint.map_or(false, |st| st.tires.has_data && st.laps_completed > 0);
                            let s_lbl = selected_stint.map_or("S1".to_string(), |st| st.short_label());

                            let empty_tires = StintTireSet::default();
                            let tires_to_show = if stint_has_tires {
                                selected_stint.map(|st| &st.tires).unwrap_or(&empty_tires)
                            } else {
                                &empty_tires
                            };

                            ui.horizontal(|ui| {
                                let header_lbl = if stint_has_tires {
                                    format!("TIRES · {}", s_lbl)
                                } else {
                                    "TIRES · AWAITING PIT".to_string()
                                };
                                ui.label(egui::RichText::new(header_lbl).color(muted).size(11.0).extra_letter_spacing(1.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if stint_has_tires {
                                        ui.menu_button(egui::RichText::new("Copy ▼").color(muted).size(11.5), |ui| {
                                            if ui.button("Copy 4-Tire Temps").clicked() {
                                                let tsv = format_temps_excel_tsv(tires_to_show, is_metric);
                                                ui.output_mut(|o| o.copied_text = tsv);
                                                self.toast_message = Some(("Copied 4-Tire Temps for Excel".to_string(), std::time::Instant::now()));
                                                ui.close_menu();
                                            }
                                            if ui.button("Copy 4-Tire Wear").clicked() {
                                                let tsv = format_wear_excel_tsv(tires_to_show);
                                                ui.output_mut(|o| o.copied_text = tsv);
                                                self.toast_message = Some(("Copied 4-Tire Wear for Excel".to_string(), std::time::Instant::now()));
                                                ui.close_menu();
                                            }
                                            if ui.button("Copy Full 4-Corner Table").clicked() {
                                                let tsv = format_tires_excel_tsv(tires_to_show, is_metric);
                                                ui.output_mut(|o| o.copied_text = tsv);
                                                self.toast_message = Some(("Copied 4-Corner Tires for Excel".to_string(), std::time::Instant::now()));
                                                ui.close_menu();
                                            }
                                        });
                                    }
                                });
                            });
                            ui.add_space(6.0);

                            ui.columns(2, |tire_cols| {
                                tire_cols[0].vertical(|ui| {
                                    render_tire_corner_card(ui, "LF", &tires_to_show.lf, is_metric, &self.settings);
                                    ui.add_space(6.0);
                                    render_tire_corner_card(ui, "LR", &tires_to_show.lr, is_metric, &self.settings);
                                 });
                                tire_cols[1].vertical(|ui| {
                                    render_tire_corner_card(ui, "RF", &tires_to_show.rf, is_metric, &self.settings);
                                    ui.add_space(6.0);
                                    render_tire_corner_card(ui, "RR", &tires_to_show.rr, is_metric, &self.settings);
                                });
                            });

                            ui.add_space(16.0);

                            ui.label(egui::RichText::new("RIVALS").color(muted).size(11.0).extra_letter_spacing(1.0));
                            ui.add_space(6.0);

                            let rivals: Vec<&DriverState> = s.drivers.values()
                                .filter(|d| d.car_idx != s.player_car_idx && d.position > 0)
                                .take(4)
                                .collect();

                            if rivals.is_empty() {
                                ui.label(egui::RichText::new("No rival cars on track yet").color(muted).size(12.5));
                            } else {
                                for rival in rivals {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(format!("P{}", rival.position)).size(13.0).color(muted));
                                        ui.label(egui::RichText::new(format!("#{} {}", rival.car_num, rival.driver_name)).size(13.0).color(white));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if rival.is_on_pit_road {
                                                egui::Frame::none()
                                                    .fill(with_alpha(amber, 32))
                                                    .rounding(egui::Rounding::same(4.0))
                                                    .inner_margin(egui::Margin::symmetric(6.0, 1.5))
                                                    .show(ui, |ui| {
                                                        ui.label(egui::RichText::new("In pit").color(amber).size(11.0).strong());
                                                    });
                                            } else {
                                                ui.label(egui::RichText::new("Track").color(muted).size(11.5));
                                            }
                                        });
                                    });
                                    ui.add_space(2.0);
                                }
                            }
                        });
                    });
            });
        });
    }
}
