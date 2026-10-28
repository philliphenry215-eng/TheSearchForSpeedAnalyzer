use eframe::egui;
use egui_extras::{Column, TableBuilder};
use crate::{display_font, with_alpha, WarRoomApp};
use crate::formatters::*;
use crate::models::*;
use crate::views::*;
use crate::discord_card;
use crate::stint_db;

pub(crate) fn render_tire_corner_card(
    ui: &mut egui::Ui,
    corner_title: &str,
    data: &TireCornerData,
    use_metric: bool,
    primary: egui::Color32,
    _secondary: egui::Color32,
    white: egui::Color32,
) {
    let avg_wear = if data.wear_m > 0.01 {
        (data.wear_l + data.wear_m + data.wear_r) / 3.0
    } else {
        0.0
    };

    let muted = egui::Color32::from_rgb(139, 147, 163);
    let wear_color = if avg_wear >= 90.0 {
        egui::Color32::from_rgb(75, 225, 110) // Crisp green (>90%)
    } else if avg_wear >= 80.0 {
        primary // Vibrant Gold (80-89%)
    } else if avg_wear >= 70.0 {
        egui::Color32::from_rgb(255, 165, 50) // Amber (70-79%)
    } else if avg_wear > 0.0 {
        egui::Color32::from_rgb(255, 75, 75) // Red (<70%)
    } else {
        muted
    };

    let press_str = if data.pressure > 0.1 {
        if use_metric {
            format!("{:.2} bar", data.pressure / 100.0)
        } else {
            format!("{:.1} psi", data.pressure * 0.145038)
        }
    } else {
        "— psi".to_string()
    };

    let is_left = corner_title.contains("LEFT") || corner_title.starts_with("LF") || corner_title.starts_with("LR");

    let zone_label = if is_left {
        "O·M·I"
    } else {
        "I·M·O"
    };

    let temp_str = if data.temp_m > 1.0 {
        if use_metric {
            format!("{:.0}°\u{00A0}·\u{00A0}{:.0}°\u{00A0}·\u{00A0}{:.0}°C", data.temp_l, data.temp_m, data.temp_r)
        } else {
            format!("{:.0}°\u{00A0}·\u{00A0}{:.0}°\u{00A0}·\u{00A0}{:.0}°F", data.temp_l * 1.8 + 32.0, data.temp_m * 1.8 + 32.0, data.temp_r * 1.8 + 32.0)
        }
    } else {
        "—\u{00A0}·\u{00A0}—\u{00A0}·\u{00A0}—".to_string()
    };

    let wear_str = if data.wear_m > 0.01 {
        format!("{:.0}%\u{00A0}·\u{00A0}{:.0}%\u{00A0}·\u{00A0}{:.0}%", data.wear_l, data.wear_m, data.wear_r)
    } else {
        "—%\u{00A0}·\u{00A0}—%\u{00A0}·\u{00A0}—%".to_string()
    };

    let card_border = if avg_wear > 0.0 {
        with_alpha(wear_color, 70)
    } else {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    };

    egui::Frame::none()
        .fill(egui::Color32::from_rgb(25, 29, 38))
        .stroke(egui::Stroke::new(1.0, card_border))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(8.0, 7.0))
        .show(ui, |ui| {
            ui.spacing_mut().button_padding = egui::vec2(2.0, 2.0);
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(corner_title).color(white).size(12.5).strong());
                if avg_wear > 0.0 {
                    ui.label(egui::RichText::new(format!("{:.0}%", avg_wear)).color(wear_color).size(12.0).strong());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(press_str).color(muted).size(11.5));
                });
            });
            ui.add_space(5.0);

            ui.label(egui::RichText::new(format!("TEMPS ({})", zone_label)).color(muted).size(9.5).extra_letter_spacing(0.4));
            let temp_resp = ui.add(
                egui::Button::new(egui::RichText::new(&temp_str).color(white).size(12.0).strong())
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false)
                    .wrap(false)
            );
            if temp_resp.clicked() && data.temp_m > 1.0 {
                let tsv = format_single_tire_temps_tsv(data, use_metric);
                ui.output_mut(|o| o.copied_text = tsv);
            }
            temp_resp.on_hover_text("Click to copy temperatures for Excel");

            ui.add_space(3.0);

            ui.label(egui::RichText::new(format!("WEAR ({})", zone_label)).color(muted).size(9.5).extra_letter_spacing(0.4));
            let wear_resp = ui.add(
                egui::Button::new(egui::RichText::new(&wear_str).color(wear_color).size(12.0).strong())
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false)
                    .wrap(false)
            );
            if wear_resp.clicked() && data.wear_m > 0.01 {
                let tsv = format_single_tire_wear_tsv(data);
                ui.output_mut(|o| o.copied_text = tsv);
            }
            wear_resp.on_hover_text("Click to copy wear for Excel");
        });
}

pub(crate) fn render_stint_lap_chart(
    ui: &mut egui::Ui,
    stint: &StintData,
    height: f32,
    primary: egui::Color32,
    _secondary: egui::Color32,
    white: egui::Color32,
) {
    let muted = egui::Color32::from_rgb(139, 147, 163);
    let valid_laps = stint.valid_laps();
    if valid_laps.is_empty() {
        ui.label(egui::RichText::new("No completed green laps recorded yet for this stint.").color(muted));
        return;
    }

    let min_t = valid_laps.iter().cloned().fold(f32::INFINITY, f32::min);
    let max_t = valid_laps.iter().cloned().fold(f32::NEG_INFINITY, f32::max);

    let margin = (max_t - min_t).max(0.300) * 0.20;
    let y_min = (min_t - margin).max(0.0);
    let y_max = max_t + margin;
    let y_range = (y_max - y_min).max(0.001);

    let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    let rect = response.rect;

    // Background & Soft Rounding
    painter.rect_filled(rect, 10.0, egui::Color32::from_rgb(14, 17, 23));
    painter.rect_stroke(rect, 10.0, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)));

    let left_pad = 58.0;
    let right_pad = 20.0;
    let top_pad = 20.0;
    let bottom_pad = 26.0;

    let chart_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + left_pad, rect.min.y + top_pad),
        egui::pos2(rect.max.x - right_pad, rect.max.y - bottom_pad),
    );

    // Draw horizontal grid lines (Y-axis)
    let num_y_ticks = 4;
    for i in 0..=num_y_ticks {
        let frac = i as f32 / num_y_ticks as f32;
        let y_val = y_min + frac * y_range;
        let y_pos = chart_rect.max.y - frac * chart_rect.height();

        painter.line_segment(
            [egui::pos2(chart_rect.min.x, y_pos), egui::pos2(chart_rect.max.x, y_pos)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(28, 33, 44)),
        );

        painter.text(
            egui::pos2(rect.min.x + 10.0, y_pos),
            egui::Align2::LEFT_CENTER,
            format_time(y_val),
            egui::FontId::proportional(11.0),
            muted,
        );
    }

    // Map lap points to screen coordinates
    let n = valid_laps.len();
    let x_step = if n > 1 { chart_rect.width() / (n - 1) as f32 } else { 0.0 };

    let mut points = Vec::new();
    for (i, &t) in valid_laps.iter().enumerate() {
        let x = if n > 1 { chart_rect.min.x + i as f32 * x_step } else { chart_rect.center().x };
        let y = chart_rect.max.y - ((t - y_min) / y_range) * chart_rect.height();
        points.push(egui::pos2(x, y));

        painter.text(
            egui::pos2(x, chart_rect.max.y + 12.0),
            egui::Align2::CENTER_CENTER,
            format!("L{}", i + 1),
            egui::FontId::proportional(10.0),
            muted,
        );
    }

    // Draw Lap Trend Line
    for w in points.windows(2) {
        painter.line_segment([w[0], w[1]], egui::Stroke::new(2.2, primary));
    }

    // Draw Data Points & Fast Lap Highlight
    let mouse_pos = response.hover_pos();
    let mut hovered_lap: Option<(usize, f32, egui::Pos2)> = None;

    for (i, &pt) in points.iter().enumerate() {
        let t = valid_laps[i];
        let is_fast = (t - min_t).abs() < 0.0001;

        if is_fast {
            painter.circle_filled(pt, 5.5, egui::Color32::from_rgb(75, 225, 110));
            painter.circle_stroke(pt, 5.5, egui::Stroke::new(1.5, white));
        } else {
            painter.circle_filled(pt, 3.5, primary);
            painter.circle_stroke(pt, 3.5, egui::Stroke::new(1.0, egui::Color32::BLACK));
        }

        if let Some(mp) = mouse_pos {
            if (mp - pt).length() < 16.0 {
                hovered_lap = Some((i + 1, t, pt));
            }
        }
    }

    // Hover Tooltip
    if let Some((lap_num, lap_time, pt)) = hovered_lap {
        painter.circle_filled(pt, 7.0, white);
        let delta = lap_time - min_t;
        let tooltip_text = if delta <= 0.001 {
            format!("Lap {}: {} (Best)", lap_num, format_time(lap_time))
        } else {
            format!("Lap {}: {} (+{:.3}s)", lap_num, format_time(lap_time), delta)
        };

        let tooltip_pos = egui::pos2(pt.x, pt.y - 18.0);
        let galley = painter.layout_no_wrap(tooltip_text, egui::FontId::proportional(12.0), white);
        let bg_rect = egui::Rect::from_center_size(tooltip_pos, galley.size() + egui::vec2(14.0, 8.0));
        painter.rect_filled(bg_rect, 6.0, egui::Color32::from_rgb(25, 29, 38));
        painter.rect_stroke(bg_rect, 6.0, egui::Stroke::new(1.0, primary));
        painter.galley(bg_rect.min + egui::vec2(7.0, 4.0), galley, white);
    }
}

pub(crate) fn render_multi_stint_lap_chart(
    ui: &mut egui::Ui,
    stints: &[(&StintData, String, egui::Color32)],
    height: f32,
    white: egui::Color32,
) {
    let muted = egui::Color32::from_rgb(139, 147, 163);

    let mut all_valid: Vec<Vec<f32>> = Vec::new();
    let mut max_laps = 0;
    let mut global_min = f32::INFINITY;
    let mut global_max = f32::NEG_INFINITY;

    for (st, _, _) in stints {
        let valid = st.valid_laps();
        if !valid.is_empty() {
            max_laps = max_laps.max(valid.len());
            for &t in &valid {
                global_min = global_min.min(t);
                global_max = global_max.max(t);
            }
        }
        all_valid.push(valid);
    }

    if max_laps == 0 || global_min.is_infinite() {
        ui.label(egui::RichText::new("No completed green laps recorded yet for the selected stints.").color(muted));
        return;
    }

    let margin = (global_max - global_min).max(0.300) * 0.20;
    let y_min = (global_min - margin).max(0.0);
    let y_max = global_max + margin;
    let y_range = (y_max - y_min).max(0.001);

    let (response, painter) = ui.allocate_painter(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    let rect = response.rect;

    // Background & Soft Rounding
    painter.rect_filled(rect, 10.0, egui::Color32::from_rgb(14, 17, 23));
    painter.rect_stroke(rect, 10.0, egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)));

    let left_pad = 58.0;
    let right_pad = 20.0;
    let top_pad = 28.0; // Space for legend
    let bottom_pad = 26.0;

    let chart_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + left_pad, rect.min.y + top_pad),
        egui::pos2(rect.max.x - right_pad, rect.max.y - bottom_pad),
    );

    // Draw horizontal grid lines (Y-axis)
    let num_y_ticks = 4;
    for i in 0..=num_y_ticks {
        let frac = i as f32 / num_y_ticks as f32;
        let y_val = y_min + frac * y_range;
        let y_pos = chart_rect.max.y - frac * chart_rect.height();

        painter.line_segment(
            [egui::pos2(chart_rect.min.x, y_pos), egui::pos2(chart_rect.max.x, y_pos)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(28, 33, 44)),
        );

        painter.text(
            egui::pos2(rect.min.x + 10.0, y_pos),
            egui::Align2::LEFT_CENTER,
            format_time(y_val),
            egui::FontId::proportional(11.0),
            muted,
        );
    }

    // Legend inside chart at top
    let mut legend_x = chart_rect.min.x;
    let legend_y = rect.min.y + 12.0;
    for (_, label, col) in stints {
        painter.circle_filled(egui::pos2(legend_x + 5.0, legend_y), 4.5, *col);
        let text_pos = egui::pos2(legend_x + 14.0, legend_y);
        let galley = painter.layout_no_wrap(label.clone(), egui::FontId::proportional(11.0), white);
        let text_w = galley.size().x;
        painter.galley(egui::pos2(text_pos.x, text_pos.y - galley.size().y * 0.5), galley, white);
        legend_x += text_w + 24.0;
    }

    // X-step
    let x_step = if max_laps > 1 { chart_rect.width() / (max_laps - 1) as f32 } else { 0.0 };

    // Draw X-axis tick labels
    let label_interval = if max_laps <= 15 { 1 } else if max_laps <= 35 { 2 } else if max_laps <= 70 { 5 } else { 10 };
    for lap_idx in 0..max_laps {
        let lap_num = lap_idx + 1;
        if lap_num == 1 || lap_num == max_laps || lap_num % label_interval == 0 {
            let x = if max_laps > 1 { chart_rect.min.x + lap_idx as f32 * x_step } else { chart_rect.center().x };
            painter.text(
                egui::pos2(x, chart_rect.max.y + 12.0),
                egui::Align2::CENTER_CENTER,
                format!("L{}", lap_num),
                egui::FontId::proportional(10.0),
                muted,
            );
        }
    }

    // Calculate and draw points and lines for each stint
    let mut stint_points: Vec<Vec<egui::Pos2>> = Vec::new();
    for (s_idx, valid) in all_valid.iter().enumerate() {
        let mut pts = Vec::new();
        let col = stints[s_idx].2;
        let stint_best = valid.iter().cloned().fold(f32::INFINITY, f32::min);

        for (lap_idx, &t) in valid.iter().enumerate() {
            let x = if max_laps > 1 { chart_rect.min.x + lap_idx as f32 * x_step } else { chart_rect.center().x };
            let y = chart_rect.max.y - ((t - y_min) / y_range) * chart_rect.height();
            pts.push(egui::pos2(x, y));
        }

        // Draw trend line
        for w in pts.windows(2) {
            painter.line_segment([w[0], w[1]], egui::Stroke::new(2.0, col));
        }

        // Draw dots
        for (lap_idx, &pt) in pts.iter().enumerate() {
            let t = valid[lap_idx];
            let is_stint_best = (t - stint_best).abs() < 0.0001;
            if is_stint_best {
                painter.circle_filled(pt, 5.0, col);
                painter.circle_stroke(pt, 5.0, egui::Stroke::new(1.5, white));
            } else {
                painter.circle_filled(pt, 3.0, col);
                painter.circle_stroke(pt, 3.0, egui::Stroke::new(0.8, egui::Color32::BLACK));
            }
        }

        stint_points.push(pts);
    }

    // Interactive Hover on Chart
    if let Some(mouse_pos) = response.hover_pos() {
        if chart_rect.contains(mouse_pos) && max_laps > 0 {
            let rel_x = (mouse_pos.x - chart_rect.min.x).clamp(0.0, chart_rect.width());
            let nearest_lap_idx = if max_laps > 1 {
                ((rel_x / x_step).round() as usize).min(max_laps - 1)
            } else {
                0
            };
            let nearest_x = if max_laps > 1 { chart_rect.min.x + nearest_lap_idx as f32 * x_step } else { chart_rect.center().x };

            // Draw vertical cursor line
            painter.line_segment(
                [egui::pos2(nearest_x, chart_rect.min.y), egui::pos2(nearest_x, chart_rect.max.y)],
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60)),
            );

            // Find best time on this lap among active stints
            let mut best_lap_on_this = f32::INFINITY;
            for valid in &all_valid {
                if nearest_lap_idx < valid.len() {
                    best_lap_on_this = best_lap_on_this.min(valid[nearest_lap_idx]);
                }
            }

            let mut tooltip_lines = Vec::new();
            tooltip_lines.push(format!("——— Lap {} ———", nearest_lap_idx + 1));

            for (s_idx, valid) in all_valid.iter().enumerate() {
                if nearest_lap_idx < valid.len() {
                    let pt = stint_points[s_idx][nearest_lap_idx];
                    let col = stints[s_idx].2;
                    painter.circle_filled(pt, 6.0, col);
                    painter.circle_stroke(pt, 6.0, egui::Stroke::new(2.0, white));

                    let t = valid[nearest_lap_idx];
                    let delta = t - best_lap_on_this;
                    let line = if delta <= 0.0001 {
                        format!("{}: {} (Fastest)", stints[s_idx].1, format_time(t))
                    } else {
                        format!("{}: {} (+{:.3}s)", stints[s_idx].1, format_time(t), delta)
                    };
                    tooltip_lines.push(line);
                }
            }

            let tooltip_text = tooltip_lines.join("\n");
            let tooltip_anchor = egui::pos2(nearest_x, chart_rect.min.y + 20.0);
            let galley = painter.layout_no_wrap(tooltip_text, egui::FontId::proportional(11.5), white);

            let mut tooltip_pos = tooltip_anchor;
            if tooltip_pos.x + galley.size().x + 20.0 > rect.max.x {
                tooltip_pos.x -= galley.size().x + 20.0;
            } else {
                tooltip_pos.x += 12.0;
            }

            let bg_rect = egui::Rect::from_min_size(tooltip_pos, galley.size() + egui::vec2(16.0, 10.0));
            painter.rect_filled(bg_rect, 6.0, egui::Color32::from_rgb(20, 24, 32));
            painter.rect_stroke(bg_rect, 6.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 70, 90)));
            painter.galley(bg_rect.min + egui::vec2(8.0, 5.0), galley, white);
        }
    }
}

impl WarRoomApp {
    pub fn sanitize_compare_slots(&mut self, s: &WarRoomState) {
        self.stint_compare_slots.retain(|slot| {
            if let Some(drv) = s.drivers.get(&slot.car_idx) {
                drv.stints.iter().any(|st| st.stint_number == slot.stint_number)
            } else {
                false
            }
        });
        if self.stint_compare_slots.len() > 3 {
            self.stint_compare_slots.truncate(3);
        }
    }

    pub fn ensure_compare_slots_populated(&mut self, s: &WarRoomState) {
        self.sanitize_compare_slots(s);

        let target_car = self.selected_driver_car_idx.unwrap_or(s.player_car_idx);

        // 1. If slots is empty, pick target driver's selected/latest stint, or first available driver with stints
        if self.stint_compare_slots.is_empty() {
            if let Some(drv) = s.drivers.get(&target_car) {
                let sel_num = self.selected_stint_num;
                if let Some(st) = sel_num.and_then(|num| drv.stints.iter().find(|st| st.stint_number == num)).or_else(|| drv.stints.last()) {
                    self.stint_compare_slots.push(StintCompareSlot { car_idx: target_car, stint_number: st.stint_number });
                }
            }
            if self.stint_compare_slots.is_empty() {
                if let Some((&car_idx, drv)) = s.drivers.iter().find(|(_, d)| !d.stints.is_empty()) {
                    if let Some(st) = drv.stints.last() {
                        self.stint_compare_slots.push(StintCompareSlot { car_idx, stint_number: st.stint_number });
                    }
                }
            }
        }

        // 2. If slots < 2, try to find another stint from the same driver first
        if self.stint_compare_slots.len() < 2 {
            if let Some(first_slot) = self.stint_compare_slots.first() {
                let car = first_slot.car_idx;
                if let Some(drv) = s.drivers.get(&car) {
                    if let Some(other_st) = drv.stints.iter().rev().find(|st| !self.stint_compare_slots.iter().any(|slot| slot.car_idx == car && slot.stint_number == st.stint_number)) {
                        self.stint_compare_slots.push(StintCompareSlot { car_idx: car, stint_number: other_st.stint_number });
                    }
                }
            }
        }

        // 3. If still < 2, fallback to any other driver with stints
        if self.stint_compare_slots.len() < 2 {
            for (&car_idx, drv) in &s.drivers {
                if !self.stint_compare_slots.iter().any(|slot| slot.car_idx == car_idx) && !drv.stints.is_empty() {
                    if let Some(st) = drv.stints.last() {
                        self.stint_compare_slots.push(StintCompareSlot { car_idx, stint_number: st.stint_number });
                        if self.stint_compare_slots.len() >= 2 {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn render_stint_comparison_content(&mut self, ui: &mut egui::Ui, s: &WarRoomState) {
        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let green = self.settings.status_green();
        let amber = self.settings.status_amber();
        let is_metric = self.settings.use_metric;
        let fuel_conv = if is_metric { 1.0 } else { 0.264172 };
        let fuel_unit_str = if is_metric { "L" } else { "gal" };

        self.sanitize_compare_slots(s);

        if self.stint_compare_slots.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(egui::RichText::new("No stints recorded yet in this session.").color(muted).size(15.0));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Complete laps on track (or re-import a saved stint from your database) to compare stints side by side.").color(muted).size(12.5));
                ui.add_space(12.0);
                if ui.add(
                    egui::Button::new(egui::RichText::new("📂 Open Saved Stints / Re-Import").color(egui::Color32::BLACK).size(12.5).strong())
                        .fill(primary)
                        .rounding(egui::Rounding::same(6.0))
                ).clicked() {
                    self.show_stint_db_modal = true;
                }
            });
            return;
        }

        struct ResolvedCompareEntry<'a> {
            #[allow(dead_code)]
            slot_idx: usize,
            car_idx: usize,
            stint_num: i32,
            driver_name: &'a str,
            car_num: &'a str,
            setup_name: &'a str,
            is_player: bool,
            stint: &'a StintData,
            color: egui::Color32,
            slot_label: &'static str,
        }

        let slot_colors = [
            egui::Color32::from_rgb(0, 210, 255),  // Slot 0: Cyan
            egui::Color32::from_rgb(255, 175, 40), // Slot 1: Amber
            egui::Color32::from_rgb(220, 95, 255), // Slot 2: Purple
        ];
        let slot_labels = ["STINT A", "STINT B", "STINT C"];

        let mut resolved: Vec<ResolvedCompareEntry> = Vec::new();
        for (idx, slot) in self.stint_compare_slots.iter().enumerate() {
            if let Some(drv) = s.drivers.get(&slot.car_idx) {
                if let Some(stint) = drv.stints.iter().find(|st| st.stint_number == slot.stint_number) {
                    let is_player = slot.car_idx == s.player_car_idx || stint.is_imported;
                    let setup_display = if is_player {
                        if !stint.setup_name.is_empty() && stint.setup_name != "—" && stint.setup_name != "Baseline" {
                            &stint.setup_name
                        } else if !drv.setup_name.is_empty() && drv.setup_name != "—" && drv.setup_name != "Baseline" {
                            &drv.setup_name
                        } else if !s.player_setup_name.is_empty() && s.player_setup_name != "Baseline" {
                            &s.player_setup_name
                        } else {
                            "Baseline"
                        }
                    } else {
                        "— (Hidden by iRacing sim)"
                    };

                    resolved.push(ResolvedCompareEntry {
                        slot_idx: idx,
                        car_idx: slot.car_idx,
                        stint_num: slot.stint_number,
                        driver_name: &drv.driver_name,
                        car_num: &drv.car_num,
                        setup_name: setup_display,
                        is_player,
                        stint,
                        color: slot_colors[idx % slot_colors.len()],
                        slot_label: slot_labels[idx % slot_labels.len()],
                    });
                }
            }
        }

        if resolved.is_empty() {
            ui.label(egui::RichText::new("No valid stints found for comparison.").color(muted));
            return;
        }

        let mut available_drivers: Vec<(usize, String, String, bool)> = Vec::new();
        for (&car_idx, drv) in &s.drivers {
            if !drv.stints.is_empty() {
                let is_player = car_idx == s.player_car_idx;
                let label = if is_player {
                    format!("★ #{} You ({})", drv.car_num, drv.driver_name)
                } else {
                    format!("#{} {}", drv.car_num, drv.driver_name)
                };
                available_drivers.push((car_idx, drv.car_num.clone(), label, is_player));
            }
        }
        available_drivers.sort_by(|a, b| {
            if a.3 != b.3 {
                b.3.cmp(&a.3)
            } else {
                a.1.cmp(&b.1)
            }
        });

        let mut remove_slot_idx: Option<usize> = None;
        let mut add_third_stint = false;
        let mut new_driver_for_slot: Option<(usize, usize)> = None;
        let mut new_stint_for_slot: Option<(usize, i32)> = None;
        let mut updated_notes: Option<(usize, i32, String)> = None;

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            // Action & Export Bar
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("COMPARE UP TO 3 STINTS SIDE-BY-SIDE").color(muted).size(11.0).extra_letter_spacing(0.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(
                        egui::Button::new(egui::RichText::new("📸 Copy Discord Image").color(egui::Color32::BLACK).size(11.5).strong())
                            .fill(primary)
                    ).on_hover_text("Generate a Stint Comparison graphic card and copy to clipboard for pasting (Ctrl+V) into Discord").clicked() {
                        let card_entries: Vec<discord_card::DiscordStintEntry> = resolved.iter().map(|e| {
                            discord_card::DiscordStintEntry {
                                slot_label: Some(e.slot_label),
                                driver_name: e.driver_name,
                                car_num: e.car_num,
                                setup_name: e.setup_name,
                                is_player: e.is_player,
                                stint: e.stint,
                                accent_rgb: [e.color.r(), e.color.g(), e.color.b()],
                            }
                        }).collect();
                        match discord_card::render_discord_stint_card(&card_entries, s, &self.settings)
                            .and_then(|img| discord_card::copy_rgba_image_to_clipboard(&img))
                        {
                            Ok(()) => {
                                self.toast_message = Some(("📸 Copied Stint Comparison image — press Ctrl+V in Discord!".to_string(), std::time::Instant::now()));
                            }
                            Err(err) => {
                                self.toast_message = Some((format!("⚠ Image copy failed: {}", err), std::time::Instant::now()));
                            }
                        }
                    }

                    if ui.add(egui::Button::new(egui::RichText::new("Copy Comparison Table (TSV)").color(white).size(11.5)).fill(elevated)).clicked() {
                        let export_info: Vec<(&StintData, &str, &str, &str, &str, bool)> = resolved.iter().map(|e| {
                            (e.stint, e.slot_label, e.driver_name, e.car_num, e.setup_name, e.is_player)
                        }).collect();
                        let tsv = format_stints_comparison_tsv(&export_info, &self.settings.active_avg_buckets, is_metric);
                        ui.output_mut(|o| o.copied_text = tsv);
                        self.toast_message = Some(("Copied Stint Comparison spreadsheet for Excel".to_string(), std::time::Instant::now()));
                    }

                    if resolved.len() < 3 {
                        if ui.add(egui::Button::new(egui::RichText::new("➕ Add Stint").color(white).size(11.5)).fill(elevated)).on_hover_text("Add another stint to comparison (up to 3)").clicked() {
                            add_third_stint = true;
                        }
                    }

                    if ui.add(
                        egui::Button::new(egui::RichText::new("📂 Load Saved Stint").color(white).size(11.5))
                            .fill(elevated)
                    ).on_hover_text("Re-import a saved stint from your database or paste a shared stint to compare").clicked() {
                        self.show_stint_db_modal = true;
                    }
                });
            });

            ui.add_space(6.0);

            // 1. Stint Slot Selector Cards (3 Columns)
            ui.columns(3, |cols| {
                for c_idx in 0..3 {
                    if c_idx < resolved.len() {
                        let entry = &resolved[c_idx];
                        cols[c_idx].vertical(|ui| {
                            egui::Frame::none()
                                .fill(elevated)
                                .stroke(egui::Stroke::new(1.5, entry.color))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("●").color(entry.color).size(13.0));
                                        ui.label(egui::RichText::new(entry.slot_label).color(entry.color).strong().size(12.5));
                                        if entry.stint.is_imported {
                                            egui::Frame::none()
                                                .fill(with_alpha(secondary, 42))
                                                .rounding(egui::Rounding::same(4.0))
                                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new("Imported").color(secondary).strong().size(10.0));
                                                });
                                        } else if entry.is_player {
                                            egui::Frame::none()
                                                .fill(with_alpha(primary, 34))
                                                .rounding(egui::Rounding::same(4.0))
                                                .inner_margin(egui::Margin::symmetric(4.0, 1.0))
                                                .show(ui, |ui| {
                                                    ui.label(egui::RichText::new("You").color(primary).strong().size(10.0));
                                                });
                                        }
                                        if resolved.len() > 1 {
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.add(egui::Button::new(egui::RichText::new("×").color(muted).size(11.0)).fill(egui::Color32::TRANSPARENT)).on_hover_text("Remove this stint from comparison").clicked() {
                                                    remove_slot_idx = Some(c_idx);
                                                }
                                            });
                                        }
                                    });

                                    ui.add_space(4.0);

                                    // Driver Picker
                                    let current_drv_name = format!("#{} {}", entry.car_num, entry.driver_name);
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Driver:").color(muted).size(11.0));
                                        egui::ComboBox::from_id_source(format!("cmp_drv_slot_{}", c_idx))
                                            .selected_text(egui::RichText::new(&current_drv_name).color(white).size(11.5))
                                            .width(ui.available_width().max(80.0))
                                            .show_ui(ui, |ui| {
                                                for (car_idx, _c_num, d_label, _is_p) in &available_drivers {
                                                    let is_curr = *car_idx == entry.car_idx;
                                                    if ui.selectable_label(is_curr, d_label).clicked() {
                                                        new_driver_for_slot = Some((c_idx, *car_idx));
                                                    }
                                                }
                                            });
                                    });

                                    ui.add_space(2.0);

                                    // Stint Picker
                                    let st_curr_label = format!("{} ({} laps · Fast: {})", entry.stint.short_label(), entry.stint.laps_completed, format_time(entry.stint.fast_lap));
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Stint:").color(muted).size(11.0));
                                        egui::ComboBox::from_id_source(format!("cmp_stint_slot_{}", c_idx))
                                            .selected_text(egui::RichText::new(&st_curr_label).color(white).size(11.5))
                                            .width(ui.available_width().max(80.0))
                                            .show_ui(ui, |ui| {
                                                if let Some(drv) = s.drivers.get(&entry.car_idx) {
                                                    for st in &drv.stints {
                                                        let tag = if st.is_active {
                                                            " · Live"
                                                        } else if st.is_imported {
                                                            " · Imported"
                                                        } else {
                                                            ""
                                                        };
                                                        let label = format!("{} ({} laps · Fast: {}{})", st.short_label(), st.laps_completed, format_time(st.fast_lap), tag);
                                                        let is_curr = st.stint_number == entry.stint_num;
                                                        if ui.selectable_label(is_curr, label).clicked() {
                                                            new_stint_for_slot = Some((c_idx, st.stint_number));
                                                        }
                                                    }
                                                }
                                            });
                                    });

                                    ui.add_space(4.0);
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new("Setup:").color(muted).size(10.5));
                                        ui.label(egui::RichText::new(entry.setup_name).color(if entry.is_player { white } else { muted }).size(11.0));
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(egui::RichText::new("Track:").color(muted).size(10.5));
                                        let trk = if !entry.stint.track_temp_at_start.is_empty() && entry.stint.track_temp_at_start != "—" {
                                            &entry.stint.track_temp_at_start
                                        } else {
                                            &s.track_temp_raw
                                        };
                                        ui.label(egui::RichText::new(convert_unit(trk, is_metric)).color(white).size(11.0));

                                        ui.add_space(8.0);
                                        ui.label(egui::RichText::new("Air:").color(muted).size(10.5));
                                        let air = if !entry.stint.air_temp_at_start.is_empty() && entry.stint.air_temp_at_start != "—" {
                                            &entry.stint.air_temp_at_start
                                        } else {
                                            &s.air_temp_raw
                                        };
                                        ui.label(egui::RichText::new(convert_unit(air, is_metric)).color(white).size(11.0));
                                    });
                                });
                        });
                    } else if c_idx == resolved.len() && resolved.len() < 3 {
                        cols[c_idx].vertical(|ui| {
                            egui::Frame::none()
                                .fill(with_alpha(elevated, 90))
                                .stroke(egui::Stroke::new(1.0, with_alpha(primary, 90)))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(egui::Margin::same(12.0))
                                .show(ui, |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(16.0);
                                        let add_btn = egui::Button::new(
                                            egui::RichText::new(format!("➕ Add Stint {} to Compare", (b'A' + c_idx as u8) as char))
                                                .color(primary)
                                                .size(12.5)
                                                .strong()
                                        )
                                        .fill(with_alpha(primary, 30))
                                        .rounding(egui::Rounding::same(6.0));
                                        if ui.add(add_btn).clicked() {
                                            add_third_stint = true;
                                        }
                                        ui.add_space(6.0);
                                        ui.label(egui::RichText::new("Select another stint or driver to compare").color(muted).size(10.5));
                                        ui.add_space(16.0);
                                    });
                                });
                        });
                    }
                }
            });

            ui.add_space(10.0);

            // 2. Summary Metrics Side-by-Side Comparison Card
            ui.label(egui::RichText::new("KEY METRICS & PACE COMPARISON").color(muted).size(11.0).extra_letter_spacing(0.8));
            ui.add_space(4.0);

            let overall_fastest_lap = resolved.iter().map(|e| e.stint.fast_lap).filter(|&t| t > 0.0).fold(f32::INFINITY, f32::min);
            let overall_best_avg = resolved.iter().map(|e| e.stint.full_average()).filter(|&t| t > 0.0).fold(f32::INFINITY, f32::min);
            let overall_best_std_dev = resolved.iter().map(|e| e.stint.std_dev()).filter(|&s| s > 0.0001).fold(f32::INFINITY, f32::min);

            egui::Frame::none()
                .fill(elevated)
                .rounding(egui::Rounding::same(10.0))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.columns(resolved.len(), |cols| {
                        for (i, entry) in resolved.iter().enumerate() {
                            cols[i].vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("●").color(entry.color).size(13.0));
                                    ui.label(egui::RichText::new(format!("{} (#{} {})", entry.slot_label, entry.car_num, entry.driver_name)).color(entry.color).strong().size(12.0));
                                });
                                ui.add_space(6.0);

                                // Laps Completed
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Stint Laps:").color(muted).size(11.5));
                                    ui.label(egui::RichText::new(format!("{}", entry.stint.laps_completed)).font(display_font(16.0)).color(white));
                                });

                                // Fastest Lap
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Fastest:").color(muted).size(11.5));
                                    if entry.stint.fast_lap > 0.0 {
                                        let is_best = (entry.stint.fast_lap - overall_fastest_lap).abs() < 0.0001;
                                        let delta = entry.stint.fast_lap - overall_fastest_lap;
                                        if is_best || resolved.len() == 1 {
                                            ui.label(egui::RichText::new(format_time(entry.stint.fast_lap)).font(display_font(16.0)).color(green));
                                            if resolved.len() > 1 {
                                                ui.label(egui::RichText::new("BEST").color(green).size(10.0).strong());
                                            }
                                        } else {
                                            ui.label(egui::RichText::new(format_time(entry.stint.fast_lap)).font(display_font(16.0)).color(white));
                                            ui.label(egui::RichText::new(format!("(+{:.3}s)", delta)).color(amber).size(11.0));
                                        }
                                    } else {
                                        ui.label(egui::RichText::new("—").font(display_font(16.0)).color(muted));
                                    }
                                });

                                // Average Lap
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Average:").color(muted).size(11.5));
                                    let avg = entry.stint.full_average();
                                    if avg > 0.0 {
                                        let is_best = (avg - overall_best_avg).abs() < 0.0001;
                                        let delta = avg - overall_best_avg;
                                        if is_best || resolved.len() == 1 {
                                            ui.label(egui::RichText::new(format_time(avg)).font(display_font(16.0)).color(green));
                                            if resolved.len() > 1 {
                                                ui.label(egui::RichText::new("BEST").color(green).size(10.0).strong());
                                            }
                                        } else {
                                            ui.label(egui::RichText::new(format_time(avg)).font(display_font(16.0)).color(white));
                                            ui.label(egui::RichText::new(format!("(+{:.3}s)", delta)).color(amber).size(11.0));
                                        }
                                    } else {
                                        ui.label(egui::RichText::new("—").font(display_font(16.0)).color(muted));
                                    }
                                });

                                // Consistency σ
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Consistency σ:").color(muted).size(11.5));
                                    let s_dev = entry.stint.std_dev();
                                    if s_dev > 0.0001 {
                                        let is_best = (s_dev - overall_best_std_dev).abs() < 0.0001;
                                        let col = if is_best && resolved.len() > 1 { green } else { white };
                                        ui.label(egui::RichText::new(format!("{:.3}", s_dev)).font(display_font(15.0)).color(col));
                                        if is_best && resolved.len() > 1 {
                                            ui.label(egui::RichText::new("LOWEST").color(green).size(10.0).strong());
                                        }
                                    } else {
                                        ui.label(egui::RichText::new("—").font(display_font(15.0)).color(muted));
                                    }
                                });

                                // Pace Fall-off
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Fall-off (1-5 vs End):").color(muted).size(11.5));
                                    let drop = entry.stint.drop_off();
                                    if drop.abs() > 0.0001 {
                                        let sign = if drop >= 0.0 { "+" } else { "" };
                                        let col = if drop <= 0.20 { green } else if drop <= 0.60 { white } else { amber };
                                        ui.label(egui::RichText::new(format!("{}{:.3}s", sign, drop)).color(col).size(12.5).strong());
                                    } else {
                                        ui.label(egui::RichText::new("—").color(muted).size(12.0));
                                    }
                                });

                                // Fuel / Lap
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("Fuel / Lap:").color(muted).size(11.5));
                                    if entry.is_player {
                                        let f_rate = entry.stint.fuel_per_lap();
                                        if f_rate > 0.0005 {
                                            ui.label(egui::RichText::new(format!("{:.3} {}", f_rate * fuel_conv, fuel_unit_str)).color(white).size(12.0).strong());
                                        } else {
                                            ui.label(egui::RichText::new("—").color(muted).size(12.0));
                                        }
                                    } else {
                                        ui.label(egui::RichText::new("— (Rival telemetry hidden)").color(muted).size(11.0));
                                    }
                                });

                                // Rolling pace buckets
                                if !self.settings.active_avg_buckets.is_empty() {
                                    ui.add_space(4.0);
                                    ui.horizontal_wrapped(|ui| {
                                        for &b in &self.settings.active_avg_buckets {
                                            let avg_val = entry.stint.avg_first_n(b);
                                            let val_str = if avg_val > 0.0 { format_time(avg_val) } else { "—".to_string() };
                                            ui.label(egui::RichText::new(format!("Avg {}:", b)).color(muted).size(11.0));
                                            ui.label(egui::RichText::new(val_str).color(white).size(11.5).strong());
                                            ui.add_space(4.0);
                                        }
                                    });
                                }
                            });
                        }
                    });
                });

            ui.add_space(10.0);

            // 3. Multi-Stint Overlay Line Chart
            ui.label(egui::RichText::new("LAP TIME PROGRESSION OVERLAY").color(muted).size(11.0).extra_letter_spacing(0.8));
            ui.add_space(4.0);
            let chart_stints: Vec<(&StintData, String, egui::Color32)> = resolved.iter().map(|e| {
                let lbl = format!("{} (#{} {})", e.slot_label, e.car_num, e.driver_name);
                (e.stint, lbl, e.color)
            }).collect();
            render_multi_stint_lap_chart(ui, &chart_stints, 185.0, white);

            ui.add_space(12.0);

            // 4. Lap-by-Lap Comparison Table
            let valid_laps_per_stint: Vec<Vec<f32>> = resolved.iter().map(|e| e.stint.valid_laps()).collect();
            let max_laps = valid_laps_per_stint.iter().map(|v| v.len()).max().unwrap_or(0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("LAP-BY-LAP COMPARISON LOG").color(muted).size(11.0).extra_letter_spacing(0.8));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("Copy Laps TSV").color(white).size(11.5)).fill(elevated)).clicked() {
                        let export_info: Vec<(&StintData, &str, &str, &str, &str, bool)> = resolved.iter().map(|e| {
                            (e.stint, e.slot_label, e.driver_name, e.car_num, e.setup_name, e.is_player)
                        }).collect();
                        let tsv = format_stints_comparison_tsv(&export_info, &self.settings.active_avg_buckets, is_metric);
                        ui.output_mut(|o| o.copied_text = tsv);
                        self.toast_message = Some(("Copied Lap-by-Lap comparison for Excel".to_string(), std::time::Instant::now()));
                    }
                });
            });
            ui.add_space(4.0);

            if max_laps > 0 {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(14, 17, 23))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)))
                    .rounding(egui::Rounding::same(10.0))
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        let mut table = TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .column(Column::initial(70.0)); // Stint Lap

                        for i in 0..resolved.len() {
                            table = table.column(Column::initial(130.0)); // Stint time
                            if i > 0 {
                                table = table.column(Column::initial(110.0)); // Delta vs Stint A
                            }
                        }
                        if resolved.len() > 1 {
                            table = table.column(Column::remainder().at_least(120.0)); // Fastest on lap
                        } else {
                            table = table.column(Column::remainder().at_least(160.0)); // Single stint info
                        }

                        let stint_best_laps: Vec<f32> = valid_laps_per_stint.iter().map(|v| {
                            v.iter().cloned().fold(f32::INFINITY, f32::min)
                        }).collect();

                        table
                            .header(26.0, |mut header| {
                                header.col(|ui| { ui.label(egui::RichText::new("Stint Lap").color(muted).size(11.5)); });
                                for (i, entry) in resolved.iter().enumerate() {
                                    header.col(|ui| {
                                        ui.label(egui::RichText::new(format!("{} (#{})", entry.slot_label, entry.car_num)).color(entry.color).strong().size(11.5));
                                    });
                                    if i > 0 {
                                        let letter = (b'A' + i as u8) as char;
                                        header.col(|ui| {
                                            ui.label(egui::RichText::new(format!("Delta ({} vs A)", letter)).color(muted).size(11.5));
                                        });
                                    }
                                }
                                if resolved.len() > 1 {
                                    header.col(|ui| { ui.label(egui::RichText::new("Fastest on Lap").color(muted).size(11.5)); });
                                } else {
                                    header.col(|ui| { ui.label(egui::RichText::new("Comparison Status").color(muted).size(11.5)); });
                                }
                            })
                            .body(|mut body| {
                                for lap_idx in 0..max_laps {
                                    let mut lap_times: Vec<Option<f32>> = Vec::new();
                                    for v in &valid_laps_per_stint {
                                        if lap_idx < v.len() {
                                            lap_times.push(Some(v[lap_idx]));
                                        } else {
                                            lap_times.push(None);
                                        }
                                    }

                                    let min_lap_in_row = lap_times.iter().filter_map(|&t| t).fold(f32::INFINITY, f32::min);

                                    body.row(26.0, |mut row| {
                                        // Stint Lap
                                        row.col(|ui| {
                                            let resp = ui.selectable_label(false, egui::RichText::new(format!("L{}", lap_idx + 1)).color(white).size(12.5));
                                            if resp.clicked() {
                                                ui.output_mut(|o| o.copied_text = format!("{}", lap_idx + 1));
                                                self.toast_message = Some((format!("Copied Lap L{}", lap_idx + 1), std::time::Instant::now()));
                                            }
                                        });

                                        // Each stint lap time and delta
                                        for (i, entry) in resolved.iter().enumerate() {
                                            row.col(|ui| {
                                                if let Some(t) = lap_times.get(i).copied().flatten() {
                                                    let is_stint_pb = stint_best_laps.get(i).map(|&b| (t - b).abs() < 0.0001).unwrap_or(false);
                                                    let is_row_fastest = (t - min_lap_in_row).abs() < 0.0001 && resolved.len() > 1;
                                                    let col = if is_stint_pb { green } else if is_row_fastest { entry.color } else { white };
                                                    let resp = ui.selectable_label(false, egui::RichText::new(format_time(t)).color(col).strong().size(12.5));
                                                    if resp.clicked() {
                                                        ui.output_mut(|o| o.copied_text = format!("{:.3}", t));
                                                        self.toast_message = Some((format!("Copied Lap Time {:.3}", t), std::time::Instant::now()));
                                                    }
                                                } else {
                                                    ui.label(egui::RichText::new("—").color(muted).size(12.0));
                                                }
                                            });

                                            if i > 0 {
                                                row.col(|ui| {
                                                    match (lap_times.get(0).copied().flatten(), lap_times.get(i).copied().flatten()) {
                                                        (Some(ta), Some(ti)) => {
                                                            let delta = ti - ta;
                                                            if delta < -0.0001 {
                                                                ui.label(egui::RichText::new(format!("{:.3}s", delta)).color(green).strong().size(12.0));
                                                            } else if delta > 0.0001 {
                                                                ui.label(egui::RichText::new(format!("+{:.3}s", delta)).color(amber).size(12.0));
                                                            } else {
                                                                ui.label(egui::RichText::new("0.000s").color(muted).size(12.0));
                                                            }
                                                        }
                                                        _ => { ui.label(egui::RichText::new("—").color(muted).size(12.0)); }
                                                    }
                                                });
                                            }
                                        }

                                        // Remainder Column
                                        if resolved.len() > 1 {
                                            row.col(|ui| {
                                                if min_lap_in_row < f32::INFINITY {
                                                    let fast_idx = lap_times.iter().position(|&ot| ot == Some(min_lap_in_row)).unwrap_or(0);
                                                    if let Some(fast_entry) = resolved.get(fast_idx) {
                                                        ui.label(egui::RichText::new(format!("★ {} ({})", fast_entry.slot_label, format_time(min_lap_in_row))).color(fast_entry.color).strong().size(11.5));
                                                    } else {
                                                        ui.label(egui::RichText::new(format_time(min_lap_in_row)).color(white).size(11.5));
                                                    }
                                                } else {
                                                    ui.label(egui::RichText::new("—").color(muted).size(12.0));
                                                }
                                            });
                                        } else {
                                            row.col(|ui| {
                                                ui.label(egui::RichText::new("Add Stint B above to compare deltas").color(muted).size(11.0));
                                            });
                                        }
                                    });
                                }
                            });
                    });
            } else {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(14, 17, 23))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("No completed green flag laps recorded yet in the selected stints.").color(muted).size(12.0));
                    });
            }

            ui.add_space(12.0);

            // 5. Post-Stint 4-Corner Tire Data Comparison (2x2 Side-by-Side: Fronts on top row, Rears on bottom row)
            ui.label(egui::RichText::new("POST-STINT TIRE WEAR & TEMPERATURES").color(muted).size(11.0).extra_letter_spacing(0.8));
            ui.add_space(6.0);

            ui.columns(resolved.len(), |tire_cols| {
                for (i, entry) in resolved.iter().enumerate() {
                    tire_cols[i].vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("●").color(entry.color).size(13.0));
                            ui.label(egui::RichText::new(format!("{} (#{} {})", entry.slot_label, entry.car_num, entry.driver_name)).color(entry.color).strong().size(12.0));
                        });
                        ui.add_space(4.0);

                        if entry.is_player {
                            if entry.stint.tires.has_data && entry.stint.laps_completed > 0 {
                                let compact_titles = ui.available_width() < 420.0;
                                let (lf_lbl, rf_lbl, lr_lbl, rr_lbl) = if compact_titles {
                                    ("LF", "RF", "LR", "RR")
                                } else {
                                    ("LF · Left Front", "RF · Right Front", "LR · Left Rear", "RR · Right Rear")
                                };
                                ui.columns(2, |corner_cols| {
                                    corner_cols[0].vertical(|ui| {
                                        render_tire_corner_card(ui, lf_lbl, &entry.stint.tires.lf, is_metric, primary, secondary, white);
                                        ui.add_space(6.0);
                                        render_tire_corner_card(ui, lr_lbl, &entry.stint.tires.lr, is_metric, primary, secondary, white);
                                    });
                                    corner_cols[1].vertical(|ui| {
                                        render_tire_corner_card(ui, rf_lbl, &entry.stint.tires.rf, is_metric, primary, secondary, white);
                                        ui.add_space(6.0);
                                        render_tire_corner_card(ui, rr_lbl, &entry.stint.tires.rr, is_metric, primary, secondary, white);
                                    });
                                });
                            } else {
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(20, 24, 32))
                                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)))
                                    .rounding(egui::Rounding::same(8.0))
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new("ℹ No Pit Service Tire Data").color(muted).strong().size(12.0));
                                        ui.add_space(4.0);
                                        ui.label(egui::RichText::new("Tires were not changed on pit road during this stint. Tire wear and temperatures are recorded when a fresh set of tires is fitted.").color(muted).size(11.0));
                                    });
                            }
                        } else {
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(20, 24, 32))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 12)))
                                .rounding(egui::Rounding::same(8.0))
                                .inner_margin(12.0)
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new("ℹ Rival Telemetry Notice").color(secondary).strong().size(12.0));
                                    ui.add_space(4.0);
                                    ui.label(egui::RichText::new("Tire wear, temps, and pressures are not broadcast over telemetry by iRacing for rival competitors.").color(muted).size(11.0));
                                    ui.add_space(4.0);
                                    ui.label(egui::RichText::new("Lap times, sector pace, and consistency are fully tracked for comparison.").color(white).size(11.0));
                                });
                        }
                    });
                }
            });

            ui.add_space(12.0);

            // 6. Driver & Engineer Notes
            ui.label(egui::RichText::new("DRIVER & ENGINEER NOTES").color(muted).size(11.0).extra_letter_spacing(0.8));
            ui.add_space(4.0);

            ui.columns(resolved.len(), |note_cols| {
                for (i, entry) in resolved.iter().enumerate() {
                    note_cols[i].vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("●").color(entry.color).size(12.0));
                            ui.label(egui::RichText::new(format!("{} Notes:", entry.slot_label)).color(entry.color).strong().size(11.5));
                        });
                        ui.add_space(2.0);

                        if entry.is_player {
                            let mut notes_buf = entry.stint.notes.clone();
                            if ui.add(
                                egui::TextEdit::multiline(&mut notes_buf)
                                    .hint_text("Setup changes, tire pressures, or run notes for this stint...")
                                    .desired_rows(3)
                                    .desired_width(ui.available_width())
                            ).changed() {
                                updated_notes = Some((entry.car_idx, entry.stint.stint_number, notes_buf));
                            }
                        } else {
                            let notes_text = if entry.stint.notes.is_empty() {
                                "No notes recorded for competitor stint."
                            } else {
                                &entry.stint.notes
                            };
                            ui.label(egui::RichText::new(notes_text).color(muted).size(11.5));
                        }
                    });
                }
            });
        });

        // Apply mutations
        if let Some(idx) = remove_slot_idx {
            if self.stint_compare_slots.len() > 1 && idx < self.stint_compare_slots.len() {
                self.stint_compare_slots.remove(idx);
            }
        }

        if add_third_stint && self.stint_compare_slots.len() < 3 {
            let mut found = None;
            // First try same driver as slot 0
            if let Some(first) = self.stint_compare_slots.first() {
                if let Some(drv) = s.drivers.get(&first.car_idx) {
                    for st in &drv.stints {
                        if !self.stint_compare_slots.iter().any(|slot| slot.car_idx == first.car_idx && slot.stint_number == st.stint_number) {
                            found = Some(StintCompareSlot { car_idx: first.car_idx, stint_number: st.stint_number });
                            break;
                        }
                    }
                }
            }
            // Next try any driver
            if found.is_none() {
                for (&car_idx, drv) in &s.drivers {
                    for st in &drv.stints {
                        if !self.stint_compare_slots.iter().any(|slot| slot.car_idx == car_idx && slot.stint_number == st.stint_number) {
                            found = Some(StintCompareSlot { car_idx, stint_number: st.stint_number });
                            break;
                        }
                    }
                    if found.is_some() { break; }
                }
            }
            if let Some(slot) = found {
                self.stint_compare_slots.push(slot);
            } else {
                self.toast_message = Some(("No other stints recorded in this session yet".to_string(), std::time::Instant::now()));
            }
        }

        if let Some((slot_idx, new_car_idx)) = new_driver_for_slot {
            if slot_idx < self.stint_compare_slots.len() {
                if let Some(drv) = s.drivers.get(&new_car_idx) {
                    let stint_num = drv.stints.last().map(|s| s.stint_number).unwrap_or(1);
                    self.stint_compare_slots[slot_idx] = StintCompareSlot {
                        car_idx: new_car_idx,
                        stint_number: stint_num,
                    };
                }
            }
        }

        if let Some((slot_idx, new_st_num)) = new_stint_for_slot {
            if slot_idx < self.stint_compare_slots.len() {
                self.stint_compare_slots[slot_idx].stint_number = new_st_num;
            }
        }

        if let Some((car_idx, s_num, new_notes)) = updated_notes {
            let mut saved_id_to_update = String::new();
            if let Ok(mut state_guard) = self.state.lock() {
                if let Some(drv) = state_guard.drivers.get_mut(&car_idx) {
                    if let Some(st) = drv.stints.iter_mut().find(|st| st.stint_number == s_num) {
                        st.notes = new_notes.clone();
                        saved_id_to_update = st.saved_id.clone();
                    }
                }
            }
            if !saved_id_to_update.is_empty() {
                if let Some(rec) = self.stint_db.records.iter_mut().find(|r| r.id == saved_id_to_update) {
                    rec.stint.notes = new_notes;
                    self.stint_db.save();
                }
            }
        }
    }

    pub(crate) fn render_stint_comparison_window(&mut self, ctx: &egui::Context, s: &WarRoomState) {
        if !self.show_stint_comparison_window { return; }
        self.ensure_compare_slots_populated(s);

        let screen_rect = ctx.screen_rect();
        let win_w = (screen_rect.width() * 0.90).clamp(980.0, 1420.0);
        let win_h = (screen_rect.height() * 0.88).clamp(580.0, 920.0);
        let elevated = self.settings.card_elevated_color();
        let white = self.settings.text_color();

        let mut is_open = self.show_stint_comparison_window;
        let mut close_modal = false;

        egui::Window::new("⚖ Side-by-Side Stint Comparison · Up to 3 Stints")
            .id(egui::Id::new("stint_comparison_dedicated_window"))
            .open(&mut is_open)
            .default_size([win_w, win_h])
            .default_pos([screen_rect.min.x + 50.0, screen_rect.min.y + 50.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_modal = true;
                }

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("COMPARE UP TO 3 STINTS SIDE-BY-SIDE").color(self.settings.text_muted()).size(11.0).extra_letter_spacing(0.8));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized(egui::vec2(72.0, 26.0), egui::Button::new(egui::RichText::new("× Close").color(white).size(12.0)).fill(elevated)).clicked() {
                            close_modal = true;
                        }
                    });
                });

                ui.add_space(6.0);

                ui.push_id("popout_stint_comparison", |ui| {
                    self.render_stint_comparison_content(ui, s);
                });
            });

        if !is_open || close_modal {
            self.show_stint_comparison_window = false;
        }
    }

    pub(crate) fn render_stint_analyzer_modal(&mut self, ctx: &egui::Context, s: &WarRoomState) {
        if !self.show_stint_analyzer_modal { return; }

        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let green = self.settings.status_green();
        let is_metric = self.settings.use_metric;
        let fuel_conv = if is_metric { 1.0 } else { 0.264172 };
        let fuel_unit_str = if is_metric { "L" } else { "gal" };

        let is_compare = self.stint_analyzer_tab == StintAnalyzerTab::Compare;
        let any_driver_has_stints = s.drivers.values().any(|d| !d.stints.is_empty());

        let target_car_idx = self.selected_driver_car_idx.unwrap_or(s.player_car_idx);
        let target_driver = s.drivers.get(&target_car_idx).or_else(|| s.drivers.get(&s.player_car_idx));
        let (drv_ref, player_stints) = match target_driver {
            Some(d) if !d.stints.is_empty() => (d, &d.stints),
            _ => {
                if is_compare && any_driver_has_stints {
                    let fallback_drv = s.drivers.values().find(|d| !d.stints.is_empty()).unwrap();
                    (fallback_drv, &fallback_drv.stints)
                } else {
                    egui::Window::new("Stint Telemetry Analyzer")
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .collapsible(false)
                        .show(ctx, |ui| {
                            ui.label(egui::RichText::new("No stints recorded yet for this driver. Complete laps on track or re-import a saved stint to view telemetry.").color(muted));
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.add(
                                    egui::Button::new(egui::RichText::new("📂 Open Saved Stints / Re-Import").color(egui::Color32::BLACK).strong())
                                        .fill(primary)
                                ).clicked() {
                                    self.show_stint_analyzer_modal = false;
                                    self.show_stint_db_modal = true;
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("Close").color(white)).fill(elevated)).clicked() {
                                    self.show_stint_analyzer_modal = false;
                                }
                            });
                        });
                    return;
                }
            }
        };

        let selected_num = self.selected_stint_num.unwrap_or_else(|| {
            player_stints.last().map(|st| st.stint_number).unwrap_or(1)
        });

        let stint = player_stints.iter().find(|st| st.stint_number == selected_num).unwrap_or_else(|| player_stints.last().unwrap());
        let single_setup_display = if !stint.setup_name.is_empty() && stint.setup_name != "—" && stint.setup_name != "Baseline" {
            stint.setup_name.as_str()
        } else if !drv_ref.setup_name.is_empty() && drv_ref.setup_name != "—" && drv_ref.setup_name != "Baseline" {
            drv_ref.setup_name.as_str()
        } else if target_car_idx == s.player_car_idx && !s.player_setup_name.is_empty() && s.player_setup_name != "Baseline" {
            s.player_setup_name.as_str()
        } else {
            "Baseline"
        };

        let screen_rect = ctx.screen_rect();
        let win_w = if is_compare {
            (screen_rect.width() * 0.90).clamp(980.0, 1420.0)
        } else {
            (screen_rect.width() * 0.75).clamp(720.0, 1050.0)
        };
        let win_h = if is_compare {
            (screen_rect.height() * 0.88).clamp(580.0, 920.0)
        } else {
            (screen_rect.height() * 0.82).clamp(520.0, 850.0)
        };

        let mut is_open = self.show_stint_analyzer_modal;
        let mut close_modal = false;
        let mut updated_notes: Option<(i32, String)> = None;

        let window_title = if is_compare {
            "⚖ Stint Comparison (Side-by-Side) · Up to 3 Stints".to_string()
        } else {
            format!("Stint {} · #{} {} · Telemetry & Lap Log", stint.short_label(), drv_ref.car_num, drv_ref.driver_name)
        };

        egui::Window::new(window_title)
            .id(egui::Id::new("stint_analyzer_modal_window"))
            .open(&mut is_open)
            .default_size([win_w, win_h])
            .default_pos([screen_rect.min.x + 30.0, screen_rect.min.y + 30.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_modal = true;
                }

                // Top Header Row with Segmented Mode Switcher + Stint tabs + Popout + Close
                ui.horizontal(|ui| {
                    let single_btn = egui::Button::new(
                        egui::RichText::new("🔍 Single Stint")
                            .color(if !is_compare { egui::Color32::WHITE } else { muted })
                            .size(12.0)
                            .strong()
                    )
                    .fill(if !is_compare { with_alpha(primary, 190) } else { elevated })
                    .rounding(egui::Rounding::same(6.0));
                    if ui.add(single_btn).clicked() {
                        self.stint_analyzer_tab = StintAnalyzerTab::SingleStint;
                    }

                    let num_slots = self.stint_compare_slots.len().clamp(1, 3);
                    let compare_btn = egui::Button::new(
                        egui::RichText::new(format!("⚖ Compare Stints ({})", num_slots))
                            .color(if is_compare { egui::Color32::WHITE } else { muted })
                            .size(12.0)
                            .strong()
                    )
                    .fill(if is_compare { with_alpha(secondary, 190) } else { elevated })
                    .rounding(egui::Rounding::same(6.0));
                    if ui.add(compare_btn).clicked() {
                        self.ensure_compare_slots_populated(s);
                        self.stint_analyzer_tab = StintAnalyzerTab::Compare;
                    }

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if !is_compare {
                        ui.label(egui::RichText::new("STINT").color(muted).size(11.0).extra_letter_spacing(0.8));
                        for st in player_stints.iter() {
                            let is_sel = st.stint_number == selected_num;
                            let tab_title = if st.is_active {
                                format!("{} · Live ({})", st.short_label(), st.laps_completed)
                            } else {
                                format!("{} ({})", st.short_label(), st.laps_completed)
                            };

                            let btn = egui::Button::new(
                                egui::RichText::new(tab_title)
                                    .color(if is_sel { egui::Color32::WHITE } else { muted })
                                    .size(12.0)
                            )
                            .fill(if is_sel { with_alpha(secondary, 190) } else { elevated })
                            .rounding(egui::Rounding::same(6.0));

                            if ui.add(btn).clicked() {
                                self.selected_stint_num = Some(st.stint_number);
                            }
                        }
                    }

                    let single_setup_display = if !stint.setup_name.is_empty() && stint.setup_name != "—" && stint.setup_name != "Baseline" {
                        stint.setup_name.as_str()
                    } else if !drv_ref.setup_name.is_empty() && drv_ref.setup_name != "—" && drv_ref.setup_name != "Baseline" {
                        drv_ref.setup_name.as_str()
                    } else if target_car_idx == s.player_car_idx && !s.player_setup_name.is_empty() && s.player_setup_name != "Baseline" {
                        s.player_setup_name.as_str()
                    } else {
                        "Baseline"
                    };

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized(egui::vec2(72.0, 26.0), egui::Button::new(egui::RichText::new("× Close").color(white).size(12.0)).fill(elevated)).clicked() {
                            close_modal = true;
                        }

                        if ui.add_sized(egui::vec2(95.0, 26.0), egui::Button::new(egui::RichText::new("↗ Pop Out").color(white).size(12.0)).fill(elevated)).on_hover_text("Open Stint Comparison in a separate floating window").clicked() {
                            self.ensure_compare_slots_populated(s);
                            self.show_stint_comparison_window = true;
                            self.show_stint_analyzer_modal = false;
                        }

                        if ui.add_sized(egui::vec2(108.0, 26.0), egui::Button::new(egui::RichText::new("📂 Saved Stints").color(white).size(11.5)).fill(elevated)).on_hover_text("Open Saved Stints Database to re-import or share stints").clicked() {
                            self.show_stint_db_modal = true;
                        }

                        if !is_compare {
                            if ui.add_sized(
                                egui::vec2(125.0, 26.0),
                                egui::Button::new(egui::RichText::new("📎 Share Stint File").color(white).size(11.5)).fill(elevated)
                            ).on_hover_text("Save this stint to your database and copy the physical .s4s.json file to clipboard (press Ctrl+V in Discord to attach)").clicked() {
                                let record = stint_db::build_record_from_state(s, target_car_idx, stint);
                                self.stint_db.upsert_stint(record.clone());
                                let file_path = self.stint_db.record_file_path(&record);
                                match stint_db::copy_stint_file_to_clipboard(&file_path) {
                                    Ok(()) => {
                                        self.toast_message = Some((
                                            format!("📎 Saved & copied Stint {} (.s4s.json) to clipboard — press Ctrl+V in Discord to attach!", stint.short_label()),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                    Err(_) => {
                                        if let Ok(code) = stint_db::export_share_string(&record) {
                                            ui.output_mut(|o| o.copied_text = code);
                                        }
                                        self.toast_message = Some((
                                            format!("💾 Saved Stint {} to local Stint Database!", stint.short_label()),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                }
                            }

                            if ui.add_sized(
                                egui::vec2(148.0, 26.0),
                                egui::Button::new(egui::RichText::new("📸 Copy Discord Image").color(egui::Color32::BLACK).size(11.5).strong()).fill(primary)
                            ).on_hover_text("Generate a Stint Summary graphic card and copy to clipboard for pasting (Ctrl+V) into Discord").clicked() {
                                let single_entry = [discord_card::DiscordStintEntry {
                                    slot_label: None,
                                    driver_name: &drv_ref.driver_name,
                                    car_num: &drv_ref.car_num,
                                    setup_name: single_setup_display,
                                    is_player: target_car_idx == s.player_car_idx,
                                    stint,
                                    accent_rgb: [primary.r(), primary.g(), primary.b()],
                                }];
                                match discord_card::render_discord_stint_card(&single_entry, s, &self.settings)
                                    .and_then(|img| discord_card::copy_rgba_image_to_clipboard(&img))
                                {
                                    Ok(()) => {
                                        self.toast_message = Some((format!("📸 Copied Stint S{} image — press Ctrl+V in Discord!", stint.stint_number), std::time::Instant::now()));
                                    }
                                    Err(err) => {
                                        self.toast_message = Some((format!("⚠ Image copy failed: {}", err), std::time::Instant::now()));
                                    }
                                }
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                if self.stint_analyzer_tab == StintAnalyzerTab::Compare {
                    ui.push_id("modal_stint_comparison", |ui| {
                        self.render_stint_comparison_content(ui, s);
                    });
                } else {

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    // Summary Metrics Bar
                    egui::Frame::none()
                        .fill(elevated)
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.columns(5, |cols| {
                                cols[0].vertical(|ui| {
                                    ui.label(egui::RichText::new("STINT LAPS").color(muted).size(10.5).extra_letter_spacing(0.6));
                                    ui.label(egui::RichText::new(format!("{}", stint.laps_completed)).font(display_font(20.0)).color(white));
                                });
                                cols[1].vertical(|ui| {
                                    ui.label(egui::RichText::new("FASTEST LAP").color(muted).size(10.5).extra_letter_spacing(0.6));
                                    ui.label(egui::RichText::new(format_time(stint.fast_lap)).font(display_font(20.0)).color(green));
                                });
                                cols[2].vertical(|ui| {
                                    ui.label(egui::RichText::new("AVERAGE LAP").color(muted).size(10.5).extra_letter_spacing(0.6));
                                    ui.label(egui::RichText::new(format_time(stint.full_average())).font(display_font(20.0)).color(white));
                                });
                                cols[3].vertical(|ui| {
                                    ui.label(egui::RichText::new("FUEL / LAP").color(muted).size(10.5).extra_letter_spacing(0.6));
                                    let f_rate = stint.fuel_per_lap();
                                    let fuel_str = if f_rate > 0.0005 {
                                        format!("{:.3} {}", f_rate * fuel_conv, fuel_unit_str)
                                    } else {
                                        "—".to_string()
                                    };
                                    ui.label(egui::RichText::new(fuel_str).font(display_font(20.0)).color(white));
                                });
                                cols[4].vertical(|ui| {
                                    ui.label(egui::RichText::new("CONSISTENCY σ").color(muted).size(10.5).extra_letter_spacing(0.6));
                                    ui.label(egui::RichText::new(format!("{:.3}", stint.std_dev())).font(display_font(20.0)).color(primary));
                                });
                            });
                        });

                    ui.add_space(8.0);

                    // Key Setup & Conditions Bar
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Setup").color(muted).size(12.0));
                        let setup_display = if !stint.setup_name.is_empty() && stint.setup_name != "—" && stint.setup_name != "Baseline" {
                            &stint.setup_name
                        } else if !drv_ref.setup_name.is_empty() && drv_ref.setup_name != "—" && drv_ref.setup_name != "Baseline" {
                            &drv_ref.setup_name
                        } else if target_car_idx == s.player_car_idx && !s.player_setup_name.is_empty() && s.player_setup_name != "Baseline" {
                            &s.player_setup_name
                        } else {
                            "Baseline"
                        };
                        ui.label(egui::RichText::new(setup_display).color(white).strong().size(12.5));
                        
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Track").color(muted).size(12.0));
                        let track_raw = if !stint.track_temp_at_start.is_empty() && stint.track_temp_at_start != "—" {
                            &stint.track_temp_at_start
                        } else {
                            &s.track_temp_raw
                        };
                        ui.label(egui::RichText::new(convert_unit(track_raw, is_metric)).color(white).size(12.5));

                        ui.add_space(16.0);
                        ui.label(egui::RichText::new("Air").color(muted).size(12.0));
                        let air_raw = if !stint.air_temp_at_start.is_empty() && stint.air_temp_at_start != "—" {
                            &stint.air_temp_at_start
                        } else {
                            &s.air_temp_raw
                        };
                        ui.label(egui::RichText::new(convert_unit(air_raw, is_metric)).color(white).size(12.5));
                    });

                    // Rolling Averages Breakdown
                    if !self.settings.active_avg_buckets.is_empty() {
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new("Rolling pace:").color(muted).size(12.0));
                            for &b in &self.settings.active_avg_buckets {
                                let avg_val = stint.avg_first_n(b);
                                let val_str = if avg_val > 0.0 { format_time(avg_val) } else { "—".to_string() };
                                ui.label(egui::RichText::new(format!("Avg {}:", b)).color(muted).size(11.5));
                                ui.label(egui::RichText::new(val_str).color(white).size(12.0).strong());
                                ui.add_space(8.0);
                            }
                        });
                    }

                    ui.add_space(10.0);

                    // Line Graph
                    ui.label(egui::RichText::new("LAP TIME PROGRESSION").color(muted).size(11.0).extra_letter_spacing(0.8));
                    ui.add_space(4.0);
                    render_stint_lap_chart(ui, stint, 160.0, primary, secondary, white);

                    ui.add_space(12.0);

                    // --- SECTION: LAP-BY-LAP SPREADSHEET TABLE ---
                    let valid_laps = stint.valid_laps();
                    let numbered_laps = stint.numbered_valid_laps();
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("LAP-BY-LAP LOG").color(muted).size(11.0).extra_letter_spacing(0.8));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("Copy Spreadsheet (TSV)").color(egui::Color32::BLACK).size(11.5).strong()).fill(primary)).clicked() {
                                let tsv = format_stint_laps_spreadsheet_tsv(stint);
                                ui.output_mut(|o| o.copied_text = tsv);
                                self.toast_message = Some((format!("Copied {} laps spreadsheet for Excel", valid_laps.len()), std::time::Instant::now()));
                            }
                            if ui.add(egui::Button::new(egui::RichText::new("Copy Lap Times").color(white).size(11.5)).fill(elevated)).clicked() {
                                let tsv = format_laps_column_tsv(stint);
                                ui.output_mut(|o| o.copied_text = tsv);
                                self.toast_message = Some((format!("Copied {} lap times for Excel", valid_laps.len()), std::time::Instant::now()));
                            }
                            if ui.add(egui::Button::new(egui::RichText::new("⚖ Add to Compare").color(white).size(11.5)).fill(elevated)).on_hover_text("Add this stint into the Side-by-Side Comparison").clicked() {
                                let slot_entry = StintCompareSlot { car_idx: target_car_idx, stint_number: stint.stint_number };
                                if !self.stint_compare_slots.contains(&slot_entry) {
                                    if self.stint_compare_slots.len() >= 3 {
                                        self.stint_compare_slots.pop();
                                    }
                                    self.stint_compare_slots.push(slot_entry);
                                }
                                self.ensure_compare_slots_populated(s);
                                self.stint_analyzer_tab = StintAnalyzerTab::Compare;
                                self.toast_message = Some((format!("Added S{} to Stint Comparison", stint.stint_number), std::time::Instant::now()));
                            }
                        });
                    });
                    ui.add_space(4.0);

                    if !valid_laps.is_empty() {
                        let mut sum_lap_times = 0.0;

                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(14, 17, 23))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10)))
                            .rounding(egui::Rounding::same(10.0))
                            .inner_margin(10.0)
                            .show(ui, |ui| {
                                TableBuilder::new(ui)
                                    .striped(true)
                                    .resizable(true)
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .column(Column::initial(95.0))  // Stint Lap
                                    .column(Column::initial(90.0))  // Lap
                                    .column(Column::initial(130.0)) // Lap Time
                                    .column(Column::initial(140.0)) // Avg Lap Time
                                    .column(Column::remainder().at_least(90.0)) // Action
                                    .header(26.0, |mut header| {
                                        header.col(|ui| { ui.label(egui::RichText::new("Stint Lap").color(muted).size(11.5)); });
                                        header.col(|ui| { ui.label(egui::RichText::new("Session Lap").color(muted).size(11.5)); });
                                        header.col(|ui| { ui.label(egui::RichText::new("Lap Time").color(muted).size(11.5)); });
                                        header.col(|ui| { ui.label(egui::RichText::new("Running Avg").color(muted).size(11.5)); });
                                        header.col(|ui| { ui.label(egui::RichText::new("Action").color(muted).size(11.5)); });
                                    })
                                    .body(|mut body| {
                                        let min_t = valid_laps.iter().cloned().fold(f32::INFINITY, f32::min);
                                        for (i, &(overall_lap, t)) in numbered_laps.iter().enumerate() {
                                            sum_lap_times += t;
                                            let cum_avg = sum_lap_times / (i + 1) as f32;
                                            let is_fast = (t - min_t).abs() < 0.0001;

                                            body.row(26.0, |mut row| {
                                                row.col(|ui| {
                                                    let resp = ui.selectable_label(false, egui::RichText::new(format!("L{}", i + 1)).color(white).size(12.5));
                                                    if resp.clicked() {
                                                        ui.output_mut(|o| o.copied_text = format!("{}", i + 1));
                                                        self.toast_message = Some((format!("Copied Stint Lap {}", i + 1), std::time::Instant::now()));
                                                    }
                                                });

                                                row.col(|ui| {
                                                    let resp = ui.selectable_label(false, egui::RichText::new(format!("{}", overall_lap)).color(muted).size(12.5));
                                                    if resp.clicked() {
                                                        ui.output_mut(|o| o.copied_text = format!("{}", overall_lap));
                                                        self.toast_message = Some((format!("Copied Lap {}", overall_lap), std::time::Instant::now()));
                                                    }
                                                });

                                                row.col(|ui| {
                                                    let time_text = format_time(t);
                                                    let col = if is_fast { green } else { white };
                                                    let resp = ui.selectable_label(false, egui::RichText::new(time_text).color(col).strong().size(12.5));
                                                    if resp.clicked() {
                                                        ui.output_mut(|o| o.copied_text = format!("{:.3}", t));
                                                        self.toast_message = Some((format!("Copied Lap Time {:.3}", t), std::time::Instant::now()));
                                                    }
                                                });

                                                row.col(|ui| {
                                                    let avg_text = format_time(cum_avg);
                                                    let resp = ui.selectable_label(false, egui::RichText::new(avg_text).color(muted).size(12.5));
                                                    if resp.clicked() {
                                                        ui.output_mut(|o| o.copied_text = format!("{:.3}", cum_avg));
                                                        self.toast_message = Some((format!("Copied Avg Lap Time {:.3}", cum_avg), std::time::Instant::now()));
                                                    }
                                                });

                                                row.col(|ui| {
                                                    if ui.add(egui::Button::new(egui::RichText::new("Copy row").color(muted).size(11.0)).fill(elevated)).clicked() {
                                                        let row_tsv = format!("{}\t{}\t{:.3}\t{:.3}", i + 1, overall_lap, t, cum_avg);
                                                        ui.output_mut(|o| o.copied_text = row_tsv);
                                                        self.toast_message = Some((format!("Copied Lap {} row for Excel", i + 1), std::time::Instant::now()));
                                                    }
                                                });
                                            });
                                        }
                                    });
                            });
                    }

                    ui.add_space(12.0);

                    // --- SECTION: 4-CORNER POST-STINT TIRE DATA ---
                    let empty_tires = StintTireSet::default();
                    let stint_has_tires = stint.tires.has_data && stint.laps_completed > 0;
                    let stint_tires = if stint_has_tires {
                        &stint.tires
                    } else {
                        &empty_tires
                    };

                    let header_lbl = if stint_has_tires {
                        "POST-STINT TIRES & EXPORT"
                    } else {
                        "POST-STINT TIRES · NO PIT SERVICE"
                    };

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(header_lbl).color(muted).size(11.0).extra_letter_spacing(0.8));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(
                                egui::Button::new(egui::RichText::new("📸 Copy Discord Image").color(egui::Color32::BLACK).size(11.0).strong()).fill(primary)
                            ).on_hover_text("Copy Stint S{} summary graphic card to clipboard for Discord (Ctrl+V)").clicked() {
                                let single_entry = [discord_card::DiscordStintEntry {
                                    slot_label: None,
                                    driver_name: &drv_ref.driver_name,
                                    car_num: &drv_ref.car_num,
                                    setup_name: single_setup_display,
                                    is_player: target_car_idx == s.player_car_idx,
                                    stint,
                                    accent_rgb: [primary.r(), primary.g(), primary.b()],
                                }];
                                match discord_card::render_discord_stint_card(&single_entry, s, &self.settings)
                                    .and_then(|img| discord_card::copy_rgba_image_to_clipboard(&img))
                                {
                                    Ok(()) => {
                                        self.toast_message = Some((format!("📸 Copied Stint S{} image — press Ctrl+V in Discord!", stint.stint_number), std::time::Instant::now()));
                                    }
                                    Err(err) => {
                                        self.toast_message = Some((format!("⚠ Image copy failed: {}", err), std::time::Instant::now()));
                                    }
                                }
                            }
                            if ui.add(egui::Button::new(egui::RichText::new("Copy Full Stint").color(white).size(11.0)).fill(elevated)).clicked() {
                                let tsv = format_full_stint_excel_tsv(stint, &self.settings.active_avg_buckets, is_metric);
                                ui.output_mut(|o| o.copied_text = tsv);
                                self.toast_message = Some((format!("Copied Stint S{} Full Log for Excel", stint.stint_number), std::time::Instant::now()));
                            }
                            if stint_has_tires {
                                if ui.add(egui::Button::new(egui::RichText::new("Copy 4-Tire Table").color(white).size(11.0)).fill(elevated)).clicked() {
                                    let tsv = format_tires_excel_tsv(stint_tires, is_metric);
                                    ui.output_mut(|o| o.copied_text = tsv);
                                    self.toast_message = Some(("Copied 4-Corner Tires Table for Excel".to_string(), std::time::Instant::now()));
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("Copy Wear").color(white).size(11.0)).fill(elevated)).clicked() {
                                    let tsv = format_wear_excel_tsv(stint_tires);
                                    ui.output_mut(|o| o.copied_text = tsv);
                                    self.toast_message = Some(("Copied 4-Corner Wear for Excel".to_string(), std::time::Instant::now()));
                                }
                                if ui.add(egui::Button::new(egui::RichText::new("Copy Temps").color(white).size(11.0)).fill(elevated)).clicked() {
                                    let tsv = format_temps_excel_tsv(stint_tires, is_metric);
                                    ui.output_mut(|o| o.copied_text = tsv);
                                    self.toast_message = Some(("Copied 4-Corner Temps for Excel".to_string(), std::time::Instant::now()));
                                }
                            }
                        });
                    });
                    ui.add_space(6.0);

                    // 4 Visual Tire Cards
                    ui.columns(2, |tire_cols| {
                        tire_cols[0].vertical(|ui| {
                            render_tire_corner_card(ui, "LF · Left Front", &stint_tires.lf, is_metric, primary, secondary, white);
                            ui.add_space(6.0);
                            render_tire_corner_card(ui, "LR · Left Rear", &stint_tires.lr, is_metric, primary, secondary, white);
                        });
                        tire_cols[1].vertical(|ui| {
                            render_tire_corner_card(ui, "RF · Right Front", &stint_tires.rf, is_metric, primary, secondary, white);
                            ui.add_space(6.0);
                            render_tire_corner_card(ui, "RR · Right Rear", &stint_tires.rr, is_metric, primary, secondary, white);
                        });
                    });

                    ui.add_space(12.0);

                    // Notes Section
                    ui.label(egui::RichText::new("DRIVER & ENGINEER NOTES").color(muted).size(11.0).extra_letter_spacing(0.8));
                    ui.add_space(4.0);
                    let mut notes_buf = stint.notes.clone();
                    if ui.add(
                        egui::TextEdit::multiline(&mut notes_buf)
                            .hint_text("Type setup adjustments, driver feedback, tire pressure changes, or run notes for this stint...")
                            .desired_rows(3)
                            .desired_width(ui.available_width())
                    ).changed() {
                        updated_notes = Some((stint.stint_number, notes_buf));
                    }
                });
            }
        });

        if !is_compare {
            if let Some((s_num, new_notes)) = updated_notes {
                let mut saved_id_to_update = String::new();
                if let Ok(mut state_guard) = self.state.lock() {
                    if let Some(drv) = state_guard.drivers.get_mut(&target_car_idx) {
                        if let Some(st) = drv.stints.iter_mut().find(|st| st.stint_number == s_num) {
                            st.notes = new_notes.clone();
                            saved_id_to_update = st.saved_id.clone();
                        }
                    }
                }
                if !saved_id_to_update.is_empty() {
                    if let Some(rec) = self.stint_db.records.iter_mut().find(|r| r.id == saved_id_to_update) {
                        rec.stint.notes = new_notes;
                        self.stint_db.save();
                    }
                }
            }
        }

        if !is_open || close_modal {
            self.show_stint_analyzer_modal = false;
        }
    }
}
