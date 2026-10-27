#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod discord_card;
pub mod formatters;
pub mod ibt;
pub mod ibt_ui;
pub mod models;
pub mod settings;
pub mod stint_db;
pub mod telemetry;
mod updater;

pub use formatters::*;
pub use models::*;
pub use telemetry::*;
pub mod views;
pub use views::*;

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;
use iracing::telemetry::Value;
use settings::{ActiveViewMode, AppSettings};

fn main() -> eframe::Result<()> {
    let title = if settings::is_beta_mode() {
        "The Search For Speed - Stint Analyzer [BETA]"
    } else {
        "The Search For Speed - Stint Analyzer"
    };

    let app_icon = {
        let bytes = include_bytes!("../assets/icon.ico");
        image::load_from_memory(bytes).ok().map(|img| {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            egui::IconData {
                rgba: rgba.into_raw(),
                width: w,
                height: h,
            }
        })
    };

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1400.0, 820.0])
        .with_min_inner_size([960.0, 560.0])
        .with_title(title);

    if let Some(icon) = app_icon {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "The Search For Speed Analyzer",
        options,
        Box::new(|cc| {
            Box::new(WarRoomApp::new(cc))
        }),
    )
}

pub(crate) fn display_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Display".into()))
}

pub(crate) fn with_alpha(c: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

pub(crate) fn setup_custom_style(ctx: &egui::Context, settings: &AppSettings) {
    let mut style = (*ctx.style()).clone();
    let is_dark = settings.is_dark_theme();
    
    if is_dark {
        style.visuals = egui::Visuals::dark();
        style.visuals.dark_mode = true;
    } else {
        style.visuals = egui::Visuals::light();
        style.visuals.dark_mode = false;
    }
    
    let primary = settings.primary_accent();
    let secondary = settings.secondary_accent();
    let bg_canvas = settings.bg_color();
    let card_bg = settings.panel_color();
    let elevated_bg = settings.card_elevated_color();
    let text_main = settings.text_color();
    let divider = settings.divider_color();

    style.visuals.window_rounding = egui::Rounding::same(12.0);
    style.visuals.menu_rounding = egui::Rounding::same(8.0);
    if is_dark {
        style.visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 10.0),
            blur: 24.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(160),
        };
        style.visuals.popup_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 6.0),
            blur: 14.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(120),
        };
        style.visuals.faint_bg_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 5);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(8, 10, 14);
    } else {
        style.visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 8.0),
            blur: 20.0,
            spread: 0.0,
            color: egui::Color32::from_rgba_unmultiplied(20, 30, 55, 30),
        };
        style.visuals.popup_shadow = egui::epaint::Shadow {
            offset: egui::vec2(0.0, 4.0),
            blur: 12.0,
            spread: 0.0,
            color: egui::Color32::from_rgba_unmultiplied(20, 30, 55, 25),
        };
        style.visuals.faint_bg_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 6);
        style.visuals.extreme_bg_color = egui::Color32::from_rgb(255, 255, 255);
    }
    
    style.visuals.panel_fill = bg_canvas;
    style.visuals.window_fill = card_bg;
    style.visuals.window_stroke = egui::Stroke::new(1.0, divider);
    
    style.visuals.widgets.noninteractive.bg_fill = card_bg;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_main);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, divider);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);
    
    // Ghost / Tonal default buttons
    style.visuals.widgets.inactive.bg_fill = elevated_bg;
    style.visuals.widgets.inactive.weak_bg_fill = elevated_bg;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_main);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::TRANSPARENT);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    
    // Hovered buttons
    let hover_bg = if is_dark {
        egui::Color32::from_rgb(36, 42, 56)
    } else {
        egui::Color32::from_rgb(220, 226, 238)
    };
    style.visuals.widgets.hovered.bg_fill = hover_bg;
    style.visuals.widgets.hovered.weak_bg_fill = hover_bg;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, if is_dark { egui::Color32::WHITE } else { text_main });
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, with_alpha(primary, 110));
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    
    // Active buttons
    style.visuals.widgets.active.bg_fill = primary;
    style.visuals.widgets.active.weak_bg_fill = primary;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.5, if is_dark { egui::Color32::BLACK } else { egui::Color32::WHITE });
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, primary);
    style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);
    
    style.visuals.selection.bg_fill = with_alpha(secondary, 90);
    style.visuals.selection.stroke = egui::Stroke::new(1.5, primary);

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Medium.ttf")),
    );
    fonts.font_data.insert(
        "RacingSans".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/RacingSansOne-Regular.ttf")),
    );
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "Inter".to_owned());

    let mut display_family = vec!["RacingSans".to_owned(), "Inter".to_owned()];
    for font_name in fonts.families.get(&egui::FontFamily::Proportional).unwrap() {
        if !display_family.contains(font_name) {
            display_family.push(font_name.clone());
        }
    }
    fonts.families.insert(
        egui::FontFamily::Name("Display".into()),
        display_family,
    );
    ctx.set_fonts(fonts);
    
    let base_heading = 22.0 + settings.font_size_offset;
    let base_body = 13.5 + settings.font_size_offset;
    let base_small = 11.5 + settings.font_size_offset;

    use egui::{TextStyle, FontId, FontFamily::Proportional, FontFamily::Monospace};
    style.text_styles = [
        (TextStyle::Heading, display_font(base_heading)),
        (TextStyle::Body, FontId::new(base_body, Proportional)),
        (TextStyle::Monospace, FontId::new(base_body, Monospace)),
        (TextStyle::Button, FontId::new(base_body, Proportional)),
        (TextStyle::Small, FontId::new(base_small, Proportional)),
    ].into();

    ctx.set_style(style);
    ctx.options_mut(|o| {
        o.warn_on_id_clash = false;
    });
}


pub(crate) struct WarRoomApp {
    pub(crate) state: Arc<Mutex<WarRoomState>>,
    pub(crate) settings: AppSettings,
    pub(crate) logo_texture: Option<egui::TextureHandle>,
    pub(crate) show_settings_modal: bool,
    pub(crate) show_telemetry_debugger: bool,
    pub(crate) selected_driver_car_idx: Option<usize>,
    pub(crate) selected_stint_num: Option<i32>,
    pub(crate) show_stint_analyzer_modal: bool,
    pub(crate) stint_analyzer_tab: StintAnalyzerTab,
    pub(crate) stint_compare_slots: Vec<StintCompareSlot>,
    pub(crate) show_stint_comparison_window: bool,
    pub(crate) custom_bucket_input: String,
    pub(crate) toast_message: Option<(String, std::time::Instant)>,
    pub(crate) grid_sort_key: GridSortKey,
    pub(crate) grid_sort_dir: SortDirection,
    pub(crate) driver_search_query: String,
    pub(crate) updater: updater::UpdateChecker,
    pub(crate) show_release_notes_modal: bool,
    pub(crate) release_notes_custom_content: Option<String>,
    pub(crate) preview_overlay_hud: bool,
    pub(crate) preview_tire_popup_hud: bool,
    pub(crate) last_seen_tire_popup_seq: u64,
    pub(crate) tire_popup_active_until: Option<std::time::Instant>,
    pub(crate) tire_popup_stint_num: i32,
    pub(crate) main_overlay_hwnd: (isize, u8),
    pub(crate) tire_popup_hwnd: (isize, u8),
    pub(crate) ibt_lab: ibt_ui::IbtTelemetryLabState,
    pub(crate) stint_db: stint_db::StintDatabase,
    pub(crate) show_stint_db_modal: bool,
    pub(crate) stint_db_filter_current_track: bool,
    pub(crate) stint_db_search_query: String,
}

pub(crate) fn load_logo_texture(ctx: &egui::Context, is_dark: bool) -> Option<egui::TextureHandle> {
    let bytes = include_bytes!("../assets/logo.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();

    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0;
    let mut max_y = 0;

    for y in 0..h {
        for x in 0..w {
            let px = img.get_pixel(x, y);
            let lum = px[0].max(px[1]).max(px[2]);
            if lum > 15 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    let (crop_x, crop_y, crop_w, crop_h) = if max_x >= min_x && max_y >= min_y {
        (min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
    } else {
        (0, 0, w, h)
    };

    let (r, g, b) = if is_dark {
        (238, 240, 245)
    } else {
        (18, 24, 38)
    };

    let mut raw = Vec::with_capacity((crop_w * crop_h * 4) as usize);
    for y in crop_y..(crop_y + crop_h) {
        for x in crop_x..(crop_x + crop_w) {
            let px = img.get_pixel(x, y);
            let lum = px[0].max(px[1]).max(px[2]);
            if lum > 15 {
                let alpha = ((lum as u16 * 255) / 215).min(255) as u8;
                raw.extend_from_slice(&[r, g, b, alpha]);
            } else {
                raw.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }

    let color_image = egui::ColorImage::from_rgba_unmultiplied([crop_w as usize, crop_h as usize], &raw);
    Some(ctx.load_texture("s4s_logo", color_image, egui::TextureOptions::LINEAR))
}

impl WarRoomApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        updater::cleanup_old_updates();
        let settings = AppSettings::load();
        setup_custom_style(&cc.egui_ctx, &settings);
        let logo_texture = load_logo_texture(&cc.egui_ctx, settings.is_dark_theme());
        
        let state = Arc::new(Mutex::new(WarRoomState {
            track_name: "Waiting for iRacing...".to_string(),
            track_length: "—".to_string(),
            car_name: "—".to_string(),
            session_type: "—".to_string(),
            track_state: "...".to_string(),
            weather_type: "...".to_string(),
            air_temp_raw: "...".to_string(),
            track_temp_raw: "...".to_string(),
            wind_speed_raw: "...".to_string(),
            wind_dir_raw: "...".to_string(),
            relative_humidity_raw: "...".to_string(),
            debug_on_pit_road: true,
            ..Default::default()
        }));
        
        // High-Frequency 60Hz Telemetry Background Thread (0ms Lag, Pure Binary Reads)
        let state_clone = Arc::clone(&state);
        let ctx_clone = cc.egui_ctx.clone();
        
        thread::spawn(move || {
            loop {
                match LiveSimConnection::connect() {
                    Some(mut conn) => {
                        {
                            let mut s = state_clone.lock().unwrap();
                            if s.needs_new_connection_reset
                                || s.session_num != -1
                                || s.drivers.values().any(|d| d.has_live_stints() || d.total_laps_completed > 0)
                            {
                                s.reset_for_new_session();
                            }
                            s.is_connected = true;
                            if !is_real_track_name(&s.track_name) {
                                s.track_name = "iRacing Live Session".to_string();
                            }
                        }
                        ctx_clone.request_repaint();

                        // Try initial session YAML parse directly from live connection
                        if let Some((ver, raw_yaml)) = conn.read_yaml() {
                            let mut s = state_clone.lock().unwrap();
                            s.is_connected = true;
                            if parse_iracing_session_yaml(&raw_yaml, &mut s) {
                                conn.last_session_info_version = ver;
                            }
                        }
                        ctx_clone.request_repaint();

                        let mut last_session_parse_time = std::time::Instant::now();
                        let mut current_fuel_sample = 0.0f32;

                        // Real-time 60Hz Telemetry Loop (re-reads live Header every frame)
                        while let Some((header, live)) = conn.next_sample(33) {
                            let version_changed = header.session_info_version != conn.last_session_info_version;
                            if version_changed || last_session_parse_time.elapsed().as_secs_f32() >= 2.0 {
                                last_session_parse_time = std::time::Instant::now();
                                if let Some((ver, raw_yaml)) = conn.read_yaml() {
                                    let mut s = state_clone.lock().unwrap();
                                    if parse_iracing_session_yaml(&raw_yaml, &mut s) {
                                        conn.last_session_info_version = ver;
                                    }
                                }
                            }

                            let mut s = state_clone.lock().unwrap();
                            s.is_connected = true;

                            // Auto-detect player car index and update track label if pending
                            if let Ok(Value::INT(p_idx)) = live.get("PlayerCarIdx") {
                                s.player_car_idx = p_idx as usize;
                            }
                            if s.track_name == "Connecting to Session..." || s.track_name == "Waiting for iRacing..." || s.track_name == "Waiting for connection..." {
                                s.track_name = "iRacing Live Session".to_string();
                            }

                            // Read Fuel
                            if let Ok(Value::FLOAT(fuel)) = live.get("FuelLevel") {
                                if fuel >= 0.0 {
                                    current_fuel_sample = fuel;
                                    s.player_fuel_level = fuel;
                                }
                            }
                            if let Ok(Value::FLOAT(pct)) = live.get("FuelLevelPct") {
                                s.player_fuel_pct = pct;
                            }

                            // Read Live Track Temp & Air Temp directly from live telemetry SDK (degrees Celsius)
                            if let Ok(Value::FLOAT(tt)) = live.get("TrackTemp") {
                                if tt > -40.0 && tt < 130.0 {
                                    s.track_temp_raw = format!("{:.1} C", tt);
                                }
                            } else if let Ok(Value::FLOAT(tt_crew)) = live.get("TrackTempCrew") {
                                if tt_crew > -40.0 && tt_crew < 130.0 {
                                    s.track_temp_raw = format!("{:.1} C", tt_crew);
                                }
                            }

                            if let Ok(Value::FLOAT(at)) = live.get("AirTemp") {
                                if at > -40.0 && at < 130.0 {
                                    s.air_temp_raw = format!("{:.1} C", at);
                                }
                            }

                            if let Ok(Value::FLOAT(wv)) = live.get("WindVel") {
                                if wv >= 0.0 && wv < 100.0 {
                                    s.wind_speed_raw = format!("{:.2} m/s", wv);
                                }
                            }
                            if let Ok(Value::FLOAT(wd)) = live.get("WindDir") {
                                if wd.is_finite() {
                                    s.wind_dir_raw = format!("{:.3} rad", wd);
                                }
                            }
                            if let Ok(Value::FLOAT(yn)) = live.get("YawNorth") {
                                if yn.is_finite() {
                                    s.player_yaw_north = yn;
                                }
                            } else if let Ok(Value::FLOAT(yaw)) = live.get("Yaw") {
                                if yaw.is_finite() {
                                    s.player_yaw_north = yaw;
                                }
                            }
                            if let Ok(Value::FLOAT(rh)) = live.get("RelativeHumidity") {
                                if rh >= 0.0 && rh <= 100.0 {
                                    let rh_pct = if rh <= 1.0 && rh > 0.0 { rh * 100.0 } else { rh };
                                    s.relative_humidity_raw = format!("{:.0}%", rh_pct);
                                }
                            }

                            // Read Session Laps Remaining: prefer SessionLapsRemainEx (modern, accurate variable)
                            // over SessionLapsRemain (legacy variable which has known off-by-one errors)
                            let remain_val = live.get("SessionLapsRemainEx")
                                .or_else(|_| live.get("SessionLapsRemain"));
                            if let Ok(Value::INT(remain)) = remain_val {
                                if remain >= 0 && remain < 10000 {
                                    s.session_laps_remain = remain;
                                } else if remain >= 10000 {
                                    // 32767 sentinel value represents unlimited laps / timed session
                                    s.session_laps_remain = 0;
                                }
                            }

                            // Read Session Laps Total (scheduled total laps for the session)
                            if let Ok(Value::INT(total)) = live.get("SessionLapsTotal") {
                                if total > 0 && total < 10000 {
                                    s.session_laps_total = total;
                                } else if total >= 10000 {
                                    s.session_laps_total = 0;
                                }
                            }

                            // Read Driver Incidents
                            if let Ok(Value::INT(inc)) = live.get("PlayerCarMyIncCount") {
                                s.player_incidents = inc;
                            } else if let Ok(Value::INT(inc)) = live.get("PlayerCarDriverIncCount") {
                                s.player_incidents = inc;
                            } else if let Ok(Value::INT(inc)) = live.get("PlayerCarTeamIncCount") {
                                s.player_incidents = inc;
                            }

                            // Read ALL Live Timing / Telemetry Variables for Debugger
                            if let Ok(Value::INT(lap_no)) = live.get("Lap") { s.debug_lap_number = lap_no; }
                            if let Ok(Value::INT(comp)) = live.get("LapCompleted") { s.debug_lap_completed = comp; }
                            if let Ok(Value::FLOAT(curr_time)) = live.get("LapCurrentLapTime") { s.debug_lap_current_time = curr_time; }
                            if let Ok(Value::FLOAT(last_t)) = live.get("LapLastLapTime") { s.debug_lap_last_time = last_t; }
                            if let Ok(Value::FLOAT(best_t)) = live.get("LapBestLapTime") { s.debug_lap_best_time = best_t; }
                            if let Ok(Value::INT(best_lap)) = live.get("LapBestLap") { s.debug_lap_best_lap_num = best_lap; }
                            if let Ok(Value::FLOAT(dist)) = live.get("LapDist") { s.debug_lap_dist = dist; }
                            if let Ok(Value::FLOAT(dist_pct)) = live.get("LapDistPct") { s.player_lap_distance_pct = dist_pct; }
                            let is_on_track = match live.get("IsOnTrack") {
                                Ok(Value::BOOL(b)) => b,
                                Ok(Value::INT(i)) => i != 0,
                                Ok(Value::BITS(b)) => b != 0,
                                _ => false,
                            };
                            let cam_state = match live.get("CamCameraState") {
                                Ok(Value::BITS(b)) => b,
                                Ok(Value::INT(i)) => i as u32,
                                _ => 0,
                            };
                            let is_replay = (cam_state & 0x0002) != 0; // irsdk_IsCamReplay = 0x0002

                            s.debug_is_on_track = is_on_track;
                            if let Ok(Value::FLOAT(spd)) = live.get("Speed") { s.debug_speed = spd * 2.23694; } // MPH
                            if let Ok(Value::FLOAT(rpm)) = live.get("RPM") { s.debug_rpm = rpm; }
                            if let Ok(Value::INT(gear)) = live.get("Gear") { s.debug_gear = gear; }
                            if let Ok(Value::FLOAT(thr)) = live.get("Throttle") { s.debug_throttle = thr; }
                            if let Ok(Value::FLOAT(brk)) = live.get("Brake") { s.debug_brake = brk; }
                            let cur_session_time = match live.get("SessionTime") {
                                Ok(Value::DOUBLE(stime)) => stime,
                                Ok(Value::FLOAT(stime)) => stime as f64,
                                _ => s.debug_session_time,
                            };
                            let cur_session_num = match live.get("SessionNum") {
                                Ok(Value::INT(n)) => n,
                                _ => -1,
                            };
                            let cur_session_state = match live.get("SessionState") {
                                Ok(Value::INT(st)) => st,
                                _ => 0,
                            };
                            let cur_unique_id = match live.get("SessionUniqueID") {
                                Ok(Value::INT(id)) => id,
                                _ => -1,
                            };

                            // Session Phase, Subsession, and Restart Transition Detector
                            let transition_detected = s.check_session_transition(
                                cur_session_num,
                                cur_unique_id,
                                cur_session_state,
                                cur_session_time,
                                is_replay,
                            );

                            if let Ok(Value::INT(wetness)) = live.get("TrackWetness") {
                                if wetness >= 0 && wetness <= 6 && wetness != s.track_wetness {
                                    s.track_wetness = wetness;
                                    let raw_rubber = s.subsession_rubber_states.get(&s.session_num).cloned().unwrap_or_default();
                                    s.track_state = format_track_state(&raw_rubber, s.track_wetness);
                                }
                            }

                            let flags_opt = match live.get("SessionFlags") {
                                Ok(Value::BITS(flags)) => Some(flags),
                                Ok(Value::INT(flags)) => Some(flags as u32),
                                _ => None,
                            };

                            // Read Live Driver Arrays
                            let pit_road = match live.get("CarIdxOnPitRoad") { Ok(Value::BoolVec(v)) => v, _ => vec![false; 64] };
                            let laps_comp = match live.get("CarIdxLapCompleted") { Ok(Value::IntVec(v)) => v, _ => vec![0; 64] };
                            let last_laps = match live.get("CarIdxLastLapTime") { Ok(Value::FloatVec(v)) => v, _ => vec![0.0; 64] };
                            let fast_laps = match live.get("CarIdxFastestLapTime") { Ok(Value::FloatVec(v)) => v, _ => vec![0.0; 64] };
                            let positions = match live.get("CarIdxPosition") { Ok(Value::IntVec(v)) => v, _ => vec![0; 64] };
                            let surfaces = match live.get("CarIdxTrackSurface") { Ok(Value::IntVec(v)) => v, _ => vec![0; 64] };
                            let lap_dist_pcts = match live.get("CarIdxLapDistPct") { Ok(Value::FloatVec(v)) => v, _ => vec![0.0; 64] };

                            let player_live_pressures = [
                                get_sample_f32(&live, "LFpressure"),
                                get_sample_f32(&live, "RFpressure"),
                                get_sample_f32(&live, "LRpressure"),
                                get_sample_f32(&live, "RRpressure"),
                            ];

                            let frame_input = TelemetryFrame {
                                cur_session_state,
                                flags: flags_opt,
                                cur_session_time,
                                current_fuel_sample,
                                player_live_pressures,
                                is_on_track,
                                is_replay,
                                pit_road: &pit_road,
                                laps_comp: &laps_comp,
                                last_laps: &last_laps,
                                fast_laps: &fast_laps,
                                positions: &positions,
                                surfaces: &surfaces,
                                lap_dist_pcts: &lap_dist_pcts,
                            };

                            let state_changed = s.process_telemetry_frame(&frame_input, Some(&live), transition_detected);

                            if state_changed || version_changed {
                                ctx_clone.request_repaint();
                            }
                        }

                        drop(conn);
                        {
                            let mut s = state_clone.lock().unwrap();
                            s.is_connected = false;
                            s.player_is_in_car = false;
                            s.needs_new_connection_reset = true;
                            s.track_name = "Waiting for connection...".to_string();
                        }
                        ctx_clone.request_repaint();
                        thread::sleep(std::time::Duration::from_millis(200));
                    },
                    None => {
                        {
                            let mut s = state_clone.lock().unwrap();
                            s.is_connected = false;
                            s.player_is_in_car = false;
                        }
                        ctx_clone.request_repaint();
                        thread::sleep(std::time::Duration::from_millis(200));
                    }
                }
            }
        });
        
        let mut updater = updater::UpdateChecker::new();
        if std::env::args().any(|a| a == updater::UPDATE_CANCELLED_ARG) {
            // Relaunched by the update helper after the UAC prompt was declined or the copy failed
            updater.status = updater::UpdateStatus::Error(
                "Update cancelled: administrator permission is required because this copy is in a protected folder. \
                 Install with SearchForSpeedAnalyzer-Setup.exe to get updates without admin prompts."
                    .to_string(),
            );
        } else if settings.updates.check_on_startup {
            updater.trigger_check(settings.updates.include_prereleases);
        }

        Self { 
            state,
            settings,
            logo_texture,
            show_settings_modal: false,
            show_telemetry_debugger: false,
            selected_driver_car_idx: None,
            selected_stint_num: None,
            show_stint_analyzer_modal: false,
            stint_analyzer_tab: StintAnalyzerTab::SingleStint,
            stint_compare_slots: Vec::new(),
            show_stint_comparison_window: false,
            custom_bucket_input: String::new(),
            toast_message: None,
            grid_sort_key: GridSortKey::Position,
            grid_sort_dir: SortDirection::Ascending,
            driver_search_query: String::new(),
            updater,
            show_release_notes_modal: false,
            release_notes_custom_content: None,
            preview_overlay_hud: false,
            preview_tire_popup_hud: false,
            last_seen_tire_popup_seq: 0,
            tire_popup_active_until: None,
            tire_popup_stint_num: 1,
            main_overlay_hwnd: (0, 0),
            tire_popup_hwnd: (0, 0),
            ibt_lab: ibt_ui::IbtTelemetryLabState::default(),
            stint_db: stint_db::StintDatabase::load(),
            show_stint_db_modal: false,
            stint_db_filter_current_track: true,
            stint_db_search_query: String::new(),
        }
    }

    #[cfg(test)]
    pub fn new_for_test(state: Arc<Mutex<WarRoomState>>) -> Self {
        Self {
            state,
            settings: AppSettings::default(),
            logo_texture: None,
            show_settings_modal: false,
            show_telemetry_debugger: false,
            selected_driver_car_idx: None,
            selected_stint_num: None,
            show_stint_analyzer_modal: false,
            stint_analyzer_tab: StintAnalyzerTab::SingleStint,
            stint_compare_slots: Vec::new(),
            show_stint_comparison_window: false,
            custom_bucket_input: String::new(),
            toast_message: None,
            grid_sort_key: GridSortKey::Position,
            grid_sort_dir: SortDirection::Ascending,
            driver_search_query: String::new(),
            updater: updater::UpdateChecker::new(),
            show_release_notes_modal: false,
            release_notes_custom_content: None,
            preview_overlay_hud: false,
            preview_tire_popup_hud: false,
            last_seen_tire_popup_seq: 0,
            tire_popup_active_until: None,
            tire_popup_stint_num: 1,
            main_overlay_hwnd: (0, 0),
            tire_popup_hwnd: (0, 0),
            ibt_lab: ibt_ui::IbtTelemetryLabState::default(),
            stint_db: stint_db::StintDatabase::default(),
            show_stint_db_modal: false,
            stint_db_filter_current_track: true,
            stint_db_search_query: String::new(),
        }
    }

}

impl eframe::App for WarRoomApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.settings.bg_color().to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.updater.poll();
        let primary = self.settings.primary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let green = self.settings.status_green();
        
        let s = self.state.lock().unwrap();
        let connected = s.is_connected;
        let track_name = s.track_name.clone();
        let track_len = s.track_length.clone();
        let car_name = s.car_name.clone();
        let sess_type = s.session_type.clone();
        let track_state = s.track_state.clone();
        let session_id = s.session_id;
        let sof = s.sof;
        let current_state = s.clone_state_for_ui();
        
        let player = s.drivers.get(&s.player_car_idx);
        let player_pos = player.map_or(1, |p| p.position);
        let total_drivers = s.drivers.values().filter(|d| d.position > 0).count().max(1);
        let player_laps = player.map_or(0, |p| p.total_laps_completed);

        if let Some(sel_idx) = self.selected_driver_car_idx {
            match s.drivers.get(&sel_idx) {
                None => {
                    self.selected_driver_car_idx = None;
                    self.selected_stint_num = None;
                    self.show_stint_analyzer_modal = false;
                }
                Some(drv) => {
                    if let Some(sel_stint) = self.selected_stint_num {
                        if !drv.stints.iter().any(|st| st.stint_number == sel_stint) {
                            self.selected_stint_num = None;
                        }
                    }
                }
            }
        } else if let Some(sel_stint) = self.selected_stint_num {
            let player_has_stint = player.map_or(false, |drv| drv.stints.iter().any(|st| st.stint_number == sel_stint));
            if !player_has_stint {
                self.selected_stint_num = None;
            }
        }
        
        let current_metric = self.settings.use_metric;
        let air_str = convert_unit(&s.air_temp_raw, current_metric);
        let track_str = convert_unit(&s.track_temp_raw, current_metric);
        let wind_spd_str = convert_unit(&s.wind_speed_raw, current_metric);
        let wind_dir_str = format_wind_dir(&s.wind_dir_raw);
        let rh_str = convert_unit(&s.relative_humidity_raw, current_metric);
        let sky_str = if s.weather_type.is_empty() || s.weather_type == "..." {
            "Clear".to_string()
        } else {
            s.weather_type.clone()
        };

        drop(s);

        // Automatically persist completed live player stints (1+ laps) into the local Stint Database
        self.stint_db.sync_player_stints_from_state(&current_state);

        // --- SINGLE REFINED MOTORSPORT HEADER BAR (Design A) ---
        let header_frame = egui::Frame::none()
            .fill(self.settings.bg_color())
            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
            .stroke(egui::Stroke::new(1.0, self.settings.divider_color()));

        egui::TopBottomPanel::top("top_header").exact_height(60.0).frame(header_frame).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Brand Logo (Top-Left)
                if let Some(tex) = &self.logo_texture {
                    let h = 36.0;
                    let aspect = tex.size_vec2().x / tex.size_vec2().y.max(1.0);
                    ui.add(egui::Image::from_texture((tex.id(), egui::vec2(h * aspect, h))));
                } else {
                    ui.label(egui::RichText::new("THE SEARCH FOR SPEED").font(display_font(16.0)).color(white));
                }
                ui.add_space(10.0);

                // Segmented View Switcher (Live Stints: Cockpit | Grid) + Prominent Standalone (.IBT Telemetry Lab)
                let is_cockpit = self.settings.view_mode == ActiveViewMode::CockpitEngineer;
                let is_grid = self.settings.view_mode == ActiveViewMode::GridTable;
                let is_ibt = self.settings.view_mode == ActiveViewMode::IbtTelemetry;
                egui::Frame::none()
                    .fill(elevated)
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(3.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        let c_btn = egui::Button::new(
                            egui::RichText::new("Cockpit")
                                .size(12.5)
                                .color(if is_cockpit { white } else { muted })
                        )
                        .fill(if is_cockpit { self.settings.panel_color() } else { egui::Color32::TRANSPARENT })
                        .rounding(egui::Rounding::same(6.0));
                        if ui.add(c_btn).clicked() {
                            self.settings.view_mode = ActiveViewMode::CockpitEngineer;
                        }

                        let g_btn = egui::Button::new(
                            egui::RichText::new("Grid")
                                .size(12.5)
                                .color(if is_grid { white } else { muted })
                        )
                        .fill(if is_grid { self.settings.panel_color() } else { egui::Color32::TRANSPARENT })
                        .rounding(egui::Rounding::same(6.0));
                        if ui.add(g_btn).clicked() {
                            self.settings.view_mode = ActiveViewMode::GridTable;
                        }
                    });

                // Prominent Separate Mode Button for Offline .IBT Telemetry Lab (distinct from Live Stint Views)
                let ibt_stroke = if is_ibt {
                    egui::Stroke::new(1.5, primary)
                } else {
                    egui::Stroke::new(1.0, with_alpha(self.settings.secondary_accent(), 140))
                };
                let ibt_fill = if is_ibt {
                    with_alpha(primary, 32)
                } else {
                    with_alpha(self.settings.secondary_accent(), 28)
                };
                let ibt_btn = egui::Button::new(
                    egui::RichText::new("ðŸ“Š .IBT Telemetry Lab")
                        .size(12.5)
                        .color(if is_ibt { primary } else { white })
                        .strong(),
                )
                .fill(ibt_fill)
                .stroke(ibt_stroke)
                .rounding(egui::Rounding::same(8.0));
                if ui
                    .add(ibt_btn)
                    .on_hover_text("Open Standalone Offline .IBT Telemetry Analysis Lab (60Hz Traces, Ride Height & Rake, Shock Histograms, Tire Spreads)")
                    .clicked()
                {
                    self.settings.view_mode = ActiveViewMode::IbtTelemetry;
                }

                ui.add_space(4.0);

                // Context Pills: Track, Car/Session, Position, Full Weather (with Wind Direction & Humidity)
                let pill_bg = elevated;
                let pill_margin = egui::Margin::symmetric(10.0, 5.0);
                let pill_round = egui::Rounding::same(16.0);

                let track_len_str = convert_unit(&track_len, current_metric);
                egui::Frame::none().fill(pill_bg).rounding(pill_round).inner_margin(pill_margin).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(egui::RichText::new(&track_name).color(white).size(12.5));
                        if !track_len_str.is_empty() && track_len_str != "—" {
                            ui.label(egui::RichText::new(format!("· {}", track_len_str)).color(muted).size(12.0));
                        }
                    });
                });

                egui::Frame::none().fill(pill_bg).rounding(pill_round).inner_margin(pill_margin).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(egui::RichText::new(&car_name).color(white).size(12.5));
                        let sess_info = if !track_state.is_empty() && track_state != "—" && track_state != "..." {
                            format!("· {} L{} · {}", sess_type, player_laps, track_state)
                        } else {
                            format!("· {} L{}", sess_type, player_laps)
                        };
                        ui.label(egui::RichText::new(sess_info).color(muted).size(12.0));
                    });
                });

                egui::Frame::none()
                    .fill(with_alpha(primary, 30))
                    .rounding(pill_round)
                    .inner_margin(pill_margin)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new(format!("P{} / {}", player_pos, total_drivers)).color(primary).strong().size(12.5));
                    });

                let (header_wind_spd, header_wind_unit) = parse_wind_speed(&current_state.wind_speed_raw, current_metric);
                let header_wind_rad = parse_wind_direction_rad(&current_state.wind_dir_raw);
                let header_rel_wind = header_wind_rad.map(|w_rad| {
                    calculate_relative_wind_rad(w_rad, current_state.player_yaw_north)
                });
                egui::Frame::none().fill(pill_bg).rounding(pill_round).inner_margin(pill_margin).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(egui::RichText::new(format!("☀ {} · Air {} · Track {} · Wind {}", sky_str, air_str, track_str, wind_spd_str)).color(muted).size(12.0));
                        draw_wind_indicator(ui, header_wind_spd, header_wind_unit, header_rel_wind, &wind_dir_str, 0.72);
                        ui.label(egui::RichText::new(format!("· RH {}", rh_str)).color(muted).size(12.0));
                    });
                });

                // Right-side controls: Live pill, Overlay, Overflow menu (…), Settings (⚙)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized(egui::vec2(32.0, 30.0), egui::Button::new(egui::RichText::new("⚙").color(white).size(14.0)).fill(elevated)).on_hover_text("Settings & Layouts").clicked() {
                        self.show_settings_modal = true;
                    }

                    ui.menu_button(egui::RichText::new("…").color(white).size(15.0), |ui| {
                        ui.set_min_width(210.0);
                        if ui.button("ðŸ“– What's New & Release Notes").clicked() {
                            self.show_release_notes_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Stint Telemetry Analyzer").clicked() {
                            self.stint_analyzer_tab = StintAnalyzerTab::SingleStint;
                            self.show_stint_analyzer_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("⚖ Compare Stints (Side-by-Side)").clicked() {
                            self.ensure_compare_slots_populated(&current_state);
                            self.stint_analyzer_tab = StintAnalyzerTab::Compare;
                            self.show_stint_analyzer_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("ðŸ“‚ Saved Stints Database (Re-Import / Share)").clicked() {
                            self.show_stint_db_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("ðŸ“Š Offline .IBT Telemetry Lab").clicked() {
                            self.settings.view_mode = ActiveViewMode::IbtTelemetry;
                            ui.close_menu();
                        }
                        let dbg_label = if self.show_telemetry_debugger { "✓ Live Telemetry Debugger" } else { "Live Telemetry Debugger" };
                        if ui.button(dbg_label).clicked() {
                            self.show_telemetry_debugger = !self.show_telemetry_debugger;
                            ui.close_menu();
                        }
                        ui.separator();
                        let unit_label = if current_metric { "Switch to Imperial (mph / °F)" } else { "Switch to Metric (km/h / °C)" };
                        if ui.button(unit_label).clicked() {
                            self.settings.use_metric = !current_metric;
                            self.settings.save();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("ðŸ”„ Reset / Clear Session Stints").on_hover_text("Cleanly resets stint counts, lap numbers, and best lap times for a new session or run, while preserving baseline fuel burn rate.").clicked() {
                            let mut s = self.state.lock().unwrap();
                            s.reset_session_data(true);
                            self.selected_driver_car_idx = None;
                            self.selected_stint_num = None;
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(egui::RichText::new(format!("Session ID: {} · SOF: {}", session_id, sof)).color(muted).size(11.0));
                    });

                    let is_overlay_open = self.settings.overlay.enabled;
                    let overlay_btn = egui::Button::new(
                        egui::RichText::new(if is_overlay_open { "Overlay · On" } else { "Overlay" })
                            .size(12.5)
                            .color(if is_overlay_open { primary } else { white })
                    )
                    .fill(if is_overlay_open { with_alpha(primary, 30) } else { elevated });
                    if ui.add_sized(egui::vec2(88.0, 30.0), overlay_btn).clicked() {
                        self.settings.overlay.enabled = !self.settings.overlay.enabled;
                        self.settings.save();
                    }

                    let (live_bg, live_fg, live_txt) = if connected {
                        (with_alpha(green, 28), green, "Live")
                    } else {
                        (elevated, muted, "Standby")
                    };
                    egui::Frame::none()
                        .fill(live_bg)
                        .rounding(pill_round)
                        .inner_margin(egui::Margin::symmetric(10.0, 5.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                                ui.painter().circle_filled(rect.center(), 4.0, live_fg);
                                ui.label(egui::RichText::new(live_txt).color(live_fg).size(12.0).strong());
                            });
                        });
                });
            });
        });

        // --- OPTIONAL UPDATE ALERT BANNER ---
        let show_banner = match &self.updater.status {
            updater::UpdateStatus::UpdateAvailable(info) => {
                self.settings.updates.ignored_version.as_deref() != Some(&info.tag_name)
            }
            updater::UpdateStatus::Downloading { .. } => true,
            updater::UpdateStatus::ReadyToRestart { .. } => true,
            updater::UpdateStatus::Error(_) => true,
            _ => false,
        };

        if show_banner {
            egui::TopBottomPanel::top("update_banner")
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(18, 32, 54))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(38, 85, 150)))
                        .inner_margin(egui::Margin::symmetric(14.0, 7.0))
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        let updater_status = self.updater.status.clone();
                    match &updater_status {
                            updater::UpdateStatus::UpdateAvailable(info) => {
                                let info_clone = info.clone();
                                let icon = if info_clone.is_prerelease { "ðŸ“¢" } else { "ðŸš€" };
                                let title = if info_clone.is_prerelease {
                                    format!("{} Pre-release Available: {}", icon, info_clone.tag_name)
                                } else {
                                    format!("{} New Update Available: {}", icon, info_clone.tag_name)
                                };
                                ui.label(egui::RichText::new(title).color(egui::Color32::WHITE).strong().size(12.5));
                                if !info_clone.name.is_empty() && info_clone.name != info_clone.tag_name {
                                    ui.label(egui::RichText::new(format!("({})", info_clone.name)).color(muted).size(12.0));
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button(egui::RichText::new("Ã— Dismiss").color(muted).size(11.5)).clicked() {
                                        self.updater.dismiss();
                                    }
                                    if ui.button(egui::RichText::new("Skip this version").color(muted).size(11.5)).clicked() {
                                        self.settings.updates.ignored_version = Some(info_clone.tag_name.clone());
                                        self.settings.save();
                                        self.updater.dismiss();
                                    }
                                    if ui.button(egui::RichText::new("What's New ðŸ“–").color(white).size(12.0)).clicked() {
                                        self.show_release_notes_modal = true;
                                    }
                                    let update_btn = egui::Button::new(
                                        egui::RichText::new("Update Now ⚡").color(egui::Color32::BLACK).strong().size(12.0)
                                    ).fill(primary);
                                    if ui.add(update_btn).clicked() {
                                        self.updater.trigger_download_and_install(&info_clone);
                                    }
                                });
                            }
                            updater::UpdateStatus::Downloading { tag, percent, downloaded_bytes, total_bytes } => {
                                ui.spinner();
                                let mb_down = *downloaded_bytes as f32 / (1024.0 * 1024.0);
                                let mb_tot = *total_bytes as f32 / (1024.0 * 1024.0);
                                let text = if *total_bytes > 0 {
                                    format!("ðŸ“¥ Downloading {}: {:.0}% ({:.1} / {:.1} MB)", tag, percent, mb_down, mb_tot)
                                } else {
                                    format!("ðŸ“¥ Downloading {}: {:.1} MB", tag, mb_down)
                                };
                                ui.label(egui::RichText::new(text).color(white).strong().size(12.5));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let prog = if *total_bytes > 0 { *percent / 100.0 } else { 0.5 };
                                    ui.add(egui::ProgressBar::new(prog).desired_width(180.0).show_percentage());
                                });
                            }
                            updater::UpdateStatus::ReadyToRestart { tag } => {
                                ui.label(egui::RichText::new(format!("✅ Update {} downloaded and ready!", tag)).color(self.settings.status_green()).strong().size(12.5));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button(egui::RichText::new("Later").color(muted).size(11.5)).clicked() {
                                        self.updater.dismiss();
                                    }
                                    let restart_btn = egui::Button::new(
                                        egui::RichText::new("Restart to Apply ðŸ”„").color(egui::Color32::BLACK).strong().size(12.0)
                                    ).fill(self.settings.status_green());
                                    if ui.add(restart_btn).clicked() {
                                        if let Err(e) = updater::apply_update_and_restart() { self.updater.status = updater::UpdateStatus::Error(e); }
                                    }
                                });
                            }
                            updater::UpdateStatus::Error(err) => {
                                ui.label(egui::RichText::new(format!("⚠ Update notice: {}", err)).color(egui::Color32::from_rgb(255, 120, 120)).size(12.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button(egui::RichText::new("Ã— Dismiss").color(muted).size(11.5)).clicked() {
                                        self.updater.dismiss();
                                    }
                                });
                            }
                            _ => {}
                        }
                    });
                });
        }

        // --- MAIN WORKSPACE ---
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.settings.bg_color()).inner_margin(egui::Margin::same(12.0)))
            .show(ctx, |ui| {
                match self.settings.view_mode {
                    ActiveViewMode::CockpitEngineer => {
                        self.render_cockpit_view(ui, &current_state);
                    }
                    ActiveViewMode::GridTable => {
                        self.render_grid_table_view(ui, &current_state);
                    }
                    ActiveViewMode::IbtTelemetry => {
                        self.ibt_lab.render(ctx, ui, &self.settings, &mut self.toast_message);
                    }
                }
            });

        self.render_settings_modal(ctx);
        self.render_telemetry_debugger(ctx, &current_state);
        self.render_stint_analyzer_modal(ctx, &current_state);
        self.render_stint_comparison_window(ctx, &current_state);
        self.render_stint_db_modal(ctx, &current_state);
        self.render_release_notes_modal(ctx);
        self.render_floating_overlay_hud(ctx, &current_state);
        self.render_tire_popup_overlay(ctx, &current_state);

        // Toast Notification Overlay
        if let Some((msg, time)) = &self.toast_message {
            if time.elapsed().as_secs_f32() < 3.0 {
                egui::Area::new(egui::Id::new("toast_notification_area"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, [-24.0, -24.0])
                    .order(egui::Order::Tooltip)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(self.settings.panel_color())
                            .stroke(egui::Stroke::new(1.0, green))
                            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                            .rounding(8.0)
                            .shadow(egui::epaint::Shadow { offset: egui::vec2(0.0, 4.0), blur: 12.0, spread: 0.0, color: egui::Color32::from_black_alpha(160) })
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(msg).color(white).strong().size(12.5));
                                });
                            });
                    });
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        // Keep UI smoothly repainting and responsive
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::FuelHeroMetric;

    #[test]
    fn test_stint_averages_and_sums() {
        let mut stint = StintData::default();
        stint.stint_number = 1;
        stint.setup_name = "PB RollaRink R1.03".to_string();
        stint.lap_times = vec![
            15.093, 14.801, 14.682, 14.774, 14.755,
            14.755, 14.870, 14.824, 15.040, 15.000,
            14.981, 15.092, 15.142, 15.152, 15.119,
            15.133, 15.182, 15.141, 15.173, 15.232,
        ];
        stint.laps_completed = 20;

        // Test 10-lap sum and avg
        let sum_10 = stint.sum_first_n(10);
        assert!((sum_10 - 148.594).abs() < 0.001, "Expected ~148.594, got {}", sum_10);
        let avg_10 = stint.avg_first_n(10);
        assert!((avg_10 - 14.8594).abs() < 0.001, "Expected ~14.8594, got {}", avg_10);

        // Test 11-20 segment avg
        let seg_11_20 = stint.avg_segment(11, 20);
        assert!((seg_11_20 - 15.1347).abs() < 0.001, "Expected ~15.1347, got {}", seg_11_20);

        // Test 20-lap total
        let sum_20 = stint.sum_first_n(20);
        assert!((sum_20 - 299.941).abs() < 0.001, "Expected ~299.941, got {}", sum_20);
    }

    #[test]
    fn test_format_session_type() {
        assert_eq!(format_session_type("Open Practice"), "PRACTICE");
        assert_eq!(format_session_type("Lone Qualifying"), "QUALIFYING");
        assert_eq!(format_session_type("Qualify"), "QUALIFYING");
        assert_eq!(format_session_type("Warmup"), "WARMUP");
        assert_eq!(format_session_type("Race"), "RACE");
        assert_eq!(format_session_type("Feature Race"), "RACE");
        assert_eq!(format_session_type(""), "SESSION");
    }

    #[test]
    fn test_session_transition_reset_and_archive() {
        let mut state = WarRoomState::default();
        state.session_num = 0;
        state.session_type = "PRACTICE".to_string();

        let mut driver = DriverState::default();
        driver.car_idx = 0;
        driver.driver_name = "Player 1".to_string();
        driver.car_num = "42".to_string();

        // Simulate Practice Stint with 10 laps and 2.50L burn per lap
        let mut stint = StintData::default();
        stint.stint_number = 1;
        stint.laps_completed = 10;
        stint.fast_lap = 14.850;
        stint.lap_fuel_burns = vec![2.50, 2.48, 2.52, 2.50, 2.50];
        driver.stints.push(stint);
        driver.total_laps_completed = 10;
        driver.last_processed_lap = 10;
        driver.last_known_lap_time = 14.900;
        state.drivers.insert(0, driver);

        // Verify fuel burn calculated in practice
        let practice_burn = state.drivers.get(&0).unwrap().estimated_fuel_burn_ltr();
        assert!(practice_burn > 2.45 && practice_burn < 2.55, "Burn should be ~2.50L");

        // Transition: Practice -> Qualifying
        state.reset_session_data(false);
        state.session_num = 1;
        state.session_type = "QUALIFYING".to_string();

        // 1. Verify Practice was archived
        assert_eq!(state.session_history.len(), 1);
        assert_eq!(state.session_history[0].session_num, 0);
        assert_eq!(state.session_history[0].session_type, "PRACTICE");

        // 2. Verify active driver state is clean for new session
        let drv_qual = state.drivers.get(&0).unwrap();
        assert_eq!(drv_qual.stints.len(), 0, "Stints should be empty for new session");
        assert_eq!(drv_qual.total_laps_completed, 0);
        assert_eq!(drv_qual.last_processed_lap, 0);
        assert_eq!(drv_qual.last_known_lap_time, 0.0);

        // 3. Verify historical fuel burn rate is preserved as fallback
        assert!((drv_qual.historical_fuel_burn_rate - practice_burn).abs() < 0.001);
        assert_eq!(drv_qual.estimated_fuel_burn_ltr(), practice_burn, "Should use historical baseline burn rate");

        // Simulate Qualifying Stint (low fuel flyer)
        let drv_qual_mut = state.drivers.get_mut(&0).unwrap();
        let mut q_stint = StintData::default();
        q_stint.stint_number = 1;
        q_stint.laps_completed = 2;
        q_stint.fast_lap = 14.200; // Fast qualifying flyer
        q_stint.lap_fuel_burns = vec![2.40, 2.40];
        drv_qual_mut.stints.push(q_stint);
        drv_qual_mut.total_laps_completed = 2;

        // Transition: Qualifying -> Race
        state.reset_session_data(false);
        state.session_num = 2;
        state.session_type = "RACE".to_string();

        // 4. Verify Qualifying was archived
        assert_eq!(state.session_history.len(), 2);
        assert_eq!(state.session_history[1].session_num, 1);
        assert_eq!(state.session_history[1].session_type, "QUALIFYING");

        // 5. Verify Race starts fresh: S1, no 14.200 qualifying lap bleeding in
        let drv_race = state.drivers.get(&0).unwrap();
        assert_eq!(drv_race.stints.len(), 0);
        assert_eq!(drv_race.total_laps_completed, 0);

        // New Race stint starts as S1
        let drv_race_mut = state.drivers.get_mut(&0).unwrap();
        let mut r_stint = StintData::default();
        r_stint.stint_number = drv_race_mut.stints.len() as i32 + 1;
        assert_eq!(r_stint.stint_number, 1, "Race should start cleanly at Stint S1");
        r_stint.laps_completed = 1;
        r_stint.fast_lap = 15.100; // Race lap pace
        drv_race_mut.stints.push(r_stint);

        assert_eq!(drv_race_mut.stints[0].fast_lap, 15.100, "Fast lap in race should NOT be the 14.200 qualifying lap!");
    }

    #[test]
    fn test_format_track_state() {
        assert_eq!(format_track_state("Moderate", 1), "Moderate Rubber");
        assert_eq!(format_track_state("Clean", 1), "Clean Track");
        assert_eq!(format_track_state("Heavy", 1), "Heavy Rubber");
        assert_eq!(format_track_state("Light", 3), "Light Rubber (Very Light Wet)");
        assert_eq!(format_track_state("Carried over", 5), "Carried Over (Moderate Wet)");
        assert_eq!(format_track_state("", 0), "Clean Track");
        assert_eq!(format_track_state("...", 1), "Clean Track");
        assert_eq!(format_track_state("high rubber", 4), "High Rubber (Light Wet)");
    }

    #[test]
    fn test_clean_setup_name() {
        assert_eq!(clean_setup_name(r"C:\Users\peebs\Documents\iRacing\setups\mustanggt3\Atlanta_Race_v2.sto"), "Atlanta_Race_v2");
        assert_eq!(clean_setup_name("mustanggt3/Atlanta_Qual.STO"), "Atlanta_Qual");
        assert_eq!(clean_setup_name("baseline.sto"), "baseline");
        assert_eq!(clean_setup_name("baseline"), "baseline");
        assert_eq!(clean_setup_name(""), "");
        assert_eq!(clean_setup_name("—"), "");
    }

    #[test]
    fn test_parse_iracing_session_yaml_resilient() {
        // Test sample YAML containing 'ResultsPositions: ~' which causes iracing-0.4.1 to crash
        let sample_yaml = r#"
WeekendInfo:
  TrackDisplayName: "Atlanta Motor Speedway - Oval"
  TrackLength: "1.54 mi"
  TrackWeatherType: "Realistic"
  TrackAirTemp: "72.4 F"
  TrackSurfaceTemp: "85.1 F"
  TrackWindVel: "5.2 mph"
  TrackWindDir: "1.23 rad"
  WeekendOptions:
    WindDirection: "NW"
    RelativeHumidity: "45%"
  SessionID: 1234567

SessionInfo:
  Sessions:
    - SessionNum: 0
      SessionType: "Practice"
      SessionTrackRubberState: "Moderately Grooved"
      ResultsPositions: ~
    - SessionNum: 1
      SessionType: "Race"
      SessionTrackRubberState: "Heavily Grooved"
      ResultsPositions: ~

DriverInfo:
  DriverCarIdx: 1
  DriverCarFuelMaxLtr: 120.0
  DriverSetupName: "mustanggt3/Atlanta_Race_v2.sto"
  Drivers:
    - CarIdx: 0
      UserName: "Pace Car"
      CarNumber: "0"
      IRating: 0
    - CarIdx: 1
      UserName: "PB Henry"
      CarNumber: "95"
      IRating: 2450
      CarScreenName: "Ford Mustang GT3"
    - CarIdx: 2
      UserName: "Ben Tracy"
      CarNumber: 33
      IRating: 1980
      CarScreenName: "BMW M4 GT3"
"#;

        let mut state = WarRoomState::default();
        let success = parse_iracing_session_yaml(sample_yaml, &mut state);
        assert!(success, "parse_iracing_session_yaml should succeed with unit ResultsPositions: ~");

        // Verify Weekend & Session Info
        assert_eq!(state.track_name, "Atlanta Motor Speedway - Oval");
        assert_eq!(state.track_length, "1.54 mi");
        assert_eq!(state.weather_type, "Realistic");
        assert_eq!(state.air_temp_raw, "72.4 F");
        assert_eq!(state.track_temp_raw, "85.1 F");
        assert_eq!(state.wind_speed_raw, "5.2 mph");
        assert_eq!(state.wind_dir_raw, "NW");
        assert_eq!(state.relative_humidity_raw, "45%");
        assert_eq!(state.session_id, 1234567);

        // Verify Subsessions
        assert_eq!(state.subsession_types.get(&0).unwrap(), "Practice");
        assert_eq!(state.subsession_types.get(&1).unwrap(), "Race");

        // Verify Driver & Setup
        assert_eq!(state.player_car_idx, 1);
        assert_eq!(state.car_name, "Ford Mustang GT3");
        assert_eq!(state.player_setup_name, "Atlanta_Race_v2");
        assert_eq!(state.player_fuel_capacity, 120.0);

        // Verify Driver Registry
        assert!(state.drivers.contains_key(&1));
        let pb = state.drivers.get(&1).unwrap();
        assert_eq!(pb.driver_name, "PB Henry");
        assert_eq!(pb.car_num, "95");
        assert_eq!(pb.irating, "2450");
        assert_eq!(pb.setup_name, "Atlanta_Race_v2");

        assert!(state.drivers.contains_key(&2));
        let ben = state.drivers.get(&2).unwrap();
        assert_eq!(ben.driver_name, "Ben Tracy");
        assert_eq!(ben.car_num, "33");
        assert_eq!(ben.irating, "1980");
        assert_eq!(ben.setup_name, "Baseline");
    }

    #[test]
    fn test_driver_first_frame_initialization_and_live_attach() {
        // Test 1: Driver connected while in pit stall on Lap 0
        let mut d_pit = DriverState::default();
        d_pit.car_idx = 1;
        assert!(!d_pit.is_initialized);

        let is_pit_frame1 = true;
        let in_world_frame1 = true;
        let current_lap_0 = 0;

        // Simulate Frame 1 in pit
        if !d_pit.is_initialized {
            d_pit.is_initialized = true;
            d_pit.last_processed_lap = current_lap_0;
            d_pit.is_on_pit_road = is_pit_frame1;
            d_pit.is_in_world = in_world_frame1;
        }

        assert!(d_pit.is_initialized);
        assert!(d_pit.is_on_pit_road);
        assert!(d_pit.stints.is_empty(), "Car in pit should not start stint yet");

        // Simulate Frame 2 (car leaves pit onto track)
        let was_in_pit = d_pit.is_on_pit_road;
        let is_pit_frame2 = false;
        let in_world_frame2 = true;

        assert!(was_in_pit && !is_pit_frame2 && in_world_frame2, "Pit exit MUST be detected even on Lap 0");

        // Test 2: Driver connected while ALREADY driving on track (e.g. Lap 5)
        let mut d_track = DriverState::default();
        d_track.car_idx = 1;
        assert!(!d_track.is_initialized);

        let is_pit_track = false;
        let in_world_track = true;
        let current_lap_5 = 5;

        // Simulate Frame 1 on track
        if !d_track.is_initialized {
            d_track.is_initialized = true;
            d_track.last_processed_lap = current_lap_5;
            d_track.is_on_pit_road = is_pit_track;
            d_track.is_in_world = in_world_track;

            if !is_pit_track && in_world_track && d_track.stints.is_empty() {
                d_track.stints.push(StintData {
                    stint_number: 1,
                    start_lap: current_lap_5,
                    end_lap: current_lap_5,
                    out_lap_completed: true,
                    laps_completed: 0,
                    is_active: true,
                    ..Default::default()
                });
            }
        }

        assert!(d_track.is_initialized);
        assert_eq!(d_track.stints.len(), 1, "Connecting while on track should immediately start Stint 1");
        assert!(d_track.stints[0].is_active);
        assert_eq!(d_track.stints[0].start_lap, 5);
    }

    #[test]
    fn test_oval_race_green_flag_runs_and_caution_stint_reset_johnny_and_bob() {
        let mut state = WarRoomState::default();
        state.session_type = "RACE".to_string();
        state.is_oval = true;
        state.player_car_idx = 1; // Johnny is Player (CarIdx 1), Bob is CarIdx 2

        let mut pit_road = vec![false; 4];
        let mut laps_comp = vec![0; 4];
        let mut last_laps = vec![0.0; 4];
        let fast_laps = vec![0.0; 4];
        let positions = vec![0, 1, 2, 0];
        let surfaces = vec![0, 3, 3, 0]; // 1 and 2 on track
        let mut lap_dist_pcts = vec![0.0, 0.90, 0.89, 0.0];

        // 1. Pre-race parade lap (SessionState = STATE_PARADE_LAPS): No stints should start yet
        let frame_parade = TelemetryFrame {
            cur_session_state: STATE_PARADE_LAPS,
            flags: Some(FLAG_ONE_TO_GREEN),
            cur_session_time: 50.0,
            current_fuel_sample: 60.0,
            player_live_pressures: [200.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_parade, None, false);
        assert_eq!(state.caution_count, 0, "Pre-race parade lap must not count as a caution");
        assert!(state.drivers.get(&1).unwrap().stints.is_empty(), "No stint during pre-race parade laps");
        assert!(state.drivers.get(&2).unwrap().stints.is_empty(), "No stint during pre-race parade laps");

        // 2. Green flag waves! (SessionState = STATE_RACING, FLAG_GREEN)
        // Both Johnny (1) and Bob (2) start Stint S1
        let frame_green_start = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 60.0,
            current_fuel_sample: 59.5,
            player_live_pressures: [200.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_green_start, None, false);
        assert_eq!(state.drivers.get(&1).unwrap().stints.len(), 1);
        assert_eq!(state.drivers.get(&2).unwrap().stints.len(), 1);
        assert!(state.drivers.get(&1).unwrap().stints[0].is_active);
        assert!(state.drivers.get(&2).unwrap().stints[0].is_active);

        // 3. Complete Laps 1..=14 under Green Flag
        for lap in 1..=14 {
            laps_comp[1] = lap;
            laps_comp[2] = lap;
            last_laps[1] = 30.0 + (lap as f32) * 0.05; // Johnny's green lap time
            last_laps[2] = 30.1 + (lap as f32) * 0.05; // Bob's green lap time
            lap_dist_pcts[1] = 0.01;
            lap_dist_pcts[2] = 0.01;
            let frame_lap = TelemetryFrame {
                cur_session_state: STATE_RACING,
                flags: Some(FLAG_GREEN),
                cur_session_time: 60.0 + (lap as f64) * 30.0,
                current_fuel_sample: 59.5 - (lap as f32) * 0.5,
                player_live_pressures: [210.0; 4],
                is_on_track: true,
                is_replay: false,
                pit_road: &pit_road,
                laps_comp: &laps_comp,
                last_laps: &last_laps,
                fast_laps: &fast_laps,
                positions: &positions,
                surfaces: &surfaces,
                lap_dist_pcts: &lap_dist_pcts,
            };
            state.process_telemetry_frame(&frame_lap, None, false);
        }

        assert_eq!(state.drivers.get(&1).unwrap().stints[0].laps_completed, 14);
        assert_eq!(state.drivers.get(&2).unwrap().stints[0].laps_completed, 14);

        // 4. Caution flies on Lap 15 (while cars are halfway through Lap 15, laps_comp == 14)
        lap_dist_pcts[1] = 0.50;
        lap_dist_pcts[2] = 0.48;
        let frame_caution = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_CAUTION | FLAG_CAUTION_WAVING),
            cur_session_time: 495.0,
            current_fuel_sample: 52.0,
            player_live_pressures: [210.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_caution, None, false);
        assert!(state.is_under_caution);
        assert_eq!(state.caution_count, 1);
        // Stint S1 must immediately close for BOTH Johnny and Bob with 14 laps!
        assert!(!state.drivers.get(&1).unwrap().stints[0].is_active);
        assert!(!state.drivers.get(&2).unwrap().stints[0].is_active);
        assert_eq!(state.drivers.get(&1).unwrap().stints[0].laps_completed, 14);
        assert_eq!(state.drivers.get(&2).unwrap().stints[0].laps_completed, 14);

        // 5. Cars cross S/F to complete Lap 15 under Caution (slow 48.5s caution-entry lap)
        laps_comp[1] = 15;
        laps_comp[2] = 15;
        last_laps[1] = 48.5;
        last_laps[2] = 48.9;
        lap_dist_pcts[1] = 0.01;
        lap_dist_pcts[2] = 0.01;
        let frame_lap15 = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_CAUTION),
            cur_session_time: 530.0,
            current_fuel_sample: 51.8,
            player_live_pressures: [205.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_lap15, None, false);
        assert_eq!(state.drivers.get(&1).unwrap().stints.len(), 1, "Caution lap must not start a new stint");
        assert_eq!(state.drivers.get(&1).unwrap().stints[0].laps_completed, 14, "Caution lap 15 must not be counted in S1");
        assert_eq!(state.drivers.get(&2).unwrap().stints[0].laps_completed, 14, "Caution lap 15 must not be counted in S1");
        assert_eq!(state.caution_laps, 1);

        // 6. On Lap 16 (under Caution): Johnny (1) pits, Bob (2) stays out!
        pit_road[1] = true; // Johnny enters pit road
        pit_road[2] = false; // Bob stays out
        let frame_johnny_pits = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_CAUTION),
            cur_session_time: 560.0,
            current_fuel_sample: 60.0, // refueled
            player_live_pressures: [195.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_johnny_pits, None, false);

        // Johnny exits pit road on Lap 16 while STILL under caution
        pit_road[1] = false;
        laps_comp[1] = 16;
        laps_comp[2] = 16;
        last_laps[1] = 85.0;
        last_laps[2] = 70.0;
        let frame_johnny_pit_exit_caution = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_CAUTION | FLAG_ONE_TO_GREEN),
            cur_session_time: 600.0,
            current_fuel_sample: 59.8,
            player_live_pressures: [195.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_johnny_pit_exit_caution, None, false);
        assert_eq!(state.drivers.get(&1).unwrap().stints.len(), 1, "Pit exit under caution must wait for green flag to start S2");
        assert_eq!(state.drivers.get(&2).unwrap().stints.len(), 1);
        assert_eq!(state.caution_laps, 2);

        // 7. Coming to the Green Flag on Lap 17 (in the restart zone at 92% of track before S/F)
        lap_dist_pcts[1] = 0.92;
        lap_dist_pcts[2] = 0.93;
        let frame_green_restart = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 668.0,
            current_fuel_sample: 59.5,
            player_live_pressures: [198.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_green_restart, None, false);

        // Both Johnny (who pitted) AND Bob (who stayed out) MUST now have Stint S2 active!
        assert_eq!(state.drivers.get(&1).unwrap().stints.len(), 2, "Johnny should now be on Stint S2");
        assert_eq!(state.drivers.get(&2).unwrap().stints.len(), 2, "Bob (stayed out) should ALSO be on Stint S2");
        assert_eq!(state.drivers.get(&1).unwrap().stints[1].stint_number, 2);
        assert_eq!(state.drivers.get(&2).unwrap().stints[1].stint_number, 2);
        assert!(state.drivers.get(&1).unwrap().stints[1].is_active);
        assert!(state.drivers.get(&2).unwrap().stints[1].is_active);
        assert_eq!(state.drivers.get(&1).unwrap().stints[1].laps_completed, 0);
        assert_eq!(state.drivers.get(&2).unwrap().stints[1].laps_completed, 0);

        // 8. Two seconds later, Johnny and Bob cross Start/Finish to finish final pace lap 17
        // Simulate CarIdxLapCompleted incrementing on Frame A, and CarIdxLastLapTime (72.0s pace lap) arriving on Frame B!
        laps_comp[1] = 17;
        laps_comp[2] = 17;
        lap_dist_pcts[1] = 0.01;
        lap_dist_pcts[2] = 0.01;
        let frame_restart_sf_a = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 670.0,
            current_fuel_sample: 59.4,
            player_live_pressures: [198.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_restart_sf_a, None, false);

        // Frame B (33ms later): delayed pace lap time (72.0s) emitted by iRacing for Lap 17
        last_laps[1] = 72.1;
        last_laps[2] = 71.8;
        let frame_restart_sf_b = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 670.03,
            current_fuel_sample: 59.4,
            player_live_pressures: [198.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_restart_sf_b, None, false);

        // Pace lap 17 (72s) MUST NOT be counted in Stint S2 for either driver!
        assert_eq!(state.drivers.get(&1).unwrap().stints[1].laps_completed, 0, "Final pace lap must be dropped from S2");
        assert_eq!(state.drivers.get(&2).unwrap().stints[1].laps_completed, 0, "Final pace lap must be dropped from S2");
        assert_eq!(state.caution_laps, 3, "Final pace lap should increment caution_laps to 3");

        // 9. Johnny and Bob complete Lap 18 (first green restart lap: 30.2s for Johnny, 30.9s for Bob)
        laps_comp[1] = 18;
        laps_comp[2] = 18;
        last_laps[1] = 30.200;
        last_laps[2] = 30.900;
        let frame_lap18_green = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 700.2,
            current_fuel_sample: 58.9,
            player_live_pressures: [208.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_lap18_green, None, false);

        let johnny_s2 = &state.drivers.get(&1).unwrap().stints[1];
        let bob_s2 = &state.drivers.get(&2).unwrap().stints[1];
        assert_eq!(johnny_s2.laps_completed, 1, "Johnny should have 1 green lap in Stint S2");
        assert_eq!(bob_s2.laps_completed, 1, "Bob should have 1 green lap in Stint S2");
        assert!((johnny_s2.last_lap - 30.200).abs() < 0.001);
        assert!((bob_s2.last_lap - 30.900).abs() < 0.001);
    }

    #[test]
    fn test_road_course_local_yellow_ignored_and_only_pitting_resets_stint() {
        let mut state = WarRoomState::default();
        state.session_type = "RACE".to_string();
        state.is_oval = false; // Road course!
        state.player_car_idx = 1;

        let mut pit_road = vec![false; 4];
        let mut laps_comp = vec![0; 4];
        let mut last_laps = vec![0.0; 4];
        let fast_laps = vec![0.0; 4];
        let positions = vec![0, 1, 2, 0];
        let surfaces = vec![0, 3, 3, 0];
        let lap_dist_pcts = vec![0.0, 0.02, 0.02, 0.0];

        // Start race under green
        let frame_start = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 10.0,
            current_fuel_sample: 80.0,
            player_live_pressures: [180.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_start, None, false);

        // Complete Lap 1 under green
        laps_comp[1] = 1;
        laps_comp[2] = 1;
        last_laps[1] = 95.2;
        last_laps[2] = 95.8;
        let frame_lap1 = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 105.0,
            current_fuel_sample: 77.0,
            player_live_pressures: [185.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_lap1, None, false);

        // Local yellow (FLAG_YELLOW | FLAG_YELLOW_WAVING) on Lap 2 on a Road Course:
        // Must NOT trigger caution or reset stints!
        laps_comp[1] = 2;
        laps_comp[2] = 2;
        last_laps[1] = 96.1;
        last_laps[2] = 96.4;
        let frame_local_yellow = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_YELLOW | FLAG_YELLOW_WAVING),
            cur_session_time: 201.0,
            current_fuel_sample: 74.0,
            player_live_pressures: [186.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_local_yellow, None, false);
        assert!(!state.is_under_caution, "Local yellow on road course must not trigger full course caution");
        assert_eq!(state.caution_count, 0);
        assert_eq!(state.drivers.get(&1).unwrap().stints.len(), 1);
        assert_eq!(state.drivers.get(&2).unwrap().stints.len(), 1);
        assert_eq!(state.drivers.get(&1).unwrap().stints[0].laps_completed, 2, "Lap during local yellow still counts on road course");
        assert_eq!(state.drivers.get(&2).unwrap().stints[0].laps_completed, 2);

        // Johnny (1) pits under green on Lap 3 while Bob (2) stays out
        pit_road[1] = true;
        let frame_johnny_pit = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 290.0,
            current_fuel_sample: 71.5,
            player_live_pressures: [186.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_johnny_pit, None, false);
        assert!(!state.drivers.get(&1).unwrap().stints[0].is_active, "Johnny's stint closes on pit entry");
        assert!(state.drivers.get(&2).unwrap().stints[0].is_active, "Bob's stint stays active while Bob stays out");

        // Johnny exits pits on Lap 3; Bob completes Lap 3 on track
        pit_road[1] = false;
        laps_comp[2] = 3;
        last_laps[2] = 95.5;
        let frame_johnny_exit = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 325.0,
            current_fuel_sample: 80.0,
            player_live_pressures: [180.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_johnny_exit, None, false);
        assert_eq!(state.drivers.get(&1).unwrap().stints.len(), 2, "Johnny starts S2 after pitting");
        assert_eq!(state.drivers.get(&2).unwrap().stints.len(), 1, "Bob is still on S1");
        assert_eq!(state.drivers.get(&2).unwrap().stints[0].laps_completed, 3);
    }

    #[test]
    fn test_close_race_session_and_load_new_practice_session_wipes_old_stints_and_drivers() {
        let mut state = WarRoomState::default();

        // 1. Load Race Session YAML (with Johnny at CarIdx 1, Bob at CarIdx 2, Rival at CarIdx 15)
        let race_yaml = r#"
WeekendInfo:
  TrackDisplayName: "Charlotte Motor Speedway - Oval"
  TrackLength: "1.50 mi"
  Category: "Oval"
  TrackType: "super speedway"
  SessionID: 900100
  SubSessionID: 770011
SessionInfo:
  Sessions:
    - SessionNum: 0
      SessionType: "Practice"
    - SessionNum: 1
      SessionType: "Qualify"
    - SessionNum: 2
      SessionType: "Race"
DriverInfo:
  DriverCarIdx: 1
  DriverCarFuelMaxLtr: 70.0
  DriverSetupName: "Charlotte_Race.sto"
  Drivers:
    - CarIdx: 1
      UserName: "Johnny Racer"
      CarNumber: "12"
      IRating: 3200
      CarScreenName: "NASCAR Cup Series Next Gen Chevrolet"
    - CarIdx: 2
      UserName: "Bob Competitor"
      CarNumber: "24"
      IRating: 3100
    - CarIdx: 15
      UserName: "Old Race Guy"
      CarNumber: "99"
      IRating: 2800
"#;
        assert!(parse_iracing_session_yaml(race_yaml, &mut state));
        let transition = state.check_session_transition(2, 3, STATE_RACING, 120.0, false);
        assert!(!transition);
        assert_eq!(state.session_type, "RACE");
        assert_eq!(state.drivers.len(), 3);

        // Simulate green flag laps in Race so Johnny, Bob, and Old Race Guy have stints
        let pit_road = vec![false; 16];
        let mut laps_comp = vec![0; 16];
        let mut last_laps = vec![0.0; 16];
        let fast_laps = vec![0.0; 16];
        let positions = vec![0, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3];
        let surfaces = vec![0, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3];
        let lap_dist_pcts = vec![0.02; 16];

        let frame_race_start = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 120.0,
            current_fuel_sample: 65.0,
            player_live_pressures: [200.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_race_start, None, false);

        for lap in 1..=5 {
            laps_comp[1] = lap;
            laps_comp[2] = lap;
            laps_comp[15] = lap;
            last_laps[1] = 29.5 + (lap as f32) * 0.1;
            last_laps[2] = 29.7 + (lap as f32) * 0.1;
            last_laps[15] = 30.1 + (lap as f32) * 0.1;
            let frame_lap = TelemetryFrame {
                cur_session_state: STATE_RACING,
                flags: Some(FLAG_GREEN),
                cur_session_time: 120.0 + (lap as f64) * 30.0,
                current_fuel_sample: 65.0 - (lap as f32) * 0.8,
                player_live_pressures: [210.0; 4],
                is_on_track: true,
                is_replay: false,
                pit_road: &pit_road,
                laps_comp: &laps_comp,
                last_laps: &last_laps,
                fast_laps: &fast_laps,
                positions: &positions,
                surfaces: &surfaces,
                lap_dist_pcts: &lap_dist_pcts,
            };
            state.process_telemetry_frame(&frame_lap, None, false);
        }

        assert_eq!(state.drivers.get(&1).unwrap().stints[0].laps_completed, 5);
        assert_eq!(state.drivers.get(&15).unwrap().stints[0].laps_completed, 5);

        // 2. Now user closes the Race session and loads a brand new Practice session!
        // Even if we test WITHOUT setting needs_new_connection_reset first (pure YAML change from Race to new Practice):
        let new_practice_yaml = r#"
WeekendInfo:
  TrackDisplayName: "Charlotte Motor Speedway - Oval"
  TrackLength: "1.50 mi"
  Category: "Oval"
  TrackType: "super speedway"
  SessionID: 900250
  SubSessionID: 770999
SessionInfo:
  Sessions:
    - SessionNum: 0
      SessionType: "Practice"
DriverInfo:
  DriverCarIdx: 0
  DriverCarFuelMaxLtr: 70.0
  DriverSetupName: "Charlotte_Prac.sto"
  Drivers:
    - CarIdx: 0
      UserName: "Johnny Racer"
      CarNumber: "12"
      IRating: 3200
      CarScreenName: "NASCAR Cup Series Next Gen Chevrolet"
"#;
        assert!(parse_iracing_session_yaml(new_practice_yaml, &mut state));
        let _ = state.check_session_transition(0, 1, STATE_RACING, 15.0, false);

        // Old race session must be archived in session_history
        assert_eq!(state.session_history.len(), 1);
        assert_eq!(state.session_history[0].session_type, "RACE");

        // Current state must be 100% clean for the new Practice session!
        assert_eq!(state.session_type, "PRACTICE");
        assert_eq!(state.session_num, 0);
        assert_eq!(state.player_car_idx, 0);
        assert!(!state.drivers.contains_key(&2), "Old race driver Bob must be wiped");
        assert!(!state.drivers.contains_key(&15), "Old race driver at CarIdx 15 must be wiped");
        assert_eq!(state.drivers.len(), 1, "Only the new Practice session driver should exist");
        assert!(state.drivers.get(&0).unwrap().stints.is_empty(), "New Practice session must start with 0 stints");
        assert_eq!(state.drivers.get(&0).unwrap().total_laps_completed, 0);
    }

    #[test]
    fn test_sim_header_readiness_during_iracing_loading_screen() {
        let mut header: iracing::telemetry::Header = unsafe { std::mem::zeroed() };
        assert!(!is_sim_header_ready(&header), "Zeroed header during initial load must not be ready");

        // Sim sets status = 1 (stConnected), but n_vars is still 0
        header.status = 1;
        header.n_vars = 0;
        header.buffer_length = 1024;
        header.header_offset = 144;
        assert!(!is_sim_header_ready(&header), "Header with 0 telemetry variables must not be ready");

        // Sim populates n_vars, buffer_length, and header_offset while status == 1
        header.n_vars = 280;
        assert!(is_sim_header_ready(&header), "Fully initialized connected header should be ready");

        // Sim exits session (status bit 0 cleared to 0)
        header.status = 0;
        assert!(!is_sim_header_ready(&header), "Disconnected header (status=0) must immediately report not ready");
    }

    #[test]
    fn test_overlay_settings_backwards_compatibility_and_customization() {
        // 1. Simulating older JSON without the new overlay fields
        let legacy_json = r#"{
            "enabled": true,
            "shape": "Square",
            "opacity": 0.88,
            "always_on_top": true,
            "safety_margin_laps": 1.0,
            "show_weather": true,
            "show_fastest": true,
            "show_averages": true,
            "position": [100.0, 200.0]
        }"#;

        let parsed: settings::OverlaySettings = serde_json::from_str(legacy_json).expect("Legacy JSON must parse");
        assert!(parsed.show_pace_card, "show_pace_card should default to true");
        assert!(parsed.show_fuel_card, "show_fuel_card should default to true");
        assert!(parsed.show_race_control_card, "show_race_control_card should default to true");
        assert!(parsed.show_weather, "show_weather should default to true");
        assert_eq!(parsed.fuel_hero_metric, FuelHeroMetric::FuelAmount);
        assert!((parsed.hud_scale - 1.0).abs() < 0.001);

        // 2. Custom settings serialization & roundtrip
        let mut custom = parsed;
        custom.fuel_hero_metric = FuelHeroMetric::LapsToEmpty;
        custom.show_pace_card = false;
        custom.show_weather = true;
        custom.hud_scale = 1.20;

        let serialized = serde_json::to_string(&custom).expect("Must serialize");
        let roundtrip: settings::OverlaySettings = serde_json::from_str(&serialized).expect("Roundtrip must parse");
        assert_eq!(roundtrip.fuel_hero_metric, FuelHeroMetric::LapsToEmpty);
        assert!(!roundtrip.show_pace_card);
        assert!(roundtrip.show_fuel_card);
        assert!(roundtrip.show_weather);
        assert!((roundtrip.hud_scale - 1.20).abs() < 0.001);
    }

    #[test]
    fn test_parse_car_setup_tires_exact_iracing_garage_values() {
        let sample_yaml = r#"
WeekendInfo:
  TrackDisplayName: "Phoenix Raceway"
SessionInfo:
  Sessions:
    - SessionNum: 0
      SessionType: "Practice"
DriverInfo:
  DriverCarIdx: 0
  Drivers:
    - CarIdx: 0
      UserName: "Test Driver"
CarSetup:
  UpdateCount: 3
  Tires:
    LeftFront:
      ColdPressure: "10.0 psi"
      LastHotPressure: "12.1 psi"
      LastTempsOMI: "194F, 187F, 164F"
      TreadRemaining: "86%, 86%, 100%"
    RightFront:
      ColdPressure: "20.0 psi"
      LastHotPressure: "38.5 psi"
      LastTempsIMO: "259F, 246F, 255F"
      TreadRemaining: "38%, 36%, 44%"
    LeftRear:
      ColdPressure: "10.0 psi"
      LastHotPressure: "12.9 psi"
      LastTempsOMI: "216F, 210F, 205F"
      TreadRemaining: "84%, 84%, 88%"
    RightRear:
      ColdPressure: "20.0 psi"
      LastHotPressure: "37.8 psi"
      LastTempsIMO: "241F, 235F, 240F"
      TreadRemaining: "24%, 22%, 30%"
"#;
        let mut state = WarRoomState::default();
        let success = parse_iracing_session_yaml(sample_yaml, &mut state);
        assert!(success);

        let tires = &state.latest_tire_data;
        assert!(tires.has_data);

        // Check LF (Left tire: 194 · 187 · 164 F, 86% · 86% · 100%)
        assert!((tires.lf.pressure * 0.145038 - 12.1).abs() < 0.15, "LF pressure should be 12.1 psi");
        assert!((tires.lf.temp_l * 1.8 + 32.0 - 194.0).abs() < 0.5, "LF Outer temp should be 194 F");
        assert!((tires.lf.temp_m * 1.8 + 32.0 - 187.0).abs() < 0.5, "LF Middle temp should be 187 F");
        assert!((tires.lf.temp_r * 1.8 + 32.0 - 164.0).abs() < 0.5, "LF Inner temp should be 164 F");
        assert_eq!(tires.lf.wear_l, 86.0, "LF Outer wear should be 86%");
        assert_eq!(tires.lf.wear_m, 86.0, "LF Middle wear should be 86%");
        assert_eq!(tires.lf.wear_r, 100.0, "LF Inner wear should be 100%");

        // Check RF (Right tire: I·M·O order: 259 · 246 · 255 F, 38% · 36% · 44%)
        assert!((tires.rf.pressure * 0.145038 - 38.5).abs() < 0.15, "RF pressure should be 38.5 psi");
        assert!((tires.rf.temp_l * 1.8 + 32.0 - 259.0).abs() < 0.5, "RF Inner temp should be 259 F");
        assert!((tires.rf.temp_m * 1.8 + 32.0 - 246.0).abs() < 0.5, "RF Middle temp should be 246 F");
        assert!((tires.rf.temp_r * 1.8 + 32.0 - 255.0).abs() < 0.5, "RF Outer temp should be 255 F");
        assert_eq!(tires.rf.wear_l, 38.0, "RF Inner wear should be 38%");
        assert_eq!(tires.rf.wear_m, 36.0, "RF Middle wear should be 36%");
        assert_eq!(tires.rf.wear_r, 44.0, "RF Outer wear should be 44%");

        // Check LR (Left tire: 216 · 210 · 205 F, 84% · 84% · 88%)
        assert!((tires.lr.pressure * 0.145038 - 12.9).abs() < 0.15, "LR pressure should be 12.9 psi");
        assert!((tires.lr.temp_l * 1.8 + 32.0 - 216.0).abs() < 0.5, "LR Outer temp should be 216 F");
        assert_eq!(tires.lr.wear_l, 84.0, "LR Outer wear should be 84%");

        // Check RR (Right tire: I·M·O order: 241 · 235 · 240 F, 24% · 22% · 30%)
        assert!((tires.rr.pressure * 0.145038 - 37.8).abs() < 0.15, "RR pressure should be 37.8 psi");
        assert!((tires.rr.temp_l * 1.8 + 32.0 - 241.0).abs() < 0.5, "RR Inner temp should be 241 F");
        assert_eq!(tires.rr.wear_l, 24.0, "RR Inner wear should be 24%");

        // Verify TSV export matches iRacing display
        let lf_tsv = format_single_tire_tsv("Left Front (LF)", &tires.lf, false);
        assert!(lf_tsv.contains("12.1 PSI"), "LF TSV contains pressure: {}", lf_tsv);
        assert!(lf_tsv.contains("194°·187°·164°F"), "LF TSV should match garage order: {}", lf_tsv);
        assert!(lf_tsv.contains("86%·86%·100%"), "LF TSV should match garage order: {}", lf_tsv);

        let rf_tsv = format_single_tire_tsv("Right Front (RF)", &tires.rf, false);
        assert!(rf_tsv.contains("38.5 PSI"), "RF TSV contains pressure: {}", rf_tsv);
        assert!(rf_tsv.contains("259°·246°·255°F"), "RF TSV should match garage order: {}", rf_tsv);
        assert!(rf_tsv.contains("38%·36%·44%"), "RF TSV should match garage order: {}", rf_tsv);
    }

    #[test]
    fn test_stint_tires_isolation_and_no_premature_pit_entry_latch() {
        let mut state = WarRoomState::default();
        state.session_type = "RACE".to_string();
        state.is_oval = true;
        state.player_car_idx = 0;

        let race_yaml = r#"
WeekendInfo:
  TrackDisplayName: "Phoenix Raceway"
SessionInfo:
  Sessions:
    - SessionNum: 0
      SessionType: "Race"
DriverInfo:
  DriverCarIdx: 0
  Drivers:
    - CarIdx: 0
      UserName: "Test Driver"
"#;
        assert!(parse_iracing_session_yaml(race_yaml, &mut state));
        let _ = state.check_session_transition(0, 0, STATE_RACING, 10.0, false);

        // Frame 0: Green flag start on Lap 0
        let mut pit_road = vec![false; 1];
        let mut laps_comp = vec![0; 1];
        let mut last_laps = vec![0.0; 1];
        let fast_laps = vec![0.0; 1];
        let positions = vec![1; 1];
        let mut surfaces = vec![4; 1]; // OnTrack
        let mut lap_dist_pcts = vec![0.02; 1];

        let frame_start = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 10.0,
            current_fuel_sample: 65.0,
            player_live_pressures: [0.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_start, None, false);

        // Frame 1: Driver completes lap 1 out on track
        laps_comp[0] = 1;
        last_laps[0] = 28.5;
        lap_dist_pcts[0] = 0.05;

        let frame_lap1 = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 40.0,
            current_fuel_sample: 64.5,
            player_live_pressures: [0.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_lap1, None, false);

        let driver = state.drivers.get(&0).unwrap();
        assert_eq!(driver.stints.len(), 1);
        assert_eq!(driver.stints[0].laps_completed, 1);
        assert!(!driver.stints[0].tires.has_data, "Active stint on track has no pit tire data");

        // Frame 2: Car enters pit road (pit entry line at speed).
        pit_road[0] = true;
        surfaces[0] = 3; // ApproachingPits (moving at pit speed, not stopped in stall)
        let frame_pit_entry = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 70.0,
            current_fuel_sample: 64.0,
            player_live_pressures: [0.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame_pit_entry, None, false);

        let driver = state.drivers.get(&0).unwrap();
        assert_eq!(driver.stints.len(), 1);
        let s1 = &driver.stints[0];
        assert!(!s1.is_active, "Stint S1 is closed at pit entry");
        assert!(!s1.tires.has_data, "Stint S1 must NOT latch tire data at pit entry before stopping in stall");
    }

    #[test]
    fn test_weather_overlay_parsing_and_formatting() {
        // 1. Temperature parsing
        assert_eq!(parse_temp_num("72.4 F", false), Some(72));
        assert_eq!(parse_temp_num("72.4 F", true), Some(22));
        assert_eq!(parse_temp_num("82.0 F", false), Some(82));
        assert_eq!(parse_temp_num("82.0 F", true), Some(28));
        assert_eq!(parse_temp_num("22 C", false), Some(72));
        assert_eq!(parse_temp_num("22 C", true), Some(22));
        assert_eq!(parse_temp_num("28 C", true), Some(28));
        assert_eq!(parse_temp_num("...", false), None);
        assert_eq!(parse_temp_num("—", true), None);

        // 2. Wind speed parsing and conversion precision
        assert_eq!(parse_wind_speed("5.2 mph", false), (Some(5), "mph"));
        assert_eq!(parse_wind_speed("6 mph", false), (Some(6), "mph"));
        assert_eq!(parse_wind_speed("6 mph", true), (Some(10), "km/h"));
        assert_eq!(parse_wind_speed("2.24 m/s", false), (Some(5), "mph"));
        assert_eq!(parse_wind_speed("2.68 m/s", false), (Some(6), "mph"));
        assert_eq!(parse_wind_speed("3.13 m/s", false), (Some(7), "mph"));
        assert_eq!(parse_wind_speed("3.58 m/s", false), (Some(8), "mph"));
        assert_eq!(parse_wind_speed("10 km/h", true), (Some(10), "km/h"));
        assert_eq!(parse_wind_speed("...", false), (None, "mph"));

        // Verify convert_unit preserves exact integer mph from 2-decimal m/s SDK inputs
        assert_eq!(convert_unit("2.24 m/s", false), "5 mph");
        assert_eq!(convert_unit("2.68 m/s", false), "6 mph");
        assert_eq!(convert_unit("3.13 m/s", false), "7 mph");
        assert_eq!(convert_unit("3.58 m/s", false), "8 mph");

        // 3. Wind direction parsing
        let ne_rad = parse_wind_direction_rad("NE").expect("NE must parse");
        assert!((ne_rad - 45.0_f32.to_radians()).abs() < 0.001);

        let nw_rad = parse_wind_direction_rad("NW").expect("NW must parse");
        assert!((nw_rad - 315.0_f32.to_radians()).abs() < 0.001);

        let n_rad = parse_wind_direction_rad("N").expect("N must parse");
        assert!(n_rad.abs() < 0.001);

        let rad_val = parse_wind_direction_rad("0.785 rad").expect("rad must parse");
        assert!((rad_val - 0.785).abs() < 0.001);

        let deg_val = parse_wind_direction_rad("90 deg").expect("deg must parse");
        assert!((deg_val - 90.0_f32.to_radians()).abs() < 0.001);

        assert_eq!(parse_wind_direction_rad("..."), None);
        assert_eq!(parse_wind_direction_rad("—"), None);

        // 4. Stint analyzer sample: 72/82 temp display format
        let air = parse_temp_num("72 F", false);
        let trk = parse_temp_num("82 F", false);
        let formatted = format!("{}/{}", air.unwrap(), trk.unwrap());
        assert_eq!(formatted, "72/82");

        // 5. Car-relative wind calculations
        // User scenario: Wind from West (270° = 3pi/2), car facing North (yaw = 0.0)
        // Wind is blowing from left to right across the car -> arrow points RIGHT (90° = pi/2)
        let w_west = 270.0_f32.to_radians();
        let rel_north = calculate_relative_wind_rad(w_west, 0.0);
        assert!((rel_north - 90.0_f32.to_radians()).abs() < 0.001);
        assert_eq!(describe_relative_wind(rel_north), "Crosswind (Left → Right)");

        // Car turns 90° right to face East in iRacing (yaw_north = +pi/2 clockwise). Wind from West is now TAILWIND (0° / UP)
        let rel_east = calculate_relative_wind_rad(w_west, 90.0_f32.to_radians());
        assert!(rel_east.abs() < 0.001 || (rel_east - 2.0 * std::f32::consts::PI).abs() < 0.001);
        assert_eq!(describe_relative_wind(rel_east), "Tailwind (Pushing from behind)");

        // Car turns another 90° right to face South (yaw_north = pi). Wind from West is now CROSSWIND R->L (270° / LEFT)
        let rel_south = calculate_relative_wind_rad(w_west, 180.0_f32.to_radians());
        assert!((rel_south - 270.0_f32.to_radians()).abs() < 0.001);
        assert_eq!(describe_relative_wind(rel_south), "Crosswind (Right → Left)");

        // Car turns another 90° right to face West (yaw_north = 3pi/2 or -pi/2). Wind from West is now HEADWIND (180° / DOWN)
        let rel_west_pos = calculate_relative_wind_rad(w_west, 270.0_f32.to_radians());
        assert!((rel_west_pos - 180.0_f32.to_radians()).abs() < 0.001);
        assert_eq!(describe_relative_wind(rel_west_pos), "Headwind (Directly into front)");

        let rel_west_neg = calculate_relative_wind_rad(w_west, -90.0_f32.to_radians());
        assert!((rel_west_neg - 180.0_f32.to_radians()).abs() < 0.001);
        assert_eq!(describe_relative_wind(rel_west_neg), "Headwind (Directly into front)");

        // User report verification: Stint Analyzer 2 o'clock (60°) Quartering Tailwind vs old bug (which showed 4 o'clock / 120°)
        // With West wind (270°) and car at 30° heading:
        let rel_2oclock = calculate_relative_wind_rad(w_west, 30.0_f32.to_radians());
        assert!((rel_2oclock - 60.0_f32.to_radians()).abs() < 0.001);
        assert_eq!(describe_relative_wind(rel_2oclock), "Quartering Tailwind (Left → Right)");

        // Direct Tailwind (Wind from South = 180°, car facing North = 0°) -> 0° (UP / Forward)
        let tailwind = calculate_relative_wind_rad(180.0_f32.to_radians(), 0.0);
        assert!(tailwind.abs() < 0.001 || (tailwind - 2.0 * std::f32::consts::PI).abs() < 0.001);
        assert_eq!(describe_relative_wind(tailwind), "Tailwind (Pushing from behind)");

        // Direct Headwind (Wind from North = 0°, car facing North = 0°) -> 180° (DOWN / Backward)
        let headwind = calculate_relative_wind_rad(0.0, 0.0);
        assert!((headwind - 180.0_f32.to_radians()).abs() < 0.001);
        assert_eq!(describe_relative_wind(headwind), "Headwind (Directly into front)");
    }

    #[test]
    fn test_multi_stint_comparison_tsv_format() {
        let mut s1 = StintData::default();
        s1.stint_number = 1;
        s1.laps_completed = 3;
        s1.lap_times = vec![14.800, 14.850, 14.900];
        s1.fast_lap = 14.800;
        s1.setup_name = "PB Baseline".to_string();

        let mut s2 = StintData::default();
        s2.stint_number = 2;
        s2.laps_completed = 3;
        s2.lap_times = vec![14.750, 14.780, 14.820];
        s2.fast_lap = 14.750;
        s2.setup_name = "PB HighDownforce".to_string();

        let mut s3 = StintData::default();
        s3.stint_number = 1;
        s3.laps_completed = 3;
        s3.lap_times = vec![14.950, 15.000, 15.050];
        s3.fast_lap = 14.950;
        s3.setup_name = "— (Hidden by iRacing sim)".to_string();

        let stints_info = vec![
            (&s1, "Stint A", "Player 1", "42", "PB Baseline", true),
            (&s2, "Stint B", "Player 1", "42", "PB HighDownforce", true),
            (&s3, "Stint C", "Bob Rival", "7", "— (Hidden by iRacing sim)", false),
        ];

        let active_buckets = vec![5];
        let tsv = format_stints_comparison_tsv(&stints_info, &active_buckets, false);

        assert!(tsv.contains("THE SEARCH FOR SPEED — STINT COMPARISON REPORT"));
        assert!(tsv.contains("Stint A (#42 Player 1)"));
        assert!(tsv.contains("Stint B (#42 Player 1)"));
        assert!(tsv.contains("Stint C (#7 Bob Rival)"));
        assert!(tsv.contains("Fastest Lap\t14.800\t14.750\t14.950"));
        assert!(tsv.contains("Setup\tPB Baseline\tPB HighDownforce\t— (Hidden by iRacing sim)"));
        assert!(tsv.contains("LAP-BY-LAP COMPARISON"));
        // Check delta calculation in row 1: B - A = 14.750 - 14.800 = -0.050, C - A = 14.950 - 14.800 = +0.150
        assert!(tsv.contains("L1\t14.800\t14.750\t14.950\t-0.050\t+0.150"));
    }

    #[test]
    fn test_multi_stint_slots_population_and_sanitization() {
        let mut state = WarRoomState::default();
        state.player_car_idx = 0;

        let mut player = DriverState::default();
        player.car_idx = 0;
        player.car_num = "42".to_string();
        player.driver_name = "You".to_string();

        let mut p_s1 = StintData::default();
        p_s1.stint_number = 1;
        p_s1.laps_completed = 10;
        player.stints.push(p_s1);

        let mut p_s2 = StintData::default();
        p_s2.stint_number = 2;
        p_s2.laps_completed = 12;
        player.stints.push(p_s2);

        state.drivers.insert(0, player);

        let mut rival = DriverState::default();
        rival.car_idx = 1;
        rival.car_num = "7".to_string();
        rival.driver_name = "Rival Bob".to_string();

        let mut r_s1 = StintData::default();
        r_s1.stint_number = 1;
        r_s1.laps_completed = 15;
        rival.stints.push(r_s1);

        state.drivers.insert(1, rival);

        let state_arc = Arc::new(Mutex::new(state.clone()));
        let mut app = WarRoomApp::new_for_test(state_arc);

        // Slots should populate with player stint 2 and stint 1
        app.ensure_compare_slots_populated(&state);
        assert_eq!(app.stint_compare_slots.len(), 2);
        assert_eq!(app.stint_compare_slots[0].car_idx, 0);

        // Add 3rd stint from rival
        app.stint_compare_slots.push(StintCompareSlot { car_idx: 1, stint_number: 1 });
        assert_eq!(app.stint_compare_slots.len(), 3);

        // Sanitize: all 3 exist
        app.sanitize_compare_slots(&state);
        assert_eq!(app.stint_compare_slots.len(), 3);

        // Remove rival driver from state
        state.drivers.remove(&1);
        app.sanitize_compare_slots(&state);
        assert_eq!(app.stint_compare_slots.len(), 2, "Rival slot must be cleanly pruned when driver disappears");
    }

    #[test]
    fn test_multi_stint_pace_falloff_and_deltas() {
        let mut s1 = StintData::default();
        s1.stint_number = 1;
        // 5 fast laps, 5 slower laps -> drop off = +0.300
        s1.lap_times = vec![
            15.000, 15.000, 15.000, 15.000, 15.000,
            15.300, 15.300, 15.300, 15.300, 15.300,
        ];
        s1.laps_completed = 10;
        s1.fast_lap = 15.000;

        let mut s2 = StintData::default();
        s2.stint_number = 2;
        // 5 fast laps, 5 much slower laps -> drop off = +0.600
        s2.lap_times = vec![
            14.900, 14.900, 14.900, 14.900, 14.900,
            15.500, 15.500, 15.500, 15.500, 15.500,
        ];
        s2.laps_completed = 10;
        s2.fast_lap = 14.900;

        assert!((s1.drop_off() - 0.300).abs() < 0.001);
        assert!((s2.drop_off() - 0.600).abs() < 0.001);
        assert!(s2.fast_lap < s1.fast_lap, "Stint 2 has faster peak lap");
        assert!(s1.drop_off() < s2.drop_off(), "Stint 1 has less tire degradation / fall-off");
    }

    #[test]
    fn test_multi_stint_single_stint_and_zero_stint_safety() {
        // 1. Zero stints
        let mut state_empty = WarRoomState::default();
        state_empty.player_car_idx = 0;
        let state_arc = Arc::new(Mutex::new(state_empty.clone()));
        let mut app = WarRoomApp::new_for_test(state_arc);
        app.ensure_compare_slots_populated(&state_empty);
        assert_eq!(app.stint_compare_slots.len(), 0);

        // 2. Exactly one stint in the entire session (e.g. solo practice / test drive)
        let mut state_one = WarRoomState::default();
        state_one.player_car_idx = 0;
        let mut player = DriverState::default();
        player.car_idx = 0;
        player.car_num = "88".to_string();
        player.driver_name = "Solo Driver".to_string();
        let mut s1 = StintData::default();
        s1.stint_number = 1;
        s1.laps_completed = 5;
        s1.lap_times = vec![14.800, 14.850, 14.900];
        s1.fast_lap = 14.800;
        player.stints.push(s1.clone());
        state_one.drivers.insert(0, player);

        let state_arc_one = Arc::new(Mutex::new(state_one.clone()));
        let mut app_one = WarRoomApp::new_for_test(state_arc_one);
        app_one.ensure_compare_slots_populated(&state_one);
        assert_eq!(app_one.stint_compare_slots.len(), 1, "Should gracefully hold 1 stint when only 1 exists");

        // Format TSV with 1 stint - must never panic
        let info = [(&s1, "STINT A", "Solo Driver", "88", "Baseline", true)];
        let tsv = format_stints_comparison_tsv(&info, &[5], false);
        assert!(tsv.contains("Solo Driver"));
        assert!(tsv.contains("L1\t14.800"));

        // 3. One stint for player, one stint for competitor -> should pair them up automatically
        let mut state_two_diff = state_one.clone();
        let mut rival = DriverState::default();
        rival.car_idx = 1;
        rival.car_num = "99".to_string();
        rival.driver_name = "Rival".to_string();
        let mut r1 = StintData::default();
        r1.stint_number = 1;
        r1.laps_completed = 5;
        r1.lap_times = vec![14.900, 14.950];
        r1.fast_lap = 14.900;
        rival.stints.push(r1);
        state_two_diff.drivers.insert(1, rival);

        let state_arc_two = Arc::new(Mutex::new(state_two_diff.clone()));
        let mut app_two = WarRoomApp::new_for_test(state_arc_two);
        app_two.ensure_compare_slots_populated(&state_two_diff);
        assert_eq!(app_two.stint_compare_slots.len(), 2, "Should pair player stint with competitor stint");
        assert_eq!(app_two.stint_compare_slots[0].car_idx, 0);
        assert_eq!(app_two.stint_compare_slots[1].car_idx, 1);
    }

    #[test]
    fn test_overlay_laps_remain_and_done_formatting_no_negative_laps() {
        // 1. Negative lap (pre-race / pace lap / grid) must NEVER display "Lap -1 done"
        let (rem_str, done_str) = format_overlay_race_remain_strings(50, -1, false, 50);
        assert_eq!(done_str, "0 laps done", "Negative player lap must cleanly format as '0 laps done'");
        assert_eq!(rem_str, "50 laps");

        // 2. Lap 0 (on lap 1 before line crossing)
        let (rem_str0, done_str0) = format_overlay_race_remain_strings(50, 0, false, 50);
        assert_eq!(done_str0, "0 laps done");
        assert_eq!(rem_str0, "50 laps");

        // 3. Lap 1 completed
        let (_, done_str1) = format_overlay_race_remain_strings(49, 1, false, 50);
        assert_eq!(done_str1, "1 lap done");

        // 4. Lap 15 completed
        let (_, done_str15) = format_overlay_race_remain_strings(35, 15, false, 50);
        assert_eq!(done_str15, "Lap 15 done");
    }

    #[test]
    fn test_overlay_laps_remain_countdown_and_finish_states() {
        // Scheduled 50-lap race
        let total = 50;

        // Start of race (0 laps completed)
        let rem_start = compute_overlay_laps_remain(total, 50, 0, false, false);
        assert_eq!(rem_start, 50);
        let (hero_start, _) = format_overlay_race_remain_strings(rem_start, 0, false, total);
        assert_eq!(hero_start, "50 laps");

        // 2 TO GO: 48 laps completed out of 50
        // Must show 2 laps, NEVER prematurely show "1 to go" / "1 lap"
        let rem_2togo = compute_overlay_laps_remain(total, 1, 48, false, false);
        assert_eq!(rem_2togo, 2, "At 2 to go (48/50), remaining laps must be 2 even if legacy telemetry jumped to 1");
        let (hero_2togo, done_2togo) = format_overlay_race_remain_strings(rem_2togo, 48, false, total);
        assert_eq!(hero_2togo, "2 laps");
        assert_eq!(done_2togo, "Lap 48 done");

        // 1 TO GO (White Flag): 49 laps completed out of 50
        let rem_1togo = compute_overlay_laps_remain(total, 1, 49, true, false);
        assert_eq!(rem_1togo, 1);
        let (hero_1togo, done_1togo) = format_overlay_race_remain_strings(rem_1togo, 49, false, total);
        assert_eq!(hero_1togo, "1 lap", "1 lap remaining must format as '1 lap' rather than '1 laps'");
        assert_eq!(done_1togo, "Lap 49 done");

        // FINISH (Checkered Flag): 50 laps completed out of 50
        // Must transition to Finished, NEVER stay stuck on "1 to go"
        let rem_finish = compute_overlay_laps_remain(total, 0, 50, false, true);
        assert_eq!(rem_finish, 0);
        let (hero_finish, done_finish) = format_overlay_race_remain_strings(rem_finish, 50, true, total);
        assert_eq!(hero_finish, "Finished", "Race end must transition to Finished rather than staying stuck or Unlimited");
        assert_eq!(done_finish, "Lap 50 done");

        // Unlimited / Practice session (total = 0, remain = 0)
        let rem_unlimited = compute_overlay_laps_remain(0, 0, 5, false, false);
        assert_eq!(rem_unlimited, 0);
        let (hero_unlimited, _) = format_overlay_race_remain_strings(rem_unlimited, 5, false, 0);
        assert_eq!(hero_unlimited, "Unlimited");

        // Timed race with 14 laps left
        let rem_timed = compute_overlay_laps_remain(0, 14, 10, false, false);
        assert_eq!(rem_timed, 14);
        let (hero_timed, _) = format_overlay_race_remain_strings(rem_timed, 10, false, 0);
        assert_eq!(hero_timed, "14 laps");

        // Timed race checkered flag
        let rem_timed_finish = compute_overlay_laps_remain(0, 0, 25, false, true);
        assert_eq!(rem_timed_finish, 0);
        let (hero_timed_finish, _) = format_overlay_race_remain_strings(rem_timed_finish, 25, true, 0);
        assert_eq!(hero_timed_finish, "Finished");
    }

    #[test]
    fn test_driver_lap_completed_negative_one_sanitized_to_zero() {
        let mut state = WarRoomState::default();
        state.session_type = "RACE".to_string();
        state.player_car_idx = 1;

        let pit_road = vec![false; 4];
        let laps_comp = vec![-1; 4];
        let last_laps = vec![0.0; 4];
        let fast_laps = vec![0.0; 4];
        let positions = vec![0, 1, 2, 0];
        let surfaces = vec![0, 3, 3, 0];
        let lap_dist_pcts = vec![0.0; 4];

        let frame = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 100.0,
            current_fuel_sample: 60.0,
            player_live_pressures: [200.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };

        state.process_telemetry_frame(&frame, None, false);

        let driver = state.drivers.get(&1).unwrap();
        assert_eq!(driver.total_laps_completed, 0, "Initial -1 CarIdxLapCompleted must be clamped to 0");
        assert_eq!(driver.stints[0].start_lap, 0);
        assert_eq!(driver.stints[0].end_lap, 0);
    }

    #[test]
    fn test_tire_popup_trigger_and_overlay_transparency_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.overlay.bg_opacity, 0.0, "Default outer HUD background box opacity should be 0.0 (transparent)");
        assert!(!settings.tire_popup_overlay.enabled, "Tire popup overlay should default to disabled (opt-in like main overlay)");
        assert_eq!(settings.tire_popup_overlay.duration_secs, 15.0);
        assert_eq!(settings.tire_popup_overlay.scale, 1.0);
        assert_eq!(settings.tire_popup_overlay.bg_opacity, 0.0);

        // 1. Pressure-only tire set (sampled while driving on track) must NOT trigger popup
        let mut on_track_tires = StintTireSet::default();
        on_track_tires.lf.pressure = 195.0;
        on_track_tires.rf.pressure = 205.0;
        assert!(!StintTireSet::default().should_trigger_popup(&on_track_tires));

        // 2. Post-pit tire set with wear & temps MUST trigger popup once
        let mut post_pit_tires = on_track_tires.clone();
        post_pit_tires.lf.wear_m = 91.0;
        post_pit_tires.rf.wear_m = 82.0;
        post_pit_tires.lf.temp_m = 94.0;
        post_pit_tires.rf.temp_m = 105.0;
        post_pit_tires.has_data = true;
        assert!(on_track_tires.should_trigger_popup(&post_pit_tires));

        // 3. Subsequent identical frames while sitting in pit stall must NOT re-trigger popup
        assert!(!post_pit_tires.should_trigger_popup(&post_pit_tires));

        // 4. Updated readings when pit crew finishes service or garage CarSetup arrives MUST re-trigger
        let mut updated_pit_tires = post_pit_tires.clone();
        updated_pit_tires.rf.wear_m = 76.0;
        assert!(post_pit_tires.should_trigger_popup(&updated_pit_tires));
    }

    #[test]
    fn test_imported_stint_coexists_with_live_telemetry() {
        let mut state = WarRoomState::default();
        state.player_car_idx = 1;
        state.track_name = "Bristol Motor Speedway".to_string();
        state.car_name = "NASCAR Cup Series Next Gen Chevrolet Camaro ZL1".to_string();

        let mut saved_stint = StintData::default();
        saved_stint.stint_number = 1;
        saved_stint.laps_completed = 10;
        saved_stint.lap_times = vec![15.1, 15.0, 14.9, 15.0, 15.05, 15.1, 15.12, 15.15, 15.2, 15.22];
        saved_stint.fast_lap = 14.9;
        saved_stint.fuel_used = 4.20;
        saved_stint.lap_fuel_burns = vec![0.42; 10];
        saved_stint.setup_name = "Bristol_Q_Race.sto".to_string();

        let rec = stint_db::SavedStintRecord {
            id: "test_bristol_stint_1".to_string(),
            saved_at_unix: 1700000000,
            saved_at_display: "2026-10-24 17:00".to_string(),
            track_name: "Bristol Motor Speedway".to_string(),
            track_length: "0.53 mi".to_string(),
            car_name: "NASCAR Cup Series Next Gen Chevrolet Camaro ZL1".to_string(),
            session_type: "PRACTICE".to_string(),
            driver_name: "TestDriver".to_string(),
            car_num: "24".to_string(),
            setup_name: "Bristol_Q_Race.sto".to_string(),
            track_temp: "30.0 C".to_string(),
            air_temp: "22.0 C".to_string(),
            track_state: "50%".to_string(),
            stint: saved_stint,
        };

        let imp_num = state.import_saved_stint(&rec);
        assert_eq!(imp_num, 101);

        let drv = state.drivers.get(&1).unwrap();
        assert_eq!(drv.stints.len(), 1);
        assert!(drv.stints[0].is_imported);
        assert_eq!(drv.stints[0].short_label(), "IMP 1");
        assert!((drv.historical_fuel_burn_rate - 0.42).abs() < 0.001, "Historical fuel burn should be seeded from imported stint");

        // Now simulate live telemetry on track: live stint S1 should be created alongside IMP 1
        let mut pit_road = vec![false; 64];
        let mut laps_comp = vec![0; 64];
        let mut last_laps = vec![0.0; 64];
        let mut fast_laps = vec![0.0; 64];
        let mut positions = vec![0; 64];
        let mut surfaces = vec![0; 64];
        let mut lap_dist_pcts = vec![0.0; 64];

        surfaces[1] = 3; // On track
        laps_comp[1] = 1;
        last_laps[1] = 15.050;
        fast_laps[1] = 15.050;
        positions[1] = 1;
        lap_dist_pcts[1] = 0.05;
        pit_road[1] = false;

        let frame1 = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 50.0,
            current_fuel_sample: 60.0,
            player_live_pressures: [200.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame1, None, false);

        // Complete a flying lap
        laps_comp[1] = 2;
        last_laps[1] = 14.850;
        fast_laps[1] = 14.850;
        let frame2 = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: 65.0,
            current_fuel_sample: 59.58,
            player_live_pressures: [200.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame2, None, false);

        let drv = state.drivers.get(&1).unwrap();
        assert_eq!(drv.stints.len(), 2, "Should have both IMP 1 and live S1");
        assert_eq!(drv.stints[0].short_label(), "IMP 1");
        assert_eq!(drv.stints[0].laps_completed, 10, "IMP 1 laps must remain untouched");
        assert_eq!(drv.stints[1].short_label(), "S1");
        assert!(!drv.stints[1].is_imported);
        assert!(drv.stints[1].is_active);
        assert_eq!(drv.stints[1].laps_completed, 1);

        // Unloading IMP 1 should leave live S1 intact
        assert!(state.remove_imported_stint(101));
        let drv = state.drivers.get(&1).unwrap();
        assert_eq!(drv.stints.len(), 1);
        assert_eq!(drv.stints[0].short_label(), "S1");
    }

    /// Feeds a single-car (CarIdx 0) telemetry frame into the state machine.
    fn feed_single_car_frame(state: &mut WarRoomState, t: f64, on_pit_road: bool, lap_completed: i32, last_lap: f32) {
        let pit_road = vec![on_pit_road];
        let laps_comp = vec![lap_completed];
        let last_laps = vec![last_lap];
        let fast_laps = vec![0.0];
        let positions = vec![1];
        let surfaces = vec![if on_pit_road { 1 } else { 3 }];
        let lap_dist_pcts = vec![0.01];
        let frame = TelemetryFrame {
            cur_session_state: STATE_RACING,
            flags: Some(FLAG_GREEN),
            cur_session_time: t,
            current_fuel_sample: 0.0,
            player_live_pressures: [0.0; 4],
            is_on_track: true,
            is_replay: false,
            pit_road: &pit_road,
            laps_comp: &laps_comp,
            last_laps: &last_laps,
            fast_laps: &fast_laps,
            positions: &positions,
            surfaces: &surfaces,
            lap_dist_pcts: &lap_dist_pcts,
        };
        state.process_telemetry_frame(&frame, None, false);
    }

    #[test]
    fn test_practice_stint_lap_numbers_and_near_identical_laps() {
        // Reproduces a real practice stint: pit exit during session lap 41 (out-lap, untimed), flying laps 42..63.
        // Laps 52 and 53 were 0.4 ms apart and lap 53 used to be silently dropped, shifting every later lap.
        let mut state = WarRoomState::default();
        state.session_type = "PRACTICE".to_string();
        state.player_car_idx = 0;

        let prev_last = 34.000_f32;
        feed_single_car_frame(&mut state, 100.0, true, 40, prev_last); // In pit stall
        feed_single_car_frame(&mut state, 105.0, false, 40, prev_last); // Pit exit -> S1 starts, out-lap (session lap 41)
        feed_single_car_frame(&mut state, 140.0, false, 41, prev_last); // Out-lap crosses S/F (untimed)
        feed_single_car_frame(&mut state, 140.3, false, 41, 38.900); // iRacing publishes a late out-lap time -> must be ignored

        let times: [f32; 22] = [
            32.614, 32.650, 32.823, 32.897, 33.139, 33.281, 33.146, 33.109, 33.080, 33.094, 33.1876,
            33.1872, 33.157, 33.328, 33.447, 33.202, 33.146, 33.273, 33.285, 33.404, 33.449, 33.370,
        ];
        let mut t = 140.3;
        let mut prev = 38.900_f32;
        for (i, &lap_time) in times.iter().enumerate() {
            let session_lap = 42 + i as i32;
            t += lap_time as f64;
            // Lap count increments first; CarIdxLastLapTime follows a few frames later
            feed_single_car_frame(&mut state, t, false, session_lap, prev);
            feed_single_car_frame(&mut state, t + 0.1, false, session_lap, prev);
            feed_single_car_frame(&mut state, t + 0.25, false, session_lap, lap_time);
            prev = lap_time;
        }

        let stint = &state.drivers.get(&0).unwrap().stints[0];
        assert_eq!(stint.laps_completed, 22, "All 22 flying laps (42..63) must be counted");
        let numbered = stint.numbered_valid_laps();
        assert_eq!(numbered.first().copied(), Some((42, 32.614)));
        assert_eq!(numbered[10], (52, 33.1876));
        assert_eq!(numbered[11], (53, 33.1872));
        assert_eq!(numbered.last().copied(), Some((63, 33.370)));

        let tsv = format_stint_laps_spreadsheet_tsv(stint);
        let rows: Vec<&str> = tsv.lines().collect();
        assert!(rows[1].starts_with("1\t42\t32.614\t"));
        assert!(rows[12].starts_with("12\t53\t33.187\t"));
        assert!(rows[22].starts_with("22\t63\t33.370\t"));
    }

    #[test]
    fn test_exactly_identical_consecutive_lap_time_is_recorded_after_grace() {
        let mut state = WarRoomState::default();
        state.session_type = "PRACTICE".to_string();
        state.player_car_idx = 0;

        feed_single_car_frame(&mut state, 100.0, true, 10, 0.0);
        feed_single_car_frame(&mut state, 105.0, false, 10, 0.0); // Pit exit, out-lap = session lap 11
        feed_single_car_frame(&mut state, 140.0, false, 11, 0.0); // Out-lap done
        feed_single_car_frame(&mut state, 173.0, false, 12, 0.0);
        feed_single_car_frame(&mut state, 173.2, false, 12, 33.0); // Lap 12 = 33.000

        // Lap 13 is bit-for-bit identical: the count increments but the time never changes
        feed_single_car_frame(&mut state, 206.0, false, 13, 33.0);
        feed_single_car_frame(&mut state, 207.0, false, 13, 33.0);
        assert_eq!(state.drivers.get(&0).unwrap().stints[0].laps_completed, 1, "Still inside grace window");
        feed_single_car_frame(&mut state, 209.5, false, 13, 33.0);
        assert_eq!(state.drivers.get(&0).unwrap().stints[0].laps_completed, 2, "Identical lap recorded after grace");
        feed_single_car_frame(&mut state, 212.0, false, 13, 33.0);
        assert_eq!(state.drivers.get(&0).unwrap().stints[0].laps_completed, 2, "Recorded only once");

        // Lap 14: time arrives BEFORE the count increments — must be one lap, labelled 14
        feed_single_car_frame(&mut state, 239.0, false, 13, 33.1);
        feed_single_car_frame(&mut state, 239.2, false, 14, 33.1);
        feed_single_car_frame(&mut state, 245.0, false, 14, 33.1);

        let stint = &state.drivers.get(&0).unwrap().stints[0];
        assert_eq!(stint.numbered_valid_laps(), vec![(12, 33.0), (13, 33.0), (14, 33.1)]);
    }

    #[test]
    fn test_all_codebase_glyphs_supported() {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Inter".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/Inter-Medium.ttf")),
        );
        fonts.font_data.insert(
            "RacingSans".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/RacingSansOne-Regular.ttf")),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "Inter".to_owned());

        let mut display_family = vec!["RacingSans".to_owned(), "Inter".to_owned()];
        for font_name in fonts.families.get(&egui::FontFamily::Proportional).unwrap() {
            if !display_family.contains(font_name) {
                display_family.push(font_name.clone());
            }
        }
        fonts.families.insert(
            egui::FontFamily::Name("Display".into()),
            display_family,
        );
        ctx.set_fonts(fonts);

        ctx.begin_frame(egui::RawInput::default());
        let fonts_mgr = ctx.fonts(|f| f.clone());
        let font_id = egui::FontId::proportional(14.0);

        let files = [
            include_str!("main.rs"),
            include_str!("models.rs"),
            include_str!("telemetry.rs"),
            include_str!("formatters.rs"),
            include_str!("views/mod.rs"),
            include_str!("views/cockpit.rs"),
            include_str!("views/grid.rs"),
            include_str!("views/stint_modal.rs"),
            include_str!("views/overlay.rs"),
            include_str!("views/modals.rs"),
            include_str!("ibt_ui.rs"),
            include_str!("settings.rs"),
            include_str!("discord_card.rs"),
            include_str!("stint_db.rs"),
            include_str!("ibt.rs"),
            include_str!("../RELEASE_NOTES.md"),
        ];

        let mut missing_chars = std::collections::BTreeMap::new();
        for file in &files {
            for c in file.chars() {
                if !c.is_ascii() && !c.is_whitespace() && !fonts_mgr.has_glyph(&font_id, c) {
                    *missing_chars.entry(c).or_insert(0) += 1;
                }
            }
        }

        for (c, count) in &missing_chars {
            println!("MISSING GLYPH: {:?} (U+{:04X}) count={}", c, *c as u32, count);
        }
        assert!(
            missing_chars.is_empty(),
            "Found unsupported glyphs that will render as '?' in the UI: {:?}",
            missing_chars
        );
    }
}


