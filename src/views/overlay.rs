use eframe::egui;
use crate::{display_font, with_alpha, WarRoomApp};
use crate::formatters::*;
use crate::models::*;
use crate::settings::*;

#[cfg(target_os = "windows")]
pub(crate) fn ensure_overlay_hwnd_transparency(window_title: &str, tracked: &mut (isize, u8)) {
    #[repr(C, packed(1))]
    struct DwmBlurBehind {
        dw_flags: u32,
        f_enable: i32,
        h_rgn_blur: isize,
        f_transition_on_maximized: i32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, lpdw_process_id: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: isize) -> i32;
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRectRgn(x1: i32, y1: i32, x2: i32, y2: i32) -> isize;
        fn DeleteObject(ho: isize) -> i32;
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmEnableBlurBehindWindow(hwnd: isize, p_blur_behind: *const DwmBlurBehind) -> i32;
        fn DwmSetWindowAttribute(
            hwnd: isize,
            dw_attribute: u32,
            pv_attribute: *const std::ffi::c_void,
            cb_attribute: u32,
        ) -> i32;
    }

    let mut wide_title: Vec<u16> = window_title.encode_utf16().collect();
    wide_title.push(0);

    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), wide_title.as_ptr());
        if hwnd == 0 {
            return;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid != std::process::id() {
            return;
        }

        if tracked.0 != hwnd {
            *tracked = (hwnd, 0);
        }
        if tracked.1 >= 10 {
            return;
        }
        if IsWindowVisible(hwnd) != 0 {
            tracked.1 += 1;
        }

        // Empty region for the blur effect so DWM composites the OpenGL alpha channel directly
        let rgn = CreateRectRgn(0, 0, -1, -1);
        let bb = DwmBlurBehind {
            dw_flags: 0x0000_0001 | 0x0000_0002, // DWM_BB_ENABLE | DWM_BB_BLURREGION
            f_enable: 1,
            h_rgn_blur: rgn,
            f_transition_on_maximized: 0,
        };
        let _ = DwmEnableBlurBehindWindow(hwnd, &bb);
        if rgn != 0 {
            let _ = DeleteObject(rgn);
        }

        // Suppress Windows 11 system border and outer rounding on frameless transparent overlay HWNDs
        let color_none: u32 = 0xFFFF_FFFE; // DWMWA_COLOR_NONE
        let _ = DwmSetWindowAttribute(
            hwnd,
            34, // DWMWA_BORDER_COLOR
            &color_none as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
        let corner_donotround: u32 = 1; // DWMWCP_DONOTROUND
        let _ = DwmSetWindowAttribute(
            hwnd,
            33, // DWMWA_WINDOW_CORNER_PREFERENCE
            &corner_donotround as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn ensure_overlay_hwnd_transparency(_window_title: &str, _tracked: &mut (isize, u8)) {}

impl WarRoomApp {
    pub(crate) fn render_floating_overlay_hud(&mut self, ctx: &egui::Context, state: &WarRoomState) {
        if !self.settings.overlay.enabled && !self.preview_overlay_hud {
            self.main_overlay_hwnd = (0, 0);
            return;
        }

        let is_preview = (!state.is_connected || !state.player_is_in_car) && self.preview_overlay_hud;
        if (!state.is_connected || !state.player_is_in_car) && !is_preview {
            self.main_overlay_hwnd = (0, 0);
            return;
        }

        let primary = self.settings.primary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let green = self.settings.status_green();
        let amber = self.settings.status_amber();
        let red = self.settings.status_red();

        let is_metric = self.settings.use_metric;
        let fuel_conv = if is_metric { 1.0 } else { 0.264172 };
        let fuel_unit_str = if is_metric { "L" } else { "gal" };

        let opacity = self.settings.overlay.opacity.clamp(0.05, 1.0);
        let bg_opacity = self.settings.overlay.bg_opacity.clamp(0.0, 1.0);
        let always_on_top = self.settings.overlay.always_on_top;
        let safety_margin = self.settings.overlay.safety_margin_laps;

        let scale = self.settings.overlay.hud_scale.clamp(0.75, 1.50);
        let show_pace = self.settings.overlay.show_pace_card;
        let show_fuel = self.settings.overlay.show_fuel_card;
        let show_rc = self.settings.overlay.show_race_control_card;
        let show_weather = self.settings.overlay.show_weather;

        // If user unchecked everything, default back to showing all cards so window isn't empty
        let (show_pace, show_fuel, show_rc, show_weather) = if !show_pace && !show_fuel && !show_rc && !show_weather {
            (true, true, true, true)
        } else {
            (show_pace, show_fuel, show_rc, show_weather)
        };

        let (stint_num, stint_lap_count, stint_avg, last_lap, fast_lap, comp_header, comp_last_lap, comp_sub, delta_opt, fuel_level_ltr, laps_of_fuel, pit_target_lap, laps_remain, player_lap, fuel_burn_rate_ltr, fuel_to_add_display, has_fuel_calc, inc_str, inc_color, caution_str, caution_color, flag_text, flag_color, player_pos) = if is_preview {
            let sample_tank_ltr = 12.4 / fuel_conv;
            let sample_burn_ltr = 0.825 / fuel_conv;
            (
                1, 8, 92.450, 92.120, 91.850,
                "LEADER LAST", 92.350, "P1 · #48".to_string(), Some(-0.230),
                sample_tank_ltr, 14.8, 22, 18, 7, sample_burn_ltr,
                14.5, true,
                "Incidents: 2 / 17x".to_string(), white,
                "Cautions: 1 (3 laps)".to_string(), muted,
                "● Green", green,
                2,
            )
        } else {
            let player = state.drivers.get(&state.player_car_idx);
            let active_stint = player.and_then(|p| p.stints.iter().rev().find(|s| s.is_active).or_else(|| p.stints.last()));

            let stint_num = active_stint.map_or(1, |s| s.stint_number);
            let stint_lap_count = active_stint.map_or(0, |s| s.laps_completed);
            let stint_avg = active_stint.map_or(0.0, |s| s.full_average());
            let last_lap = active_stint.map_or(0.0, |s| s.last_lap);
            let fast_lap = active_stint.map_or(0.0, |s| s.fast_lap);

            let player_pos = player.map_or(99, |p| p.position);
            let (comp_driver, comp_header, comp_label) = if player_pos == 1 {
                (state.drivers.values().find(|d| d.position == 2), "P2 LAST", "P2")
            } else {
                (state.drivers.values().find(|d| d.position == 1), "LEADER LAST", "P1")
            };

            let comp_last_lap = comp_driver.map_or(0.0, |d| {
                if d.last_known_lap_time > 0.0 {
                    d.last_known_lap_time
                } else {
                    d.stints.iter().rev().find_map(|s| if s.last_lap > 0.0 { Some(s.last_lap) } else { None }).unwrap_or(0.0)
                }
            });

            let comp_sub = comp_driver.map_or_else(
                || "—".to_string(),
                |d| format!("{} · #{}", comp_label, d.car_num)
            );

            let delta_opt = if last_lap > 3.0 && comp_last_lap > 3.0 {
                Some(last_lap - comp_last_lap)
            } else {
                None
            };

            let fuel_level_ltr = state.player_fuel_level;
            let fuel_burn_rate_ltr = active_stint
                .map(|s| s.fuel_per_lap())
                .filter(|&r| r > 0.001)
                .unwrap_or_else(|| player.map_or(0.0, |d| d.estimated_fuel_burn_ltr()));

            let laps_of_fuel = active_stint
                .filter(|st| st.is_active && !state.is_under_caution && !state.debug_on_pit_road)
                .map(|s| s.laps_of_fuel_crossing)
                .filter(|&l| l > 0.0)
                .unwrap_or_else(|| {
                    if fuel_burn_rate_ltr > 0.001 && fuel_level_ltr > 0.01 {
                        (fuel_level_ltr / fuel_burn_rate_ltr).max(0.0)
                    } else {
                        0.0
                    }
                });

            let player_lap = player.map_or(0, |p| p.total_laps_completed.max(0));
            let pit_target_lap = player_lap + (laps_of_fuel as i32).max(0);

            let is_checkered = state.session_flag_name == "CHECKERED"
                || (state.debug_session_flags & FLAG_CHECKERED) != 0
                || state.debug_session_state == STATE_CHECKERED;
            let is_white = state.session_flag_name == "WHITE FLAG"
                || (state.debug_session_flags & FLAG_WHITE) != 0;

            let laps_remain = compute_overlay_laps_remain(
                state.session_laps_total,
                state.session_laps_remain,
                player_lap,
                is_white,
                is_checkered,
            );

            let latched_fuel = active_stint
                .filter(|st| st.is_active && !state.is_under_caution && !state.debug_on_pit_road)
                .map(|s| s.crossing_fuel_level)
                .filter(|&f| f > 0.0)
                .unwrap_or(fuel_level_ltr);

            let (fuel_to_add_display, has_fuel_calc) = if fuel_burn_rate_ltr > 0.001 {
                let needed_total_ltr = (laps_remain as f32 + self.settings.overlay.safety_margin_laps) * fuel_burn_rate_ltr;
                let fuel_to_add_ltr = (needed_total_ltr - latched_fuel).max(0.0);
                (fuel_to_add_ltr * fuel_conv, true)
            } else {
                (0.0, false)
            };

            let inc_limit = state.incident_limit;
            let inc_str = if inc_limit > 0 {
                format!("Incidents: {} / {}x", state.player_incidents, inc_limit)
            } else {
                format!("Incidents: {}x", state.player_incidents)
            };
            let inc_color = if state.player_incidents >= 12 {
                red
            } else if state.player_incidents >= 6 {
                amber
            } else {
                white
            };

            let caution_str = if state.caution_laps > 0 {
                format!("Cautions: {} ({} laps)", state.caution_count, state.caution_laps)
            } else {
                format!("Cautions: {}", state.caution_count)
            };
            let caution_color = if state.is_under_caution { primary } else { muted };

            let (flag_text, flag_color) = if state.is_under_caution {
                ("● Caution", primary)
            } else if state.session_flag_name == "CHECKERED" {
                ("🏁 Checkered", white)
            } else if state.session_flag_name == "WHITE FLAG" {
                ("🏁 White Flag", white)
            } else {
                ("● Green", green)
            };

            (stint_num, stint_lap_count, stint_avg, last_lap, fast_lap, comp_header, comp_last_lap, comp_sub, delta_opt, fuel_level_ltr, laps_of_fuel, pit_target_lap, laps_remain, player_lap, fuel_burn_rate_ltr, fuel_to_add_display, has_fuel_calc, inc_str, inc_color, caution_str, caution_color, flag_text, flag_color, player_pos)
        };

        let (air_temp_val, track_temp_val, wind_spd_val, wind_spd_unit, rel_wind_rad, wind_compass_str, air_str, track_str) = if is_preview {
            if is_metric {
                (Some(22), Some(28), Some(10), "km/h", Some(90.0_f32.to_radians()), "NE".to_string(), "22°C".to_string(), "28°C".to_string())
            } else {
                (Some(72), Some(82), Some(6), "mph", Some(90.0_f32.to_radians()), "NE".to_string(), "72°F".to_string(), "82°F".to_string())
            }
        } else {
            let air_val = parse_temp_num(&state.air_temp_raw, is_metric);
            let trk_val = parse_temp_num(&state.track_temp_raw, is_metric);
            let (w_spd, w_unit) = parse_wind_speed(&state.wind_speed_raw, is_metric);
            let w_rad = parse_wind_direction_rad(&state.wind_dir_raw);
            let w_cmp = format_wind_dir(&state.wind_dir_raw);
            let a_str = convert_unit(&state.air_temp_raw, is_metric);
            let t_str = convert_unit(&state.track_temp_raw, is_metric);
            let rel_rad = w_rad.map(|raw_rad| calculate_relative_wind_rad(raw_rad, state.player_yaw_north));
            (air_val, trk_val, w_spd, w_unit, rel_rad, w_cmp, a_str, t_str)
        };

        let temp_display_str = match (air_temp_val, track_temp_val) {
            (Some(a), Some(t)) => format!("{}/{}", a, t),
            (Some(a), None) => format!("{}/—", a),
            (None, Some(t)) => format!("—/{}", t),
            (None, None) => "—/—".to_string(),
        };

        let temp_tooltip = format!(
            "Air Temp: {}\nTrack Temp: {}",
            if !air_str.is_empty() && air_str != "..." && air_str != "—" { &air_str } else { "Awaiting telemetry" },
            if !track_str.is_empty() && track_str != "..." && track_str != "—" { &track_str } else { "Awaiting telemetry" }
        );

        let show_bottom_bar = show_rc || show_weather;

        let mut base_h = 16.0;
        let mut count = 0;
        if show_pace {
            base_h += 77.0;
            count += 1;
        }
        if show_fuel {
            base_h += 77.0;
            count += 1;
        }
        if show_bottom_bar {
            base_h += 34.0;
            count += 1;
        }
        if count > 1 {
            base_h += (count - 1) as f32 * 5.0;
        }
        // Minimal clean buffer for border stroke (4px) - eliminates dead void at bottom
        base_h += 4.0;

        let target_height = (base_h * scale).round();
        let target_width = (595.0 * scale).round();

        let hdr_font_size = (11.5 * scale).round();
        let hero_font_size = (22.0 * scale).round();
        let sub_font_size = (15.5 * scale).round();
        let rc_font_size = (14.0 * scale).round();
        let card_padding_y = (6.0 * scale).max(4.0);
        let card_padding_x = (10.0 * scale).max(6.0);

        let vp_id = egui::ViewportId::from_hash_of("s4s_floating_overlay_hud");
        let initial_size = [target_width, target_height];

        let mut vp_builder = egui::ViewportBuilder::default()
            .with_title("S4S Floating Overlay HUD")
            .with_inner_size(initial_size)
            .with_min_inner_size([240.0 * scale, 36.0 * scale])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_minimize_button(false)
            .with_close_button(false)
            .with_taskbar(false);

        if let Some(pos) = self.settings.overlay.position {
            vp_builder = vp_builder.with_position([pos[0], pos[1]]);
        }

        if always_on_top {
            vp_builder = vp_builder.with_always_on_top();
        }

        ctx.show_viewport_immediate(vp_id, vp_builder, |ctx, _class| {
            let current_pos = ctx.input(|i| i.viewport().outer_rect.map(|r| [r.min.x, r.min.y]));
            if let Some([new_x, new_y]) = current_pos {
                if new_x > -10000.0 && new_y > -10000.0 && (new_x.abs() > 0.1 || new_y.abs() > 0.1) {
                    let should_save = match self.settings.overlay.position {
                        Some([old_x, old_y]) => (old_x - new_x).abs() > 2.0 || (old_y - new_y).abs() > 2.0,
                        None => true,
                    };
                    if should_save {
                        self.settings.overlay.position = Some([new_x, new_y]);
                        self.settings.save();
                    }
                }
            }

            if ctx.input(|i| i.pointer.primary_down()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            let outer_bg_alpha = (bg_opacity * 255.0).round() as u8;
            let bg_color = if outer_bg_alpha == 0 {
                egui::Color32::TRANSPARENT
            } else {
                egui::Color32::from_rgba_unmultiplied(11, 13, 18, outer_bg_alpha)
            };
            let outer_stroke = if outer_bg_alpha == 0 {
                egui::Stroke::NONE
            } else {
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, (18.0 * bg_opacity).round() as u8))
            };

            let card_alpha = (opacity * 255.0).round() as u8;
            let card_fill = egui::Color32::from_rgba_unmultiplied(19, 22, 29, card_alpha);
            let inner_card_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, (22.0 * opacity).round() as u8));

            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(bg_color).rounding(12.0).inner_margin(8.0 * scale).stroke(outer_stroke))
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0 * scale, (5.0 * scale).max(3.0));

                    // --- SECTION 1: STINT & ROLLING PACE ---
                    if show_pace {
                        egui::Frame::none()
                            .fill(card_fill)
                            .stroke(inner_card_stroke)
                            .rounding(8.0)
                            .inner_margin(egui::Margin::symmetric(card_padding_x, card_padding_y))
                            .show(ui, |ui| {
                                ui.columns(4, |cols| {
                                    cols[0].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        ui.label(egui::RichText::new("STINT · LAP").color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                        ui.label(egui::RichText::new(format!("S{} · L{}", stint_num, stint_lap_count)).font(display_font(hero_font_size)).color(primary));
                                        let avg_str = if stint_avg > 0.0 { format!("Avg {}", format_time(stint_avg)) } else { "—".to_string() };
                                        ui.label(egui::RichText::new(avg_str).color(muted).size(sub_font_size));
                                    });
                                    cols[1].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        ui.label(egui::RichText::new("YOUR LAST").color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                        let last_str = if last_lap > 0.0 { format_time(last_lap) } else { "—".to_string() };
                                        ui.label(egui::RichText::new(last_str).font(display_font(hero_font_size)).color(white));
                                        let best_str = if fast_lap > 0.0 { format!("Best {}", format_time(fast_lap)) } else { "—".to_string() };
                                        ui.label(egui::RichText::new(best_str).color(muted).size(sub_font_size));
                                    });
                                    cols[2].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        ui.label(egui::RichText::new(comp_header).color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                        let comp_str = if comp_last_lap > 0.0 { format_time(comp_last_lap) } else { "—".to_string() };
                                        ui.label(egui::RichText::new(comp_str).font(display_font(hero_font_size)).color(white));
                                        ui.label(egui::RichText::new(comp_sub).color(muted).size(sub_font_size));
                                    });
                                    cols[3].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        let delta_title = if player_pos == 1 { "DELTA VS P2" } else { "DELTA VS P1" };
                                        ui.label(egui::RichText::new(delta_title).color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                        if let Some(delta) = delta_opt {
                                            let (delta_str, delta_color, sub_text) = if delta < -0.005 {
                                                (format!("-{:.3}s", delta.abs()), green, if player_pos == 1 { "Pulling away" } else { "Faster than P1" })
                                            } else if delta > 0.005 {
                                                (format!("+{:.3}s", delta), amber, if player_pos == 1 { "P2 closing" } else { "Slower than P1" })
                                            } else {
                                                ("±0.000s".to_string(), white, "Equal pace")
                                            };
                                            ui.label(egui::RichText::new(delta_str).font(display_font(hero_font_size)).color(delta_color));
                                            ui.label(egui::RichText::new(sub_text).color(muted).size(sub_font_size));
                                        } else {
                                            ui.label(egui::RichText::new("—").font(display_font(hero_font_size)).color(muted));
                                            ui.label(egui::RichText::new("Awaiting Laps").color(muted).size(sub_font_size));
                                        }
                                    });
                                });
                            });
                    }

                    // --- SECTION 2: LIVE PIT & FUEL TO ADD CALCULATOR ---
                    if show_fuel {
                        egui::Frame::none()
                            .fill(card_fill)
                            .stroke(inner_card_stroke)
                            .rounding(8.0)
                            .inner_margin(egui::Margin::symmetric(card_padding_x, card_padding_y))
                            .show(ui, |ui| {
                                ui.columns(4, |cols| {
                                    cols[0].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        match self.settings.overlay.fuel_hero_metric {
                                            FuelHeroMetric::FuelAmount => {
                                                ui.label(egui::RichText::new("FUEL IN TANK").color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                                let tank_str = if fuel_level_ltr > 0.01 {
                                                    format!("{:.1} {}", fuel_level_ltr * fuel_conv, fuel_unit_str)
                                                } else {
                                                    format!("— {}", fuel_unit_str)
                                                };
                                                ui.label(egui::RichText::new(tank_str).font(display_font(hero_font_size)).color(primary));

                                                let laps_sub = if laps_of_fuel > 0.05 && fuel_burn_rate_ltr > 0.001 {
                                                    format!("{:.1} laps left", laps_of_fuel)
                                                } else {
                                                    "— laps left".to_string()
                                                };
                                                ui.label(egui::RichText::new(laps_sub).color(muted).size(sub_font_size));
                                            }
                                            FuelHeroMetric::LapsToEmpty => {
                                                ui.label(egui::RichText::new("LAPS TO EMPTY").color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                                let laps_hero_str = if laps_of_fuel > 0.05 && fuel_burn_rate_ltr > 0.001 {
                                                    format!("{:.1} laps", laps_of_fuel)
                                                } else {
                                                    "— laps".to_string()
                                                };
                                                ui.label(egui::RichText::new(laps_hero_str).font(display_font(hero_font_size)).color(primary));

                                                let tank_str = if fuel_level_ltr > 0.01 {
                                                    format!("{:.1} {} in tank", fuel_level_ltr * fuel_conv, fuel_unit_str)
                                                } else {
                                                    "— in tank".to_string()
                                                };
                                                ui.label(egui::RichText::new(tank_str).color(muted).size(sub_font_size));
                                            }
                                        }
                                    });

                                    cols[1].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        ui.label(egui::RichText::new("PIT WINDOW").color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                        let pit_str = if pit_target_lap > 0 && laps_of_fuel > 0.5 && fuel_burn_rate_ltr > 0.001 {
                                            format!("Lap {}", pit_target_lap)
                                        } else {
                                            "—".to_string()
                                        };
                                        ui.label(egui::RichText::new(pit_str).font(display_font(hero_font_size)).color(amber));

                                        let rate_str = if fuel_burn_rate_ltr > 0.001 {
                                            format!("{:.3} {}/lap", fuel_burn_rate_ltr * fuel_conv, fuel_unit_str)
                                        } else {
                                            "Awaiting Lap 1".to_string()
                                        };
                                        ui.label(egui::RichText::new(rate_str).color(muted).size(sub_font_size));
                                    });

                                    cols[2].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        ui.label(egui::RichText::new("RACE REMAIN").color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                        let is_checkered_flag = state.session_flag_name == "CHECKERED"
                                            || (state.debug_session_flags & FLAG_CHECKERED) != 0
                                            || state.debug_session_state == STATE_CHECKERED;
                                        let (rem_str, lap_done_str) = format_overlay_race_remain_strings(
                                            laps_remain,
                                            player_lap,
                                            is_checkered_flag,
                                            state.session_laps_total,
                                        );
                                        ui.label(egui::RichText::new(rem_str).font(display_font(hero_font_size)).color(white));
                                        ui.label(egui::RichText::new(lap_done_str).color(muted).size(sub_font_size));
                                    });

                                    cols[3].vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
                                        ui.label(egui::RichText::new("FUEL TO ADD (F4)").color(muted).size(hdr_font_size).extra_letter_spacing(0.8));
                                        let add_str = if !has_fuel_calc {
                                            "—".to_string()
                                        } else if laps_remain > 0 {
                                            if fuel_to_add_display <= 0.01 {
                                                "0.0 (OK)".to_string()
                                            } else {
                                                format!("+{:.1} {}", fuel_to_add_display, fuel_unit_str)
                                            }
                                        } else {
                                            "—".to_string()
                                        };

                                        let add_color = if !has_fuel_calc {
                                            muted
                                        } else if fuel_to_add_display > 0.01 {
                                            red
                                        } else {
                                            green
                                        };
                                        ui.label(egui::RichText::new(add_str).font(display_font(hero_font_size)).color(add_color));
                                        ui.label(egui::RichText::new(format!("+{:.1} lap margin", safety_margin)).color(muted).size(sub_font_size));
                                    });
                                });
                            });
                    }

                    // --- SECTION 3: RACE CONTROL, INCIDENTS, CAUTIONS & WEATHER BAR ---
                    if show_bottom_bar {
                        egui::Frame::none()
                            .fill(card_fill)
                            .stroke(inner_card_stroke)
                            .rounding(8.0)
                            .inner_margin(egui::Margin::symmetric(card_padding_x, (4.0 * scale).max(2.5)))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0 * scale;

                                    if show_rc {
                                        ui.label(egui::RichText::new(&inc_str).color(inc_color).size(rc_font_size).strong());

                                        ui.add_space(6.0 * scale);
                                        ui.label(egui::RichText::new(&caution_str).color(caution_color).size(rc_font_size));
                                    }

                                    if show_rc && show_weather {
                                        ui.add_space(8.0 * scale);
                                        ui.label(egui::RichText::new("·").color(muted).size(rc_font_size));
                                        ui.add_space(8.0 * scale);
                                    }

                                    if show_weather {
                                        let temp_resp = ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing.x = 4.0 * scale;
                                            ui.label(egui::RichText::new("Air/Track Temp:").color(primary).size(rc_font_size).strong());
                                            ui.label(egui::RichText::new(&temp_display_str).color(egui::Color32::from_rgb(56, 189, 248)).size(rc_font_size).strong());
                                        }).response;
                                        temp_resp.on_hover_text(&temp_tooltip);

                                        ui.add_space(6.0 * scale);
                                        draw_wind_indicator(ui, wind_spd_val, wind_spd_unit, rel_wind_rad, &wind_compass_str, scale);
                                    }

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(flag_text).color(flag_color).size(rc_font_size).strong());
                                    });
                                });
                            });
                    }

                    let measured_h = (ui.min_rect().max.y + 8.0 * scale).round().max(36.0 * scale);
                    let current_size = ctx.input(|i| i.viewport().inner_rect.map(|r| [r.width(), r.height()]));
                    if let Some([cw, ch]) = current_size {
                        if (cw - target_width).abs() > 1.5 || (ch - measured_h).abs() > 1.5 {
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(target_width, measured_h)));
                        }
                    }
                });

            ensure_overlay_hwnd_transparency("S4S Floating Overlay HUD", &mut self.main_overlay_hwnd);
        });
    }

    pub(crate) fn render_tire_popup_overlay(&mut self, ctx: &egui::Context, state: &WarRoomState) {
        if state.tire_popup_trigger_seq > self.last_seen_tire_popup_seq {
            self.last_seen_tire_popup_seq = state.tire_popup_trigger_seq;
            self.tire_popup_stint_num = state.tire_popup_stint_num;
            if self.settings.tire_popup_overlay.enabled {
                let dur = self.settings.tire_popup_overlay.duration_secs.clamp(3.0, 60.0);
                self.tire_popup_active_until = Some(std::time::Instant::now() + std::time::Duration::from_secs_f32(dur));
            }
        }

        let now = std::time::Instant::now();
        let remaining_secs = self.tire_popup_active_until.and_then(|until| {
            if until > now {
                Some((until - now).as_secs_f32())
            } else {
                None
            }
        });

        if remaining_secs.is_none() && self.tire_popup_active_until.is_some() {
            self.tire_popup_active_until = None;
        }

        let show_popup = self.preview_tire_popup_hud
            || (self.settings.tire_popup_overlay.enabled && remaining_secs.is_some());

        if !show_popup {
            self.tire_popup_hwnd = (0, 0);
            return;
        }

        if remaining_secs.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        let primary = self.settings.primary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let amber = self.settings.status_amber();
        let is_metric = self.settings.use_metric;

        let scale = self.settings.tire_popup_overlay.scale.clamp(0.75, 1.50);
        let opacity = self.settings.tire_popup_overlay.opacity.clamp(0.05, 1.0);
        let bg_opacity = self.settings.tire_popup_overlay.bg_opacity.clamp(0.0, 1.0);
        let always_on_top = self.settings.tire_popup_overlay.always_on_top;

        // Resolve player stint tire set or fallback to preview sample
        let player_drv = state.drivers.get(&state.player_car_idx);
        let matched_stint = player_drv.and_then(|d| {
            d.stints
                .iter()
                .rev()
                .find(|st| st.stint_number == self.tire_popup_stint_num && st.tires.has_wear_or_temps())
                .or_else(|| d.stints.iter().rev().find(|st| st.tires.has_wear_or_temps()))
        });

        let (display_stint_num, display_laps, tires_data, is_sample_data) = if let Some(st) = matched_stint {
            (st.stint_number, st.laps_completed, st.tires.clone(), false)
        } else if state.latest_tire_data.has_wear_or_temps() {
            (self.tire_popup_stint_num.max(1), 0, state.latest_tire_data.clone(), false)
        } else {
            // Realistic preview sample so user can position & size the popup anytime
            let sample = StintTireSet {
                lf: TireCornerData {
                    temp_l: 88.0,
                    temp_m: 92.0,
                    temp_r: 96.0,
                    wear_l: 94.0,
                    wear_m: 91.0,
                    wear_r: 89.0,
                    pressure: 186.2,
                },
                rf: TireCornerData {
                    temp_l: 108.0,
                    temp_m: 104.0,
                    temp_r: 99.0,
                    wear_l: 78.0,
                    wear_m: 81.0,
                    wear_r: 85.0,
                    pressure: 206.8,
                },
                lr: TireCornerData {
                    temp_l: 86.0,
                    temp_m: 89.0,
                    temp_r: 91.0,
                    wear_l: 95.0,
                    wear_m: 93.0,
                    wear_r: 92.0,
                    pressure: 179.3,
                },
                rr: TireCornerData {
                    temp_l: 103.0,
                    temp_m: 100.0,
                    temp_r: 96.0,
                    wear_l: 83.0,
                    wear_m: 85.0,
                    wear_r: 88.0,
                    pressure: 199.9,
                },
                has_data: true,
            };
            (1, 20, sample, true)
        };

        let target_width = (448.0 * scale).round();
        let target_height = (178.0 * scale).round();

        let vp_id = egui::ViewportId::from_hash_of("s4s_tire_popup_overlay_hud");
        let mut vp_builder = egui::ViewportBuilder::default()
            .with_title("S4S Post-Pit Tire Popup")
            .with_inner_size([target_width, target_height])
            .with_min_inner_size([240.0 * scale, 100.0 * scale])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_minimize_button(false)
            .with_close_button(false)
            .with_taskbar(false);

        if let Some(pos) = self.settings.tire_popup_overlay.position {
            vp_builder = vp_builder.with_position([pos[0], pos[1]]);
        }

        if always_on_top {
            vp_builder = vp_builder.with_always_on_top();
        }

        let mut dismiss_clicked = false;

        ctx.show_viewport_immediate(vp_id, vp_builder, |ctx, _class| {
            let current_pos = ctx.input(|i| i.viewport().outer_rect.map(|r| [r.min.x, r.min.y]));
            if let Some([new_x, new_y]) = current_pos {
                if new_x > -10000.0 && new_y > -10000.0 && (new_x.abs() > 0.1 || new_y.abs() > 0.1) {
                    let should_save = match self.settings.tire_popup_overlay.position {
                        Some([old_x, old_y]) => (old_x - new_x).abs() > 2.0 || (old_y - new_y).abs() > 2.0,
                        None => true,
                    };
                    if should_save {
                        self.settings.tire_popup_overlay.position = Some([new_x, new_y]);
                        self.settings.save();
                    }
                }
            }

            let outer_bg_alpha = (bg_opacity * 255.0).round() as u8;
            let bg_color = if outer_bg_alpha == 0 {
                egui::Color32::TRANSPARENT
            } else {
                egui::Color32::from_rgba_unmultiplied(11, 13, 18, outer_bg_alpha)
            };
            let outer_stroke = if outer_bg_alpha == 0 {
                egui::Stroke::NONE
            } else {
                egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, (18.0 * bg_opacity).round() as u8))
            };

            let card_alpha = (opacity * 255.0).round() as u8;
            let card_fill = egui::Color32::from_rgba_unmultiplied(19, 22, 29, card_alpha);
            let inner_card_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, (24.0 * opacity).round() as u8));

            let mut measured_h = target_height;

            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(bg_color).rounding(12.0).inner_margin(8.0 * scale).stroke(outer_stroke))
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0 * scale, 6.0 * scale);

                    // Header bar
                    egui::Frame::none()
                        .fill(card_fill)
                        .stroke(inner_card_stroke)
                        .rounding(8.0)
                        .inner_margin(egui::Margin::symmetric((10.0 * scale).max(6.0), (5.0 * scale).max(3.0)))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let title_str = if display_laps > 0 {
                                    format!("● POST-PIT TIRES · S{} ({} LAPS)", display_stint_num, display_laps)
                                } else {
                                    format!("● POST-PIT TIRES · STINT S{}", display_stint_num)
                                };
                                ui.label(egui::RichText::new(title_str).color(primary).size((12.0 * scale).round()).strong());

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add(
                                        egui::Button::new(egui::RichText::new("X").color(muted).size((11.5 * scale).round()).strong())
                                            .fill(egui::Color32::TRANSPARENT)
                                            .frame(false)
                                    ).on_hover_text("Dismiss tire popup").clicked() {
                                        dismiss_clicked = true;
                                    }

                                    if let Some(rem) = remaining_secs {
                                        ui.label(
                                            egui::RichText::new(format!("{:.0}s", rem.ceil()))
                                                .color(primary)
                                                .size((11.5 * scale).round())
                                                .strong(),
                                        );
                                    } else if self.preview_tire_popup_hud {
                                        let badge = if is_sample_data { "PREVIEW · DRAG TO MOVE" } else { "LIVE PREVIEW · DRAG" };
                                        ui.label(
                                            egui::RichText::new(badge)
                                                .color(amber)
                                                .size((10.5 * scale).round())
                                                .strong(),
                                        );
                                    }
                                });
                            });
                        });

                    // 2x2 Tire Corners Grid: Top Row (LF | RF), Bottom Row (LR | RR)
                    let mut col_bottom_y = 0.0_f32;
                    ui.columns(2, |cols| {
                        cols[0].vertical(|ui| {
                            render_overlay_tire_corner_card(
                                ui,
                                "LF · Left Front",
                                &tires_data.lf,
                                is_metric,
                                scale,
                                card_fill,
                                opacity,
                                primary,
                                white,
                                muted,
                            );
                            ui.add_space(6.0 * scale);
                            render_overlay_tire_corner_card(
                                ui,
                                "LR · Left Rear",
                                &tires_data.lr,
                                is_metric,
                                scale,
                                card_fill,
                                opacity,
                                primary,
                                white,
                                muted,
                            );
                        });
                        cols[1].vertical(|ui| {
                            render_overlay_tire_corner_card(
                                ui,
                                "RF · Right Front",
                                &tires_data.rf,
                                is_metric,
                                scale,
                                card_fill,
                                opacity,
                                primary,
                                white,
                                muted,
                            );
                            ui.add_space(6.0 * scale);
                            render_overlay_tire_corner_card(
                                ui,
                                "RR · Right Rear",
                                &tires_data.rr,
                                is_metric,
                                scale,
                                card_fill,
                                opacity,
                                primary,
                                white,
                                muted,
                            );
                        });
                        col_bottom_y = cols[0].min_rect().max.y.max(cols[1].min_rect().max.y);
                    });

                    if col_bottom_y > 20.0 {
                        measured_h = (col_bottom_y + 8.0 * scale).round().max(100.0 * scale);
                    }
                });

            if !dismiss_clicked && ctx.input(|i| i.pointer.primary_down()) {
                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }

            let current_size = ctx.input(|i| i.viewport().inner_rect.map(|r| [r.width(), r.height()]));
            if let Some([cw, ch]) = current_size {
                if (cw - target_width).abs() > 1.5 || (ch - measured_h).abs() > 1.5 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(target_width, measured_h)));
                }
            }

            ensure_overlay_hwnd_transparency("S4S Post-Pit Tire Popup", &mut self.tire_popup_hwnd);
        });

        if dismiss_clicked {
            self.tire_popup_active_until = None;
            self.preview_tire_popup_hud = false;
            self.tire_popup_hwnd = (0, 0);
        }
    }

}

pub(crate) fn render_overlay_tire_corner_card(
    ui: &mut egui::Ui,
    corner_title: &str,
    data: &TireCornerData,
    use_metric: bool,
    scale: f32,
    card_fill: egui::Color32,
    opacity: f32,
    primary: egui::Color32,
    white: egui::Color32,
    muted: egui::Color32,
) {
    let avg_wear = if data.wear_m > 0.01 {
        (data.wear_l + data.wear_m + data.wear_r) / 3.0
    } else {
        0.0
    };

    let wear_color = if avg_wear >= 90.0 {
        egui::Color32::from_rgb(75, 225, 110)
    } else if avg_wear >= 80.0 {
        primary
    } else if avg_wear >= 70.0 {
        egui::Color32::from_rgb(255, 165, 50)
    } else if avg_wear > 0.0 {
        egui::Color32::from_rgb(255, 75, 75)
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

    let is_left = corner_title.starts_with("LF") || corner_title.starts_with("LR");
    let zone_label = if is_left { "O·M·I" } else { "I·M·O" };

    let temp_str = if data.temp_m > 1.0 {
        if use_metric {
            format!("{:.0}° · {:.0}° · {:.0}°C", data.temp_l, data.temp_m, data.temp_r)
        } else {
            format!(
                "{:.0}° · {:.0}° · {:.0}°F",
                data.temp_l * 1.8 + 32.0,
                data.temp_m * 1.8 + 32.0,
                data.temp_r * 1.8 + 32.0
            )
        }
    } else {
        "— · — · —".to_string()
    };

    let wear_str = if data.wear_m > 0.01 {
        format!("{:.0}% · {:.0}% · {:.0}%", data.wear_l, data.wear_m, data.wear_r)
    } else {
        "—% · —% · —%".to_string()
    };

    let border_alpha = ((if avg_wear > 0.0 { 85.0 } else { 22.0 }) * opacity).round() as u8;
    let card_border = if avg_wear > 0.0 {
        with_alpha(wear_color, border_alpha)
    } else {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, border_alpha)
    };

    egui::Frame::none()
        .fill(card_fill)
        .stroke(egui::Stroke::new(1.0, card_border))
        .rounding(egui::Rounding::same(8.0 * scale))
        .inner_margin(egui::Margin::symmetric((8.0 * scale).max(5.0), (6.0 * scale).max(4.0)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = (2.0 * scale).max(1.0);
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(corner_title).color(white).size((12.0 * scale).round()).strong());
                if avg_wear > 0.0 {
                    ui.label(egui::RichText::new(format!("{:.0}%", avg_wear)).color(wear_color).size((12.0 * scale).round()).strong());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(press_str).color(muted).size((11.0 * scale).round()));
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("WEAR ({})", zone_label)).color(muted).size((9.5 * scale).round()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&wear_str).color(wear_color).size((12.5 * scale).round()).strong());
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("TEMP ({})", zone_label)).color(muted).size((9.5 * scale).round()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(&temp_str).color(white).size((12.0 * scale).round()).strong());
                });
            });
        });
}
