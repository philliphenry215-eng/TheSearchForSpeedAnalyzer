#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod settings;
mod updater;

use eframe::egui;
use egui_extras::{TableBuilder, Column};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use iracing::Connection;
use iracing::telemetry::Value;
use settings::{ActiveViewMode, AppSettings, FuelHeroMetric, LayoutPreset, ThemePreset};

fn main() -> eframe::Result<()> {
    let title = if settings::is_beta_mode() {
        "Search For Speed - Stint Analyzer [BETA]"
    } else {
        "Search For Speed - Stint Analyzer"
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
        "Search For Speed Analyzer",
        options,
        Box::new(|cc| {
            Box::new(WarRoomApp::new(cc))
        }),
    )
}

fn display_font(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("Display".into()))
}

fn with_alpha(c: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

fn setup_custom_style(ctx: &egui::Context, settings: &AppSettings) {
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
    fonts.families.insert(
        egui::FontFamily::Name("Display".into()),
        vec!["RacingSans".to_owned(), "Inter".to_owned()],
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


#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TireCornerData {
    pub temp_l: f32, // Inside / Left (°C)
    pub temp_m: f32, // Middle (°C)
    pub temp_r: f32, // Outside / Right (°C)
    pub wear_l: f32, // Inside / Left (% remaining: 0 - 100%)
    pub wear_m: f32, // Middle (% remaining: 0 - 100%)
    pub wear_r: f32, // Outside / Right (% remaining: 0 - 100%)
    pub pressure: f32, // Hot pressure (kPa)
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StintTireSet {
    pub lf: TireCornerData,
    pub rf: TireCornerData,
    pub lr: TireCornerData,
    pub rr: TireCornerData,
    pub has_data: bool,
}

impl StintTireSet {
    pub fn is_valid(&self) -> bool {
        self.lf.pressure > 0.5 || self.rf.pressure > 0.5 || self.lf.temp_m > 10.0 || self.rf.temp_m > 10.0 || self.lf.wear_m > 0.1 || self.rf.wear_m > 0.1
    }
}

#[derive(Clone, Debug)]
pub struct StintData {
    pub stint_number: i32,
    pub start_lap: i32,
    pub end_lap: i32,
    pub out_lap_completed: bool,
    pub awaiting_pace_lap_sf: bool,
    pub awaiting_pit_out_lap_sf: bool,
    pub laps_completed: i32,
    pub lap_times: Vec<f32>,
    pub fuel_start: f32,
    pub fuel_used: f32,
    pub last_lap: f32,
    pub fast_lap: f32,
    pub is_active: bool,
    pub is_expanded: bool,
    pub tires: StintTireSet,
    pub setup_name: String,
    pub track_temp_at_start: String,
    pub air_temp_at_start: String,
    pub notes: String,
    pub crossing_fuel_level: f32,
    pub laps_of_fuel_crossing: f32,
    pub latched_fuel_burn_rate: f32,
    pub lap_fuel_burns: Vec<f32>,
    pub prev_crossing_fuel: f32,
}

impl Default for StintData {
    fn default() -> Self {
        Self {
            stint_number: 1,
            start_lap: 0,
            end_lap: 0,
            out_lap_completed: false,
            awaiting_pace_lap_sf: false,
            awaiting_pit_out_lap_sf: false,
            laps_completed: 0,
            lap_times: Vec::new(),
            fuel_start: 0.0,
            fuel_used: 0.0,
            last_lap: 0.0,
            fast_lap: 0.0,
            is_active: true,
            is_expanded: false,
            tires: StintTireSet::default(),
            setup_name: "—".to_string(),
            track_temp_at_start: "—".to_string(),
            air_temp_at_start: "—".to_string(),
            notes: String::new(),
            crossing_fuel_level: 0.0,
            laps_of_fuel_crossing: 0.0,
            latched_fuel_burn_rate: 0.0,
            lap_fuel_burns: Vec::new(),
            prev_crossing_fuel: 0.0,
        }
    }
}

impl StintData {
    pub fn valid_laps(&self) -> Vec<f32> {
        self.lap_times.iter().copied().filter(|&t| t > 5.0 && t < 360.0).collect()
    }

    pub fn full_average(&self) -> f32 {
        let laps = self.valid_laps();
        if laps.is_empty() { return 0.0; }
        laps.iter().sum::<f32>() / laps.len() as f32
    }

    // Only populates once AT LEAST n laps have been completed in the stint.
    // Averages the FIRST n laps of the stint to capture tire fall-off progression.
    pub fn avg_first_n(&self, n: usize) -> f32 {
        let laps = self.valid_laps();
        if laps.len() < n { return 0.0; }
        let slice = &laps[0..n];
        slice.iter().sum::<f32>() / n as f32
    }

    pub fn sum_first_n(&self, n: usize) -> f32 {
        let laps = self.valid_laps();
        if laps.len() < n { return 0.0; }
        laps[0..n].iter().sum()
    }

    pub fn avg_segment(&self, start: usize, end: usize) -> f32 {
        let laps = self.valid_laps();
        if laps.len() < end || start > end || start == 0 { return 0.0; }
        let slice = &laps[start - 1..end];
        if slice.is_empty() { return 0.0; }
        slice.iter().sum::<f32>() / slice.len() as f32
    }

    pub fn avg_5(&self) -> f32 {
        self.avg_first_n(5)
    }

    pub fn avg_10(&self) -> f32 {
        self.avg_first_n(10)
    }

    pub fn avg_15(&self) -> f32 {
        self.avg_first_n(15)
    }

    pub fn avg_20(&self) -> f32 {
        self.avg_first_n(20)
    }

    pub fn closing_5(&self) -> f32 {
        let laps = self.valid_laps();
        if laps.len() < 5 { return 0.0; }
        let slice = &laps[laps.len() - 5..];
        slice.iter().sum::<f32>() / 5.0
    }

    pub fn drop_off(&self) -> f32 {
        let open = self.avg_5();
        let close = self.closing_5();
        if open <= 0.0 || close <= 0.0 { return 0.0; }
        close - open
    }

    pub fn std_dev(&self) -> f32 {
        let laps = self.valid_laps();
        if laps.len() < 2 { return 0.0; }
        let mean = laps.iter().sum::<f32>() / laps.len() as f32;
        let variance = laps.iter().map(|&t| (t - mean).powi(2)).sum::<f32>() / (laps.len() - 1) as f32;
        variance.sqrt()
    }

    pub fn fuel_per_lap(&self) -> f32 {
        let valid_burns: Vec<f32> = self.lap_fuel_burns.iter().copied().filter(|&b| b > 0.05 && b < 25.0).collect();
        if !valid_burns.is_empty() {
            if valid_burns.len() >= 3 {
                // Responsive rolling average: 70% weight to recent 5 laps, 30% to stint base
                let count = valid_burns.len();
                let recent_count = count.min(5);
                let recent_burns = &valid_burns[count - recent_count..];
                let recent_avg = recent_burns.iter().sum::<f32>() / recent_count as f32;
                let overall_avg = valid_burns.iter().sum::<f32>() / count as f32;
                0.7 * recent_avg + 0.3 * overall_avg
            } else {
                valid_burns.iter().sum::<f32>() / valid_burns.len() as f32
            }
        } else if self.latched_fuel_burn_rate > 0.001 {
            self.latched_fuel_burn_rate
        } else if self.laps_completed > 0 && self.fuel_used > 0.0 {
            (self.fuel_used / self.laps_completed as f32).max(0.0)
        } else {
            0.0
        }
    }
}

pub fn format_session_type(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("open practice") || lower.contains("practice") {
        "PRACTICE".to_string()
    } else if lower.contains("lone qualify") || lower.contains("qualif") {
        "QUALIFYING".to_string()
    } else if lower.contains("warmup") {
        "WARMUP".to_string()
    } else if lower.contains("race") {
        "RACE".to_string()
    } else if !raw.is_empty() {
        raw.to_uppercase()
    } else {
        "SESSION".to_string()
    }
}

pub fn format_track_state(raw: &str, wetness_code: i32) -> String {
    let wetness_str = match wetness_code {
        2 => "Mostly Dry",
        3 => "Very Light Wet",
        4 => "Light Wet",
        5 => "Moderate Wet",
        6 => "Very Wet",
        7 => "Extremely Wet",
        _ => "", // 0 = unknown, 1 = dry
    };

    let raw_clean = raw.trim();
    let rubber_str = if raw_clean.is_empty() || raw_clean == "..." || raw_clean == "—" {
        ""
    } else {
        let lower = raw_clean.to_lowercase();
        if lower.contains("clean") || lower.contains("clear") {
            "Clean Track"
        } else if lower.contains("moderate") {
            "Moderate Rubber"
        } else if lower.contains("very high") || lower.contains("heavy") {
            "Heavy Rubber"
        } else if lower.contains("high") {
            "High Rubber"
        } else if lower.contains("max") {
            "Maximum Rubber"
        } else if lower.contains("low") || lower.contains("light") {
            "Light Rubber"
        } else if lower.contains("carry") || lower.contains("carried") {
            "Carried Over"
        } else {
            raw_clean
        }
    };

    if !wetness_str.is_empty() && !rubber_str.is_empty() {
        format!("{} ({})", rubber_str, wetness_str)
    } else if !wetness_str.is_empty() {
        wetness_str.to_string()
    } else if !rubber_str.is_empty() {
        rubber_str.to_string()
    } else {
        "Clean Track".to_string()
    }
}

pub fn clean_setup_name(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "—" || trimmed == "..." {
        return String::new();
    }
    // Take filename without directory path (handle both / and \)
    let filename = trimmed.rsplit(['/', '\\']).next().unwrap_or(trimmed);
    let clean = if let Some(stripped) = filename.strip_suffix(".sto").or_else(|| filename.strip_suffix(".STO")) {
        stripped.trim()
    } else {
        filename.trim()
    };
    if clean.is_empty() {
        String::new()
    } else {
        clean.to_string()
    }
}

#[derive(Clone, Debug, Default)]
pub struct SessionArchive {
    pub session_num: i32,
    pub session_type: String,
    pub drivers: HashMap<usize, DriverState>,
    pub caution_count: i32,
    pub caution_laps: i32,
}

pub const FLAG_CHECKERED: u32      = 0x0000_0001;
pub const FLAG_WHITE: u32          = 0x0000_0002;
pub const FLAG_GREEN: u32          = 0x0000_0004;
pub const FLAG_YELLOW: u32         = 0x0000_0008; // Local corner yellow
pub const FLAG_RED: u32            = 0x0000_0010;
pub const FLAG_BLUE: u32           = 0x0000_0020;
pub const FLAG_DEBRIS: u32         = 0x0000_0040;
pub const FLAG_CROSSED: u32        = 0x0000_0080;
pub const FLAG_YELLOW_WAVING: u32  = 0x0000_0100; // Local waving yellow
pub const FLAG_ONE_TO_GREEN: u32   = 0x0000_0200;
pub const FLAG_GREEN_HELD: u32     = 0x0000_0400;
pub const FLAG_TEN_TO_GO: u32      = 0x0000_0800;
pub const FLAG_FIVE_TO_GO: u32     = 0x0000_1000;
pub const FLAG_CAUTION: u32        = 0x0000_4000; // Full Course Caution
pub const FLAG_CAUTION_WAVING: u32 = 0x0000_8000; // Full Course Caution Waving
pub const FLAG_START_GO: u32       = 0x8000_0000;

pub const STATE_PARADE_LAPS: i32   = 3;
pub const STATE_RACING: i32        = 4;
pub const STATE_CHECKERED: i32     = 5;

#[derive(Clone, Debug, Default)]
pub struct DriverState {
    pub car_idx: usize,
    pub driver_name: String,
    pub car_num: String,
    pub irating: String,
    pub position: i32,
    pub is_on_pit_road: bool,
    pub is_in_world: bool,
    pub is_initialized: bool,
    pub total_laps_completed: i32,
    pub last_processed_lap: i32,
    pub last_known_lap_time: f32,
    pub last_sf_crossing_time: f64,
    pub exited_pits_on_current_lap: bool,
    pub ignored_lap_sdk: i32,
    pub setup_name: String,
    pub stints: Vec<StintData>,
    pub historical_fuel_burn_rate: f32,
}

impl DriverState {
    pub fn estimated_fuel_burn_ltr(&self) -> f32 {
        // 1. If active stint has at least 1 lap recorded, use active stint rate
        if let Some(active) = self.stints.iter().rev().find(|s| s.is_active) {
            let b = active.fuel_per_lap();
            if b > 0.001 {
                return b;
            }
        }
        // 2. Fallback to previous closed stint with valid fuel data in this session
        for stint in self.stints.iter().rev() {
            let b = stint.fuel_per_lap();
            if b > 0.001 {
                return b;
            }
        }
        // 3. Fallback to session-wide valid lap burn average across all stints in this session
        let all_burns: Vec<f32> = self.stints.iter()
            .flat_map(|s| s.lap_fuel_burns.iter().copied())
            .filter(|&b| b > 0.005 && b < 25.0)
            .collect();
        if !all_burns.is_empty() {
            return all_burns.iter().sum::<f32>() / all_burns.len() as f32;
        }
        // 4. Fallback to historical fuel burn rate from earlier sessions (e.g. Practice before Race)
        if self.historical_fuel_burn_rate > 0.001 {
            return self.historical_fuel_burn_rate;
        }
        0.0
    }
}

#[derive(Clone)]
pub struct WarRoomState {
    pub is_connected: bool,
    pub session_id: i32,
    pub subsession_id: i32,
    pub session_num: i32,
    pub session_unique_id: i32,
    pub needs_new_connection_reset: bool,
    pub session_laps_total: i32,
    pub session_laps_remain: i32,
    pub sof: i32,
    pub track_name: String,
    pub track_length: String,
    pub car_name: String,
    pub session_type: String,
    pub subsession_types: HashMap<i32, String>,
    pub subsession_rubber_states: HashMap<i32, String>,
    pub session_history: Vec<SessionArchive>,
    pub track_state: String,
    pub track_wetness: i32,
    pub player_setup_name: String,
    
    // Telemetry Environmental strings
    pub weather_type: String,
    pub air_temp_raw: String,
    pub track_temp_raw: String,
    pub wind_speed_raw: String,
    pub wind_dir_raw: String,
    pub relative_humidity_raw: String,
    
    // Live Driver Map
    pub drivers: HashMap<usize, DriverState>,
    pub player_car_idx: usize,

    // Real-time Player Telemetry (60Hz)
    pub player_fuel_level: f32,
    pub player_fuel_capacity: f32,
    pub player_fuel_pct: f32,
    pub player_gap_ahead: f32,
    pub player_gap_behind: f32,
    pub player_lap_distance_pct: f32,
    pub latest_tire_data: StintTireSet,
    pub player_is_in_car: bool,
    pub player_incidents: i32,
    pub incident_limit: i32,
    pub caution_count: i32,
    pub caution_laps: i32,
    pub is_under_caution: bool,
    pub is_oval: bool,
    pub was_race_green: bool,
    pub had_caution_since_last_green: bool,
    pub session_flag_name: String,

    // Live Telemetry Debug Inspector (All Key SDK Variables)
    pub debug_lap_number: i32,
    pub debug_lap_completed: i32,
    pub debug_lap_current_time: f32,
    pub debug_lap_last_time: f32,
    pub debug_lap_best_time: f32,
    pub debug_lap_best_lap_num: i32,
    pub debug_lap_dist: f32,
    pub debug_on_pit_road: bool,
    pub debug_track_surface: i32,
    pub debug_is_on_track: bool,
    pub debug_speed: f32,
    pub debug_rpm: f32,
    pub debug_gear: i32,
    pub debug_throttle: f32,
    pub debug_brake: f32,
    pub debug_session_time: f64,
    pub debug_session_state: i32,
    pub debug_session_flags: u32,
    pub debug_event_logs: Vec<String>,
}

impl Default for WarRoomState {
    fn default() -> Self {
        Self {
            is_connected: false,
            session_id: 0,
            subsession_id: 0,
            session_num: -1,
            session_unique_id: -1,
            needs_new_connection_reset: false,
            session_laps_total: 0,
            session_laps_remain: 0,
            sof: 0,
            track_name: "Waiting for iRacing...".to_string(),
            track_length: "—".to_string(),
            car_name: "—".to_string(),
            session_type: "—".to_string(),
            subsession_types: HashMap::new(),
            subsession_rubber_states: HashMap::new(),
            session_history: Vec::new(),
            track_state: "Clean Track".to_string(),
            track_wetness: 0,
            player_setup_name: "Baseline".to_string(),
            weather_type: "...".to_string(),
            air_temp_raw: "...".to_string(),
            track_temp_raw: "...".to_string(),
            wind_speed_raw: "...".to_string(),
            wind_dir_raw: "...".to_string(),
            relative_humidity_raw: "...".to_string(),
            drivers: HashMap::new(),
            player_car_idx: 0,
            player_fuel_level: 0.0,
            player_fuel_capacity: 0.0,
            player_fuel_pct: 0.0,
            player_gap_ahead: 0.0,
            player_gap_behind: 0.0,
            player_lap_distance_pct: 0.0,
            latest_tire_data: StintTireSet::default(),
            player_is_in_car: false,
            player_incidents: 0,
            incident_limit: 0,
            caution_count: 0,
            caution_laps: 0,
            is_under_caution: false,
            is_oval: false,
            was_race_green: false,
            had_caution_since_last_green: false,
            session_flag_name: "GREEN".to_string(),
            debug_lap_number: 0,
            debug_lap_completed: 0,
            debug_lap_current_time: 0.0,
            debug_lap_last_time: 0.0,
            debug_lap_best_time: 0.0,
            debug_lap_best_lap_num: 0,
            debug_lap_dist: 0.0,
            debug_on_pit_road: true,
            debug_track_surface: 0,
            debug_is_on_track: false,
            debug_speed: 0.0,
            debug_rpm: 0.0,
            debug_gear: 0,
            debug_throttle: 0.0,
            debug_brake: 0.0,
            debug_session_time: 0.0,
            debug_session_state: 0,
            debug_session_flags: 0,
            debug_event_logs: Vec::new(),
        }
    }
}

impl WarRoomState {
    pub fn reset_session_data(&mut self, manual: bool) {
        // 1. Archive current session data before clearing if any laps or stints exist
        if !self.drivers.is_empty() && self.drivers.values().any(|d| !d.stints.is_empty() || d.total_laps_completed > 0) {
            self.session_history.push(SessionArchive {
                session_num: self.session_num,
                session_type: self.session_type.clone(),
                drivers: self.drivers.clone(),
                caution_count: self.caution_count,
                caution_laps: self.caution_laps,
            });
        }

        // 2. Snapshot player's historical fuel burn rate from practice/warmup before clearing stints
        for driver in self.drivers.values_mut() {
            let burn = driver.estimated_fuel_burn_ltr();
            if burn > 0.001 {
                driver.historical_fuel_burn_rate = burn;
            }
            driver.stints.clear();
            driver.total_laps_completed = 0;
            driver.last_processed_lap = 0;
            driver.last_known_lap_time = 0.0;
            driver.last_sf_crossing_time = 0.0;
            driver.exited_pits_on_current_lap = false;
            driver.ignored_lap_sdk = -1;
            driver.position = 0;
            driver.is_on_pit_road = true;
            driver.is_in_world = false;
            driver.is_initialized = false;
        }

        // 3. Reset session-level counters
        self.caution_count = 0;
        self.caution_laps = 0;
        self.is_under_caution = false;
        self.was_race_green = false;
        self.had_caution_since_last_green = false;
        self.session_flag_name = "GREEN".to_string();
        self.debug_lap_number = 0;
        self.debug_lap_completed = 0;
        self.debug_lap_current_time = 0.0;
        self.debug_lap_last_time = 0.0;
        self.debug_lap_best_time = 0.0;
        self.debug_lap_best_lap_num = 0;
        self.latest_tire_data = StintTireSet::default();

        if manual {
            self.debug_event_logs.push(format!(
                "[MANUAL REFRESH] Session {} (Phase #{}) cleared manually — Stints, lap counts, and best laps reset cleanly.",
                self.session_type, self.session_num
            ));
        }
    }

    pub fn reset_for_new_session(&mut self) {
        self.reset_session_data(false);
        self.drivers.clear();
        self.subsession_types.clear();
        self.subsession_rubber_states.clear();
        self.session_id = 0;
        self.subsession_id = 0;
        self.session_num = -1;
        self.session_unique_id = -1;
        self.needs_new_connection_reset = false;
        self.session_laps_total = 0;
        self.session_laps_remain = 0;
        self.sof = 0;
        self.car_name = "—".to_string();
        self.session_type = "—".to_string();
        self.player_setup_name = "Baseline".to_string();
        self.player_fuel_level = 0.0;
        self.player_fuel_capacity = 0.0;
        self.player_fuel_pct = 0.0;
        self.player_gap_ahead = 0.0;
        self.player_gap_behind = 0.0;
        self.player_lap_distance_pct = 0.0;
        self.player_is_in_car = false;
        self.player_incidents = 0;
        self.incident_limit = 0;
        self.debug_session_time = 0.0;
        self.debug_session_state = 0;
        self.debug_session_flags = 0;
        self.debug_event_logs.push(
            "[NEW SESSION LOADED] Previous session cleared — Stints, drivers, and telemetry reset cleanly for new session.".to_string(),
        );
    }

    pub fn check_session_transition(
        &mut self,
        cur_session_num: i32,
        cur_unique_id: i32,
        cur_session_state: i32,
        cur_session_time: f64,
        is_replay: bool,
    ) -> bool {
        let mut transition_detected = false;
        if self.needs_new_connection_reset {
            self.reset_for_new_session();
            transition_detected = true;
        }

        let is_first_init = self.session_num == -1;
        let is_subsession_changed = !is_first_init && cur_session_num != -1 && cur_session_num != self.session_num;
        let is_unique_id_changed = !is_first_init && self.session_unique_id != -1 && cur_unique_id != -1 && cur_unique_id != self.session_unique_id;
        let is_session_restarted = !is_first_init && self.debug_session_state >= 5 && cur_session_state > 0 && cur_session_state <= 3;
        let is_time_rewound = !is_first_init
            && !is_replay
            && self.debug_session_time > 15.0
            && cur_session_time > 0.0
            && (cur_session_time + 10.0) < self.debug_session_time;

        if is_first_init {
            if self.drivers.values().any(|d| !d.stints.is_empty() || d.total_laps_completed > 0) {
                self.reset_session_data(false);
                transition_detected = true;
            }
            if cur_session_num != -1 {
                self.session_num = cur_session_num;
            }
            if cur_unique_id != -1 {
                self.session_unique_id = cur_unique_id;
            }
            if let Some(stype) = self.subsession_types.get(&self.session_num) {
                self.session_type = format_session_type(stype);
            }
            let raw_rubber = self.subsession_rubber_states.get(&self.session_num).cloned().unwrap_or_default();
            self.track_state = format_track_state(&raw_rubber, self.track_wetness);
        } else if is_subsession_changed || is_unique_id_changed || is_session_restarted || is_time_rewound {
            let old_type = self.session_type.clone();
            let old_num = self.session_num;

            self.reset_session_data(false);

            if cur_session_num != -1 {
                self.session_num = cur_session_num;
            }
            if cur_unique_id != -1 {
                self.session_unique_id = cur_unique_id;
            }
            if let Some(stype) = self.subsession_types.get(&self.session_num) {
                self.session_type = format_session_type(stype);
            }
            let raw_rubber = self.subsession_rubber_states.get(&self.session_num).cloned().unwrap_or_default();
            self.track_state = format_track_state(&raw_rubber, self.track_wetness);

            let log_msg = format!(
                "[SESSION TRANSITION] {} (Phase #{}) -> {} (Phase #{}) · Stints & laps reset cleanly for new session.",
                old_type, old_num, self.session_type, self.session_num
            );
            self.debug_event_logs.push(log_msg);
            transition_detected = true;
        }

        if cur_session_time > 0.0 {
            self.debug_session_time = cur_session_time;
        }
        transition_detected
    }

    pub fn process_telemetry_frame(
        &mut self,
        frame: &TelemetryFrame<'_>,
        live_sample: Option<&iracing::telemetry::Sample>,
        transition_detected: bool,
    ) -> bool {
        self.debug_session_state = frame.cur_session_state;
        let is_race_session = self.session_type == "RACE";

        if let Some(flags) = frame.flags {
            self.debug_session_flags = flags;
            let is_racing_phase = frame.cur_session_state == STATE_RACING || frame.cur_session_state == 0;
            let has_fcy_flag = (flags & (FLAG_CAUTION | FLAG_CAUTION_WAVING)) != 0
                || (self.is_oval && is_race_session && (flags & (FLAG_YELLOW | FLAG_YELLOW_WAVING)) != 0)
                || (self.is_under_caution && (flags & FLAG_ONE_TO_GREEN) != 0 && (flags & (FLAG_GREEN | FLAG_START_GO)) == 0);
            let is_fcy = is_racing_phase && has_fcy_flag;

            let is_checkered = (flags & FLAG_CHECKERED) != 0 || frame.cur_session_state == STATE_CHECKERED;
            let is_white = (flags & FLAG_WHITE) != 0;
            let is_green_flag = (flags & (FLAG_GREEN | FLAG_START_GO)) != 0;
            let is_local_yellow = !self.is_oval && (flags & (FLAG_YELLOW | FLAG_YELLOW_WAVING)) != 0;

            if is_fcy {
                if !self.is_under_caution {
                    self.caution_count += 1;
                    self.is_under_caution = true;
                    self.had_caution_since_last_green = true;
                }
                self.session_flag_name = "CAUTION".to_string();
            } else {
                self.is_under_caution = false;
                if is_checkered {
                    self.session_flag_name = "CHECKERED".to_string();
                } else if is_white {
                    self.session_flag_name = "WHITE FLAG".to_string();
                } else if is_local_yellow {
                    self.session_flag_name = "LOCAL YELLOW".to_string();
                } else if frame.cur_session_state > 0 && frame.cur_session_state <= STATE_PARADE_LAPS && is_race_session {
                    self.session_flag_name = "PACING".to_string();
                } else if is_green_flag {
                    self.session_flag_name = "GREEN".to_string();
                } else {
                    self.session_flag_name = "RACING".to_string();
                }
            }
        }

        let cur_flags = self.debug_session_flags;
        let is_pre_race_pacing = is_race_session && (
            (frame.cur_session_state > 0 && frame.cur_session_state <= STATE_PARADE_LAPS)
            || ((cur_flags & (FLAG_GREEN_HELD | FLAG_ONE_TO_GREEN)) != 0
                && (cur_flags & (FLAG_GREEN | FLAG_START_GO)) == 0
                && !self.was_race_green)
        );
        let is_race_green = if is_race_session {
            !self.is_under_caution && !is_pre_race_pacing && (cur_flags & FLAG_RED) == 0
        } else {
            true
        };

        let caution_just_started = is_race_session && self.was_race_green && !is_race_green;
        let green_just_started = is_race_session && !self.was_race_green && is_race_green;
        let is_caution_restart = green_just_started && self.had_caution_since_last_green;

        let mut state_changed = transition_detected || caution_just_started || green_just_started;
        let mut pending_logs = Vec::new();
        let player_idx = self.player_car_idx;
        let player_setup_snapshot = self.player_setup_name.clone();
        let track_temp_snapshot = self.track_temp_raw.clone();
        let air_temp_snapshot = self.air_temp_raw.clone();
        let current_fuel_sample = frame.current_fuel_sample;
        let mut caution_laps_increment = 0;

        let player_surface = *frame.surfaces.get(player_idx).unwrap_or(&-1) as i32;
        self.debug_track_surface = player_surface;
        self.debug_on_pit_road = *frame.pit_road.get(player_idx).unwrap_or(&false);
        self.player_is_in_car = frame.is_on_track && !frame.is_replay && player_surface >= 1;

        for (idx, &is_pit) in frame.pit_road.iter().enumerate() {
            let pos = *frame.positions.get(idx).unwrap_or(&0) as i32;
            let surface = *frame.surfaces.get(idx).unwrap_or(&0) as i32;
            let in_world = surface >= 1;
            let car_dist_pct = *frame.lap_dist_pcts.get(idx).unwrap_or(&0.0);

            if !self.drivers.contains_key(&idx) && !in_world && pos <= 0 && idx != player_idx {
                continue;
            }

            let driver = self.drivers.entry(idx).or_insert_with(|| DriverState {
                car_idx: idx,
                driver_name: if idx == player_idx { "Player Car".to_string() } else { format!("Car #{}", idx) },
                car_num: idx.to_string(),
                irating: "—".to_string(),
                position: pos,
                is_on_pit_road: is_pit,
                is_in_world: in_world,
                is_initialized: false,
                total_laps_completed: 0,
                last_processed_lap: 0,
                last_known_lap_time: 0.0,
                last_sf_crossing_time: 0.0,
                exited_pits_on_current_lap: false,
                ignored_lap_sdk: -1,
                setup_name: if idx == player_idx && !player_setup_snapshot.is_empty() { player_setup_snapshot.clone() } else { "Baseline".to_string() },
                stints: Vec::new(),
                historical_fuel_burn_rate: 0.0,
            });

            driver.position = pos;
            if idx == player_idx && (driver.setup_name.is_empty() || driver.setup_name == "—" || driver.setup_name == "Baseline") {
                if !player_setup_snapshot.is_empty() && player_setup_snapshot != "Baseline" {
                    driver.setup_name = player_setup_snapshot.clone();
                }
            }

            let current_lap_sdk = *frame.laps_comp.get(idx).unwrap_or(&0) as i32;
            let last_lap_time = *frame.last_laps.get(idx).unwrap_or(&0.0) as f32;
            let _best_lap_time = *frame.fast_laps.get(idx).unwrap_or(&0.0) as f32;

            // Keep active stint's setup_name in sync with driver setup name
            let active_setup = if !driver.setup_name.is_empty() && driver.setup_name != "—" && driver.setup_name != "Baseline" {
                driver.setup_name.clone()
            } else if idx == player_idx && !player_setup_snapshot.is_empty() && player_setup_snapshot != "Baseline" {
                player_setup_snapshot.clone()
            } else {
                "Baseline".to_string()
            };

            // First-frame initialization guard (runs once per driver per session)
            if !driver.is_initialized {
                driver.is_initialized = true;
                driver.last_processed_lap = current_lap_sdk;
                driver.total_laps_completed = current_lap_sdk;
                driver.last_known_lap_time = last_lap_time;
                driver.is_on_pit_road = is_pit;
                driver.is_in_world = in_world;

                // If car is ALREADY on track driving when app connects (and in a race, only if green), start Stint 1 immediately!
                if !is_pit && in_world && driver.stints.is_empty() && is_race_green && !green_just_started {
                    let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                        let eff = driver.estimated_fuel_burn_ltr();
                        if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                    } else { 0.0 };
                    let prior_burn_rate = driver.estimated_fuel_burn_ltr();

                    driver.stints.push(StintData {
                        stint_number: 1,
                        start_lap: current_lap_sdk,
                        end_lap: current_lap_sdk,
                        out_lap_completed: current_lap_sdk > 0 || is_race_session,
                        awaiting_pace_lap_sf: false,
                        awaiting_pit_out_lap_sf: false,
                        laps_completed: 0,
                        lap_times: Vec::new(),
                        fuel_start: current_fuel_sample,
                        fuel_used: 0.0,
                        last_lap: 0.0,
                        fast_lap: 0.0,
                        is_active: true,
                        is_expanded: false,
                        tires: StintTireSet::default(),
                        setup_name: active_setup.clone(),
                        track_temp_at_start: track_temp_snapshot.clone(),
                        air_temp_at_start: air_temp_snapshot.clone(),
                        notes: String::new(),
                        crossing_fuel_level: current_fuel_sample,
                        laps_of_fuel_crossing: initial_laps_of_fuel,
                        latched_fuel_burn_rate: prior_burn_rate,
                        lap_fuel_burns: Vec::new(),
                        prev_crossing_fuel: current_fuel_sample,
                    });
                    if idx == player_idx {
                        pending_logs.push(format!("[LIVE ATTACH] Connected to on-track session at Lap {}. Stint S1 active.", current_lap_sdk));
                    }
                    state_changed = true;
                }
            }

            let was_in_pit = driver.is_on_pit_road;
            let was_in_world = driver.is_in_world;

            if (was_in_pit && !is_pit && in_world) || (!was_in_world && in_world && !is_pit) {
                driver.exited_pits_on_current_lap = true;
            }
            if is_pit || !in_world {
                driver.exited_pits_on_current_lap = false;
            }

            if let Some(active) = driver.stints.last_mut() {
                if active.is_active && (active.setup_name.is_empty() || active.setup_name == "—" || active.setup_name == "Baseline") {
                    if active_setup != "Baseline" {
                        active.setup_name = active_setup.clone();
                    }
                }
            }

            // Sample active dynamic on-track pressures and fuel for player car
            if idx == player_idx && !is_pit && in_world {
                let [cur_lf_p, cur_rf_p, cur_lr_p, cur_rr_p] = frame.player_live_pressures;

                if let Some(active) = driver.stints.last_mut() {
                    if active.is_active {
                        if cur_lf_p > 1.0 {
                            active.tires.lf.pressure = cur_lf_p;
                            active.tires.rf.pressure = cur_rf_p;
                            active.tires.lr.pressure = cur_lr_p;
                            active.tires.rr.pressure = cur_rr_p;
                        }
                        if active.fuel_start <= 0.0 && current_fuel_sample > 0.0 {
                            active.fuel_start = current_fuel_sample;
                        }
                        if active.fuel_start > 0.0 && current_fuel_sample > 0.0 && current_fuel_sample <= active.fuel_start {
                            active.fuel_used = (active.fuel_start - current_fuel_sample).max(active.fuel_used);
                        }
                    }
                }
            }

            // 0A. FULL COURSE CAUTION DEPLOYED IN RACE (Close active green-flag stint for ALL drivers)
            if caution_just_started {
                let mut aborted = false;
                let mut stint_num = 0;
                let mut completed_laps = 0;

                if let Some(active) = driver.stints.last_mut() {
                    if active.is_active {
                        active.is_active = false;
                        active.end_lap = current_lap_sdk;
                        stint_num = active.stint_number;
                        completed_laps = active.laps_completed;
                        if idx == player_idx && active.fuel_start > 0.0 && current_fuel_sample > 0.0 && current_fuel_sample <= active.fuel_start {
                            if active.fuel_used <= 0.0 {
                                active.fuel_used = (active.fuel_start - current_fuel_sample).max(0.0);
                            }
                            active.latched_fuel_burn_rate = active.fuel_per_lap();
                        }
                        if active.laps_completed == 0 {
                            aborted = true;
                        }
                    }
                }

                if aborted {
                    driver.stints.pop();
                    if idx == player_idx {
                        pending_logs.push(format!("[CAUTION DEPLOYED] Aborted 0-lap Stint S{} discarded.", stint_num));
                    }
                } else if stint_num > 0 && idx == player_idx {
                    pending_logs.push(format!("[CAUTION DEPLOYED] Stint S{} Closed ({} Green Laps) — Full course caution active.", stint_num, completed_laps));
                }
                state_changed = true;
            }

            // 0B. GREEN FLAG START / RESTART IN RACE (Start new stint for ALL on-track drivers)
            if green_just_started && !is_pit && in_world {
                if let Some(active) = driver.stints.last_mut() {
                    active.is_active = false;
                    active.end_lap = current_lap_sdk;
                }
                if let Some(last) = driver.stints.last() {
                    if last.laps_completed == 0 {
                        driver.stints.pop();
                    }
                }

                let crossed_sf_this_frame = current_lap_sdk > driver.last_processed_lap && current_lap_sdk > 0;
                if crossed_sf_this_frame {
                    driver.last_sf_crossing_time = frame.cur_session_time;
                    driver.exited_pits_on_current_lap = false;
                    driver.ignored_lap_sdk = current_lap_sdk;
                    if idx == player_idx && is_caution_restart {
                        caution_laps_increment += 1;
                    }
                }

                let awaiting_pace_lap_sf = if is_caution_restart {
                    if crossed_sf_this_frame {
                        false
                    } else if car_dist_pct >= 0.25 {
                        true
                    } else if driver.last_sf_crossing_time > 0.0 && (frame.cur_session_time - driver.last_sf_crossing_time) > 8.0 {
                        true
                    } else {
                        if current_lap_sdk > 0 {
                            driver.ignored_lap_sdk = current_lap_sdk;
                        }
                        false
                    }
                } else {
                    false
                };
                let awaiting_pit_out_lap_sf = !awaiting_pace_lap_sf && driver.exited_pits_on_current_lap && current_lap_sdk > 0;

                let new_stint_num = driver.stints.len() as i32 + 1;
                let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                    let eff = driver.estimated_fuel_burn_ltr();
                    if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                } else { 0.0 };
                let prior_burn_rate = driver.estimated_fuel_burn_ltr();

                driver.stints.push(StintData {
                    stint_number: new_stint_num,
                    start_lap: current_lap_sdk,
                    end_lap: current_lap_sdk,
                    out_lap_completed: !awaiting_pace_lap_sf && !awaiting_pit_out_lap_sf,
                    awaiting_pace_lap_sf,
                    awaiting_pit_out_lap_sf,
                    laps_completed: 0,
                    lap_times: Vec::new(),
                    fuel_start: current_fuel_sample,
                    fuel_used: 0.0,
                    last_lap: 0.0,
                    fast_lap: 0.0,
                    is_active: true,
                    is_expanded: false,
                    tires: StintTireSet::default(),
                    setup_name: active_setup.clone(),
                    track_temp_at_start: track_temp_snapshot.clone(),
                    air_temp_at_start: air_temp_snapshot.clone(),
                    notes: String::new(),
                    crossing_fuel_level: current_fuel_sample,
                    laps_of_fuel_crossing: initial_laps_of_fuel,
                    latched_fuel_burn_rate: prior_burn_rate,
                    lap_fuel_burns: Vec::new(),
                    prev_crossing_fuel: current_fuel_sample,
                });
                driver.last_processed_lap = current_lap_sdk;
                driver.total_laps_completed = current_lap_sdk;
                driver.last_known_lap_time = last_lap_time;

                if idx == player_idx {
                    pending_logs.push(format!("[GREEN FLAG] Stint S{} Started at Lap {}", new_stint_num, current_lap_sdk));
                }
                state_changed = true;
            }

            // 1. CAR EXITED PITS / ENTERED TRACK (Begin Stint if green)
            if (was_in_pit && !is_pit && in_world) || (!was_in_world && in_world && !is_pit) {
                if is_race_green && !green_just_started {
                    if let Some(active) = driver.stints.last_mut() {
                        active.is_active = false;
                        active.end_lap = current_lap_sdk;
                    }

                    // Discard previous 0-lap stint if aborted before completing any timed laps
                    if let Some(last) = driver.stints.last() {
                        if last.laps_completed == 0 {
                            driver.stints.pop();
                        }
                    }

                    let new_stint_num = driver.stints.len() as i32 + 1;
                    let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                        let eff = driver.estimated_fuel_burn_ltr();
                        if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                    } else { 0.0 };
                    let prior_burn_rate = driver.estimated_fuel_burn_ltr();
                    let awaiting_pit_out_lap_sf = is_race_session && current_lap_sdk > 0;

                    driver.stints.push(StintData {
                        stint_number: new_stint_num,
                        start_lap: current_lap_sdk,
                        end_lap: current_lap_sdk,
                        out_lap_completed: false,
                        awaiting_pace_lap_sf: false,
                        awaiting_pit_out_lap_sf,
                        laps_completed: 0,
                        lap_times: Vec::new(),
                        fuel_start: current_fuel_sample,
                        fuel_used: 0.0,
                        last_lap: 0.0,
                        fast_lap: 0.0,
                        is_active: true,
                        is_expanded: false,
                        tires: StintTireSet::default(),
                        setup_name: active_setup.clone(),
                        track_temp_at_start: track_temp_snapshot.clone(),
                        air_temp_at_start: air_temp_snapshot.clone(),
                        notes: String::new(),
                        crossing_fuel_level: current_fuel_sample,
                        laps_of_fuel_crossing: initial_laps_of_fuel,
                        latched_fuel_burn_rate: prior_burn_rate,
                        lap_fuel_burns: Vec::new(),
                        prev_crossing_fuel: 0.0,
                    });
                    if idx == player_idx {
                        pending_logs.push(format!("[PIT EXIT] Stint S{} Started at Lap {}", new_stint_num, current_lap_sdk));
                    }
                    driver.last_processed_lap = current_lap_sdk;
                    driver.total_laps_completed = current_lap_sdk;
                    driver.last_known_lap_time = last_lap_time;
                }
                state_changed = true;
            }

            // 2. CAR ENTERED PITS OR EXITED CAR (Stint Ends / Reset)
            if (!was_in_pit && is_pit) || (was_in_world && !in_world) {
                let mut aborted = false;
                let mut stint_num = 0;
                let mut completed_laps = 0;

                if let Some(active) = driver.stints.last_mut() {
                    if active.is_active {
                        active.is_active = false;
                        active.end_lap = current_lap_sdk;
                        stint_num = active.stint_number;
                        completed_laps = active.laps_completed;
                        if idx == player_idx && active.fuel_start > 0.0 && current_fuel_sample > 0.0 && current_fuel_sample <= active.fuel_start {
                            if active.fuel_used <= 0.0 {
                                active.fuel_used = (active.fuel_start - current_fuel_sample).max(0.0);
                            }
                            active.latched_fuel_burn_rate = active.fuel_per_lap();
                        }
                        if active.laps_completed == 0 {
                            aborted = true;
                        }
                    }
                }

                if aborted {
                    driver.stints.pop();
                    if idx == player_idx {
                        pending_logs.push(format!("[ABORTED RUN] Stint S{} had 0 completed laps; discarded to prevent empty stint line.", stint_num));
                    }
                } else if stint_num > 0 && idx == player_idx {
                    pending_logs.push(format!("[PIT ENTRY] Stint S{} Closed (Total {} Laps) · Awaiting Pit Stop Service", stint_num, completed_laps));
                }

                if is_race_green {
                    driver.last_processed_lap = current_lap_sdk;
                    driver.total_laps_completed = current_lap_sdk;
                    driver.last_known_lap_time = last_lap_time;
                }
                state_changed = true;
            }

            // 3. NEW LAP TIME / COUNT EMITTED BY IRACING ON TRACK (Direct SDK Fire)
            let is_new_lap_time = (last_lap_time - driver.last_known_lap_time).abs() > 0.0005 && last_lap_time > 3.0;
            let is_new_lap_count = current_lap_sdk > driver.last_processed_lap && current_lap_sdk > 0;

            if (is_new_lap_count || is_new_lap_time) && in_world {
                driver.last_sf_crossing_time = frame.cur_session_time;
                if !is_pit {
                    driver.exited_pits_on_current_lap = false;
                }
            }

            if !is_race_green {
                // 3A. Under caution or pre-race parade laps: do not count laps toward any stint
                if (is_new_lap_count || is_new_lap_time) && in_world {
                    let is_already_ignored = current_lap_sdk > 0 && current_lap_sdk == driver.ignored_lap_sdk;
                    if current_lap_sdk > 0 {
                        driver.ignored_lap_sdk = current_lap_sdk;
                    }
                    if idx == player_idx {
                        if self.is_under_caution && !is_already_ignored {
                            caution_laps_increment += 1;
                            pending_logs.push(format!("[CAUTION LAP] Lap {} completed under caution — ignored for green-flag stint.", current_lap_sdk));
                        }
                        if current_fuel_sample > 0.0 {
                            if let Some(last_stint) = driver.stints.last_mut() {
                                last_stint.prev_crossing_fuel = current_fuel_sample;
                                last_stint.crossing_fuel_level = current_fuel_sample;
                                let burn_rate = last_stint.fuel_per_lap();
                                if burn_rate > 0.001 {
                                    last_stint.laps_of_fuel_crossing = (current_fuel_sample / burn_rate).max(0.0);
                                }
                            }
                        }
                    }
                    driver.last_known_lap_time = last_lap_time;
                    driver.last_processed_lap = current_lap_sdk;
                    driver.total_laps_completed = current_lap_sdk;
                    state_changed = true;
                }
            } else if current_lap_sdk > 0 && current_lap_sdk == driver.ignored_lap_sdk && (is_new_lap_time || is_new_lap_count) {
                // 3B. Delayed last_lap_time update for a caution/pace/out-lap that was already ignored at S/F
                driver.last_known_lap_time = last_lap_time;
                driver.last_processed_lap = current_lap_sdk;
                driver.total_laps_completed = current_lap_sdk;
            } else if (is_new_lap_count || is_new_lap_time) && !is_pit && in_world && driver.stints.last().map_or(false, |st| st.is_active && (st.awaiting_pace_lap_sf || st.awaiting_pit_out_lap_sf)) {
                // 3C. First S/F crossing after a caution restart (finishing pace lap) or race green-flag pit exit (finishing pit out-lap)
                if let Some(active) = driver.stints.last_mut() {
                    let was_pace_lap = active.awaiting_pace_lap_sf;
                    active.awaiting_pace_lap_sf = false;
                    active.awaiting_pit_out_lap_sf = false;
                    active.out_lap_completed = true;
                    active.start_lap = current_lap_sdk;
                    active.end_lap = current_lap_sdk;
                    if current_lap_sdk > 0 {
                        driver.ignored_lap_sdk = current_lap_sdk;
                    }
                    if current_fuel_sample > 0.0 {
                        active.fuel_start = current_fuel_sample;
                        active.prev_crossing_fuel = current_fuel_sample;
                        active.crossing_fuel_level = current_fuel_sample;
                        if active.latched_fuel_burn_rate > 0.001 {
                            active.laps_of_fuel_crossing = (current_fuel_sample / active.latched_fuel_burn_rate).max(0.0);
                        }
                    }
                    if idx == player_idx {
                        if was_pace_lap {
                            caution_laps_increment += 1;
                            pending_logs.push(format!("[RESTART S/F] Stint S{}: Pace lap {} dropped at S/F. Green Lap 1 started.", active.stint_number, current_lap_sdk));
                        } else {
                            pending_logs.push(format!("[OUT LAP COMPLETED] Stint S{}: Pit out-lap {} dropped at S/F. Flying Lap 1 started.", active.stint_number, current_lap_sdk));
                        }
                    }
                }
                driver.last_known_lap_time = last_lap_time;
                driver.last_processed_lap = current_lap_sdk;
                driver.total_laps_completed = current_lap_sdk;
                state_changed = true;
            } else {
                // 3D. UNTIMED OUT-LAP COMPLETION (Start/Finish crossed for the first time since pit exit in Practice/Qualifying)
                if is_new_lap_count && !is_pit && in_world && !is_new_lap_time {
                    if let Some(active) = driver.stints.last_mut() {
                        if active.is_active && !active.out_lap_completed {
                            active.out_lap_completed = true;
                            active.prev_crossing_fuel = current_fuel_sample;
                            active.crossing_fuel_level = current_fuel_sample;
                            if active.latched_fuel_burn_rate > 0.001 {
                                active.laps_of_fuel_crossing = (current_fuel_sample / active.latched_fuel_burn_rate).max(0.0);
                            }
                            driver.last_processed_lap = current_lap_sdk;
                            driver.total_laps_completed = current_lap_sdk;
                            if idx == player_idx {
                                pending_logs.push(format!("[OUT LAP COMPLETED] Stint S{}: Crossed start/finish at Lap {}. Latched fuel at {:.2} L for Flying Lap 1", active.stint_number, current_lap_sdk, current_fuel_sample));
                            }
                            state_changed = true;
                        }
                    }
                }

                if (is_new_lap_time || (is_new_lap_count && (last_lap_time - driver.last_known_lap_time).abs() > 0.0005)) && !is_pit && in_world {
                    if driver.stints.is_empty() || !driver.stints.last().map_or(false, |st| st.is_active) {
                        let new_stint_num = driver.stints.len() as i32 + 1;
                        let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                            let eff = driver.estimated_fuel_burn_ltr();
                            if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                        } else { 0.0 };
                        let prior_burn_rate = driver.estimated_fuel_burn_ltr();

                        driver.stints.push(StintData {
                            stint_number: new_stint_num,
                            start_lap: current_lap_sdk - 1,
                            end_lap: current_lap_sdk,
                            out_lap_completed: true,
                            awaiting_pace_lap_sf: false,
                            awaiting_pit_out_lap_sf: false,
                            laps_completed: 0,
                            lap_times: Vec::new(),
                            fuel_start: current_fuel_sample,
                            fuel_used: 0.0,
                            last_lap: 0.0,
                            fast_lap: 0.0,
                            is_active: true,
                            is_expanded: false,
                            tires: StintTireSet::default(),
                            setup_name: active_setup.clone(),
                            track_temp_at_start: track_temp_snapshot.clone(),
                            air_temp_at_start: air_temp_snapshot.clone(),
                            notes: String::new(),
                            crossing_fuel_level: current_fuel_sample,
                            laps_of_fuel_crossing: initial_laps_of_fuel,
                            latched_fuel_burn_rate: prior_burn_rate,
                            lap_fuel_burns: Vec::new(),
                            prev_crossing_fuel: current_fuel_sample,
                        });
                    }

                    if let Some(active) = driver.stints.last_mut() {
                        if last_lap_time > 3.0 && last_lap_time < 360.0 {
                            let was_uncompleted_out_lap = !active.out_lap_completed;
                            active.out_lap_completed = true;
                            active.lap_times.push(last_lap_time);
                            active.last_lap = last_lap_time;
                            active.laps_completed = active.lap_times.len() as i32;
                            active.end_lap = current_lap_sdk;

                            if active.fast_lap <= 0.0 || last_lap_time < active.fast_lap {
                                active.fast_lap = last_lap_time;
                            }
                            if active.fuel_start <= 0.0 && current_fuel_sample > 0.0 {
                                active.fuel_start = current_fuel_sample;
                            }

                            // Latch fuel at start/finish line crossing: calculate exact timed-lap burn delta
                            if idx == player_idx && current_fuel_sample > 0.0 {
                                let is_anomalous_slow_lap = active.fast_lap > 3.0 && last_lap_time > active.fast_lap * 1.35;

                                if was_uncompleted_out_lap {
                                    // The sim timed the out-lap itself: latch fuel at start/finish line so Lap 1 is measured cleanly
                                    active.prev_crossing_fuel = current_fuel_sample;
                                    active.crossing_fuel_level = current_fuel_sample;
                                    if active.latched_fuel_burn_rate > 0.001 {
                                        active.laps_of_fuel_crossing = (current_fuel_sample / active.latched_fuel_burn_rate).max(0.0);
                                    }
                                    pending_logs.push(format!("[OUT LAP COMPLETED] Stint S{}: Timed out-lap ({:.3}s). Latched fuel at {:.2} L for Flying Lap 1", active.stint_number, last_lap_time, current_fuel_sample));
                                } else if !is_anomalous_slow_lap {
                                    // Clean green-flag flying lap burn
                                    if active.prev_crossing_fuel > 0.0 && active.prev_crossing_fuel >= current_fuel_sample {
                                        let lap_burn = active.prev_crossing_fuel - current_fuel_sample;
                                        if lap_burn > 0.05 && lap_burn < 25.0 {
                                            active.lap_fuel_burns.push(lap_burn);
                                            active.fuel_used += lap_burn;
                                            active.latched_fuel_burn_rate = active.fuel_per_lap();
                                        }
                                    }
                                    active.prev_crossing_fuel = current_fuel_sample;
                                    active.crossing_fuel_level = current_fuel_sample;
                                    let burn_rate = active.fuel_per_lap();
                                    if burn_rate > 0.001 {
                                        active.laps_of_fuel_crossing = (current_fuel_sample / burn_rate).max(0.0);
                                    }
                                } else {
                                    // Anomalous slow lap (e.g. spin): update fuel level without distorting green-flag fuel burn average
                                    active.prev_crossing_fuel = current_fuel_sample;
                                    active.crossing_fuel_level = current_fuel_sample;
                                }
                            }

                            if idx == player_idx {
                                pending_logs.push(format!("[SDK LAP EMISSION] S{} Lap {}: Time = {:.3} (Prev = {:.3})", active.stint_number, active.laps_completed, last_lap_time, driver.last_known_lap_time));
                            }
                            state_changed = true;
                        }
                    }
                    driver.last_known_lap_time = last_lap_time;
                    driver.last_processed_lap = current_lap_sdk;
                    driver.total_laps_completed = current_lap_sdk;
                }
            }

            driver.is_on_pit_road = is_pit;
            driver.is_in_world = in_world;
        }

        self.caution_laps += caution_laps_increment;
        self.was_race_green = is_race_green;
        if green_just_started {
            self.had_caution_since_last_green = false;
        }

        // Continuous Pit Stall / Garage Tire Capture for Player (only after completed laps)
        // Latch tire wear & temperatures when the car is stationary in the pit stall or in the garage
        let is_in_stall_or_garage = frame.surfaces.get(player_idx).map_or(false, |&s| s == 2)
            || (self.debug_on_pit_road && self.debug_speed < 2.0)
            || !self.debug_is_on_track
            || !self.player_is_in_car;

        if is_in_stall_or_garage {
            if let Some(live) = live_sample {
                let player_drv_opt = self.drivers.get(&player_idx);
                let has_completed_laps = player_drv_opt.and_then(|d| d.stints.last()).map_or(false, |st| st.laps_completed > 0);

                if has_completed_laps {
                    let prev_hot = player_drv_opt.and_then(|d| d.stints.last()).map(|st| &st.tires);
                    let pit_tires = read_player_tires(live, prev_hot);
                    if pit_tires.is_valid() {
                        self.latest_tire_data = pit_tires.clone();
                        if let Some(player_drv) = self.drivers.get_mut(&player_idx) {
                            if let Some(last_stint) = player_drv.stints.last_mut() {
                                if !last_stint.is_active {
                                    last_stint.tires = pit_tires;
                                    state_changed = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        for log in pending_logs {
            self.debug_event_logs.push(log);
        }
        while self.debug_event_logs.len() > 50 {
            self.debug_event_logs.remove(0);
        }

        state_changed
    }
}

pub struct TelemetryFrame<'a> {
    pub cur_session_state: i32,
    pub flags: Option<u32>,
    pub cur_session_time: f64,
    pub current_fuel_sample: f32,
    pub player_live_pressures: [f32; 4],
    pub is_on_track: bool,
    pub is_replay: bool,
    pub pit_road: &'a [bool],
    pub laps_comp: &'a [i32],
    pub last_laps: &'a [f32],
    pub fast_laps: &'a [f32],
    pub positions: &'a [i32],
    pub surfaces: &'a [i32],
    pub lap_dist_pcts: &'a [f32],
}

pub fn is_sim_header_ready(header: &iracing::telemetry::Header) -> bool {
    // status bit 0 == irsdk_stConnected (1). Also require valid variable & buffer descriptors.
    (header.status & 1) != 0
        && header.n_vars > 0
        && header.buffer_length > 0
        && header.header_offset > 0
}

pub fn is_telemetry_sample_ready(sample: &iracing::telemetry::Sample) -> bool {
    sample.has("SessionNum")
        && sample.has("CarIdxLapCompleted")
        && sample.has("CarIdxTrackSurface")
        && sample.has("CarIdxOnPitRoad")
}

#[cfg(target_os = "windows")]
pub struct LiveSimConnection {
    mapping: *mut std::ffi::c_void,
    view: *mut std::ffi::c_void,
    event_handle: *mut std::ffi::c_void,
    pub last_session_info_version: i32,
}

#[cfg(target_os = "windows")]
impl LiveSimConnection {
    pub fn connect() -> Option<Self> {
        type HANDLE = *mut std::ffi::c_void;
        type LPVOID = *mut std::ffi::c_void;
        type LPCWSTR = *const u16;

        extern "system" {
            fn OpenFileMappingW(desired_access: u32, inherit_handle: i32, name: LPCWSTR) -> HANDLE;
            fn MapViewOfFile(
                file_mapping_object: HANDLE,
                desired_access: u32,
                file_offset_high: u32,
                file_offset_low: u32,
                number_of_bytes_to_map: usize,
            ) -> LPVOID;
            fn CreateEventW(
                event_attributes: *mut std::ffi::c_void,
                manual_reset: i32,
                initial_state: i32,
                name: LPCWSTR,
            ) -> HANDLE;
            fn CloseHandle(handle: HANDLE) -> i32;
        }

        const FILE_MAP_READ: u32 = 0x0004;
        const TELEMETRY_PATH: &str = "Local\\IRSDKMemMapFileName";
        const DATA_EVENT_NAME: &str = "Local\\IRSDKDataValidEvent";

        let mut path: Vec<u16> = TELEMETRY_PATH.encode_utf16().collect();
        path.push(0);

        let mapping = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, path.as_ptr()) };
        if mapping.is_null() {
            return None;
        }

        let view = unsafe { MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0) };
        if view.is_null() {
            unsafe {
                CloseHandle(mapping);
            }
            return None;
        }

        let mut event_name: Vec<u16> = DATA_EVENT_NAME.encode_utf16().collect();
        event_name.push(0);
        let event_handle = unsafe { CreateEventW(std::ptr::null_mut(), 0, 0, event_name.as_ptr()) };

        let conn = Self {
            mapping,
            view,
            event_handle,
            last_session_info_version: -1,
        };

        // Ensure iRacing has finished its loading screen and populated live telemetry variables
        let header = unsafe { Connection::read_header(conn.view as *const _) };
        if !is_sim_header_ready(&header) {
            return None;
        }

        match header.telemetry(conn.view as *const _) {
            Ok(sample) if is_telemetry_sample_ready(&sample) => Some(conn),
            _ => None,
        }
    }

    pub fn next_sample(&mut self, timeout_ms: u32) -> Option<(iracing::telemetry::Header, iracing::telemetry::Sample)> {
        type HANDLE = *mut std::ffi::c_void;
        extern "system" {
            fn WaitForSingleObject(handle: HANDLE, milliseconds: u32) -> u32;
        }

        if !self.event_handle.is_null() {
            let signal = unsafe { WaitForSingleObject(self.event_handle, timeout_ms) };
            // 0x00 = WAIT_OBJECT_0, 0x102 = WAIT_TIMEOUT (still valid to read live shared memory if status & 1 != 0)
            if signal != 0x00 && signal != 0x102 {
                return None;
            }
        } else {
            thread::sleep(std::time::Duration::from_millis(16));
        }

        // Always re-read the live Header from shared memory on every tick so n_vars, status, and triple-buffers are never stale
        let header = unsafe { Connection::read_header(self.view as *const _) };
        if !is_sim_header_ready(&header) {
            return None;
        }

        let sample = header.telemetry(self.view as *const _).ok()?;
        if !is_telemetry_sample_ready(&sample) {
            return None;
        }

        Some((header, sample))
    }

    pub fn read_yaml(&self) -> Option<(i32, String)> {
        use encoding::all::ISO_8859_1;
        use encoding::{DecoderTrap, Encoding};
        use std::slice::from_raw_parts;

        let header = unsafe { Connection::read_header(self.view as *const _) };
        if !is_sim_header_ready(&header) {
            return None;
        }
        let info_len = header.session_info_length as usize;
        let info_offset = header.session_info_offset as usize;
        if info_len == 0 || info_offset == 0 {
            return None;
        }

        let start = (self.view as usize + info_offset) as *const u8;
        let raw_slice: &[u8] = unsafe { from_raw_parts(start, info_len) };
        let valid_len = raw_slice.iter().position(|&b| b == 0).unwrap_or(raw_slice.len());
        if valid_len == 0 {
            return None;
        }

        let content = ISO_8859_1.decode(&raw_slice[..valid_len], DecoderTrap::Replace).ok()?;
        Some((header.session_info_version, content))
    }
}

#[cfg(target_os = "windows")]
impl Drop for LiveSimConnection {
    fn drop(&mut self) {
        type HANDLE = *mut std::ffi::c_void;
        type LPVOID = *mut std::ffi::c_void;
        extern "system" {
            fn CloseHandle(handle: HANDLE) -> i32;
            fn UnmapViewOfFile(base_address: LPVOID) -> i32;
        }

        unsafe {
            if !self.view.is_null() {
                UnmapViewOfFile(self.view);
                self.view = std::ptr::null_mut();
            }
            if !self.mapping.is_null() {
                CloseHandle(self.mapping);
                self.mapping = std::ptr::null_mut();
            }
            if !self.event_handle.is_null() {
                CloseHandle(self.event_handle);
                self.event_handle = std::ptr::null_mut();
            }
        }
    }
}

#[cfg(target_os = "windows")]
pub fn read_raw_iracing_yaml() -> Option<String> {
    use encoding::all::ISO_8859_1;
    use encoding::{DecoderTrap, Encoding};
    use std::slice::from_raw_parts;

    type HANDLE = *mut std::ffi::c_void;
    type LPVOID = *mut std::ffi::c_void;
    type LPCWSTR = *const u16;

    extern "system" {
        fn OpenFileMappingW(desired_access: u32, inherit_handle: i32, name: LPCWSTR) -> HANDLE;
        fn MapViewOfFile(file_mapping_object: HANDLE, desired_access: u32, file_offset_high: u32, file_offset_low: u32, number_of_bytes_to_map: usize) -> LPVOID;
        fn CloseHandle(handle: HANDLE) -> i32;
        fn UnmapViewOfFile(base_address: LPVOID) -> i32;
    }

    const FILE_MAP_READ: u32 = 0x0004;
    const TELEMETRY_PATH: &str = "Local\\IRSDKMemMapFileName";

    let mut path: Vec<u16> = TELEMETRY_PATH.encode_utf16().collect();
    path.push(0);

    let mapping = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, path.as_ptr()) };
    if mapping.is_null() {
        return None;
    }

    let view = unsafe { MapViewOfFile(mapping, FILE_MAP_READ, 0, 0, 0) };
    if view.is_null() {
        unsafe { CloseHandle(mapping); }
        return None;
    }

    let header_ptr = view as *const i32;
    let (status, info_len, info_offset) = unsafe {
        let st = *header_ptr.add(1);
        let len = *header_ptr.add(4);
        let offset = *header_ptr.add(5);
        (st, len as usize, offset as usize)
    };

    if (status & 1) == 0 || info_len == 0 || info_offset == 0 {
        unsafe {
            UnmapViewOfFile(view);
            CloseHandle(mapping);
        }
        return None;
    }

    let start = (view as usize + info_offset) as *const u8;
    let raw_slice: &[u8] = unsafe { from_raw_parts(start, info_len) };
    let valid_len = raw_slice.iter().position(|&b| b == 0).unwrap_or(raw_slice.len());
    if valid_len == 0 {
        unsafe {
            UnmapViewOfFile(view);
            CloseHandle(mapping);
        }
        return None;
    }

    let content = match ISO_8859_1.decode(&raw_slice[..valid_len], DecoderTrap::Replace) {
        Ok(v) => v,
        Err(_) => {
            unsafe {
                UnmapViewOfFile(view);
                CloseHandle(mapping);
            }
            return None;
        }
    };

    unsafe {
        UnmapViewOfFile(view);
        CloseHandle(mapping);
    }

    Some(content)
}

#[cfg(not(target_os = "windows"))]
pub fn read_raw_iracing_yaml() -> Option<String> {
    None
}

fn is_real_track_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty()
        && !matches!(
            trimmed,
            "Waiting for iRacing..."
                | "Waiting for connection..."
                | "Connecting to Session..."
                | "iRacing Live Session"
                | "—"
                | "..."
        )
}

pub fn parse_iracing_session_yaml(yaml_str: &str, s: &mut WarRoomState) -> bool {
    let root: serde_yaml::Value = match serde_yaml::from_str(yaml_str) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let weekend = &root["WeekendInfo"];
    let new_track_name = weekend["TrackDisplayName"].as_str().unwrap_or("").trim();
    let new_session_id = weekend["SessionID"].as_i64().unwrap_or(0) as i32;
    let new_subsession_id = weekend["SubSessionID"].as_i64().unwrap_or(0) as i32;

    let mut new_subsession_types: HashMap<i32, String> = HashMap::new();
    let mut new_subsession_rubber: HashMap<i32, String> = HashMap::new();
    if let Some(sessions) = root["SessionInfo"]["Sessions"].as_sequence() {
        for sess in sessions {
            let s_num = sess["SessionNum"].as_i64().unwrap_or(-1) as i32;
            if s_num >= 0 {
                if let Some(stype) = sess["SessionType"].as_str() {
                    new_subsession_types.insert(s_num, stype.to_string());
                }
                if let Some(rstate) = sess["SessionTrackRubberState"].as_str() {
                    if !rstate.is_empty() {
                        new_subsession_rubber.insert(s_num, rstate.to_string());
                    }
                }
            }
        }
    }

    // Detect if this YAML belongs to a brand new session / weekend (e.g., closed race and loaded new practice)
    let has_prior_session = s.session_num != -1
        || !s.drivers.is_empty()
        || s.session_id != 0
        || s.subsession_id != 0;

    let track_changed = is_real_track_name(&s.track_name)
        && is_real_track_name(new_track_name)
        && s.track_name != new_track_name;

    let session_id_changed = new_session_id != s.session_id
        && (s.session_id != 0 || (new_session_id != 0 && s.session_num != -1));

    let subsession_id_changed = new_subsession_id != s.subsession_id
        && (s.subsession_id != 0 || (new_subsession_id != 0 && s.session_num != -1));

    let subsession_table_changed = !s.subsession_types.is_empty()
        && !new_subsession_types.is_empty()
        && ((s.session_num >= 0 && !new_subsession_types.contains_key(&s.session_num))
            || new_subsession_types
                .iter()
                .any(|(k, v)| s.subsession_types.get(k).map_or(false, |old_v| old_v != v)));

    if s.needs_new_connection_reset
        || (has_prior_session
            && (track_changed || session_id_changed || subsession_id_changed || subsession_table_changed))
    {
        s.reset_for_new_session();
    }

    // 1. Weekend Info
    if !new_track_name.is_empty() {
        s.track_name = new_track_name.to_string();
    }
    if let Some(len) = weekend["TrackLength"].as_str() {
        if !len.is_empty() {
            s.track_length = len.to_string();
        }
    }
    if let Some(w) = weekend["TrackWeatherType"].as_str() {
        if !w.is_empty() {
            s.weather_type = w.to_string();
        }
    }
    if let Some(t) = weekend["TrackAirTemp"].as_str() {
        s.air_temp_raw = t.to_string();
    }
    if let Some(t) = weekend["TrackSurfaceTemp"].as_str() {
        s.track_temp_raw = t.to_string();
    }
    if let Some(w) = weekend["TrackWindVel"].as_str() {
        s.wind_speed_raw = w.to_string();
    }
    let opt_wind = weekend["WeekendOptions"]["WindDirection"].as_str();
    let trk_wind = weekend["TrackWindDir"].as_str();
    if let Some(dir) = opt_wind.or(trk_wind) {
        s.wind_dir_raw = dir.to_string();
    }
    if let Some(rh) = weekend["WeekendOptions"]["RelativeHumidity"].as_str() {
        s.relative_humidity_raw = rh.to_string();
    }
    if weekend["SessionID"].as_i64().is_some() {
        s.session_id = new_session_id;
    }
    if weekend["SubSessionID"].as_i64().is_some() {
        s.subsession_id = new_subsession_id;
    }

    let category = weekend["Category"].as_str().unwrap_or("").to_lowercase();
    let track_type = weekend["TrackType"].as_str().unwrap_or("").to_lowercase();
    let track_config = weekend["TrackConfigName"].as_str().unwrap_or("").to_lowercase();
    let track_disp = weekend["TrackDisplayName"].as_str().unwrap_or("").to_lowercase();

    let is_explicit_road = track_type.contains("road")
        || track_config.contains("road")
        || track_config.contains("roval")
        || track_disp.contains("roval")
        || track_disp.contains("road course");
    let is_explicit_oval = category.contains("oval")
        || track_type.contains("oval")
        || track_type.contains("speedway")
        || track_config.contains("oval")
        || track_disp.contains("oval");
    if is_explicit_road || is_explicit_oval {
        s.is_oval = is_explicit_oval && !is_explicit_road;
    }

    // 2. Subsession Definitions & Rubber States
    if let Some(sessions) = root["SessionInfo"]["Sessions"].as_sequence() {
        if !new_subsession_types.is_empty() {
            s.subsession_types = new_subsession_types;
            s.subsession_rubber_states = new_subsession_rubber;
        }

        if s.session_num >= 0 {
            if let Some(stype) = s.subsession_types.get(&s.session_num) {
                s.session_type = format_session_type(stype);
            }
            if let Some(rstate) = s.subsession_rubber_states.get(&s.session_num) {
                s.track_state = format_track_state(rstate, s.track_wetness);
            }
        } else if let Some(first_sess) = sessions.first() {
            if let Some(stype) = first_sess["SessionType"].as_str() {
                s.session_type = format_session_type(stype);
            }
            if let Some(rstate) = first_sess["SessionTrackRubberState"].as_str() {
                s.track_state = format_track_state(rstate, s.track_wetness);
            }
        }
    }

    // 3. Driver Info
    let driver_info = &root["DriverInfo"];
    if let Some(car_idx) = driver_info["DriverCarIdx"].as_u64() {
        s.player_car_idx = car_idx as usize;
    }
    if let Some(fuel) = driver_info["DriverCarFuelMaxLtr"].as_f64() {
        s.player_fuel_capacity = fuel as f32;
    }

    let raw_setup = driver_info["DriverSetupName"].as_str().unwrap_or("");
    let clean_setup = clean_setup_name(raw_setup);
    if !clean_setup.is_empty() {
        s.player_setup_name = clean_setup;
    }
    let effective_setup = if !s.player_setup_name.is_empty() {
        s.player_setup_name.clone()
    } else {
        "Baseline".to_string()
    };

    let player_idx = s.player_car_idx;

    if let Some(drivers) = driver_info["Drivers"].as_sequence() {
        let mut active_yaml_indices = std::collections::HashSet::new();
        for d in drivers {
            let idx = d["CarIdx"].as_u64().unwrap_or(999) as usize;
            if idx >= 64 {
                continue;
            }
            active_yaml_indices.insert(idx);
            let user_name = d["UserName"].as_str().unwrap_or("").to_string();
            let car_num = match &d["CarNumber"] {
                serde_yaml::Value::String(s) => s.clone(),
                serde_yaml::Value::Number(n) => n.to_string(),
                _ => idx.to_string(),
            };
            let irating_str = match &d["IRating"] {
                serde_yaml::Value::Number(n) => n.to_string(),
                serde_yaml::Value::String(s) => s.clone(),
                _ => "—".to_string(),
            };
            let car_screen_name = d["CarScreenName"].as_str().unwrap_or("").to_string();
            let car_screen_short = d["CarScreenNameShort"].as_str().unwrap_or("").to_string();
            let car_path = d["CarPath"].as_str().unwrap_or("").to_string();

            // Set player's car name
            if idx == player_idx {
                if !car_screen_name.is_empty() {
                    s.car_name = car_screen_name.clone();
                } else if !car_screen_short.is_empty() {
                    s.car_name = car_screen_short;
                } else if !car_path.is_empty() {
                    s.car_name = car_path;
                } else {
                    s.car_name = format!("Car #{}", car_num);
                }
            }

            let setup_to_assign = if idx == player_idx {
                effective_setup.clone()
            } else {
                "Baseline".to_string()
            };

            s.drivers.entry(idx).and_modify(|target| {
                // If a different real driver took over this CarIdx slot, reset their stints/laps
                let had_real_name = !target.driver_name.is_empty()
                    && target.driver_name != "Player Car"
                    && !target.driver_name.starts_with("Car #");
                if had_real_name && !user_name.is_empty() && target.driver_name != user_name {
                    target.stints.clear();
                    target.total_laps_completed = 0;
                    target.last_processed_lap = 0;
                    target.last_known_lap_time = 0.0;
                    target.last_sf_crossing_time = 0.0;
                    target.exited_pits_on_current_lap = false;
                    target.ignored_lap_sdk = -1;
                    target.is_initialized = false;
                }
                if !user_name.is_empty() {
                    target.driver_name = user_name.clone();
                }
                if !car_num.is_empty() {
                    target.car_num = car_num.clone();
                }
                if !irating_str.is_empty() && irating_str != "—" {
                    target.irating = irating_str.clone();
                }
                if idx == player_idx && setup_to_assign != "Baseline" {
                    target.setup_name = setup_to_assign.clone();
                }
            }).or_insert(DriverState {
                car_idx: idx,
                driver_name: if !user_name.is_empty() { user_name } else { format!("Car #{}", idx) },
                car_num: if !car_num.is_empty() { car_num } else { idx.to_string() },
                irating: irating_str,
                position: 0,
                is_on_pit_road: true,
                is_in_world: false,
                is_initialized: false,
                total_laps_completed: 0,
                last_processed_lap: 0,
                last_known_lap_time: 0.0,
                last_sf_crossing_time: 0.0,
                exited_pits_on_current_lap: false,
                ignored_lap_sdk: -1,
                setup_name: setup_to_assign,
                stints: Vec::new(),
                historical_fuel_burn_rate: 0.0,
            });
        }

        if !active_yaml_indices.is_empty() {
            s.drivers.retain(|idx, _| active_yaml_indices.contains(idx));
        }
    }

    // Backfill setup_name into player driver and any stints that were started before setup was known
    if effective_setup != "Baseline" {
        if let Some(player_drv) = s.drivers.get_mut(&player_idx) {
            player_drv.setup_name = effective_setup.clone();
            for stint in &mut player_drv.stints {
                if stint.setup_name.is_empty() || stint.setup_name == "—" || stint.setup_name == "Baseline" {
                    stint.setup_name = effective_setup.clone();
                }
            }
        }
    }

    // 3. Extract definitive garage tire measurements from CarSetup (exact match to iRacing garage screen)
    if let Some(car_setup) = root.get("CarSetup") {
        if let Some(garage_tires) = parse_car_setup_tires(car_setup) {
            if garage_tires.is_valid() {
                s.latest_tire_data = garage_tires.clone();
                if let Some(player_drv) = s.drivers.get_mut(&player_idx) {
                    if let Some(last_stint) = player_drv.stints.last_mut() {
                        if !last_stint.is_active && last_stint.laps_completed > 0 {
                            last_stint.tires = garage_tires;
                        }
                    }
                }
            }
        }
    }

    true
}

pub fn parse_pressure_str_to_kpa(raw: &str) -> f32 {
    let clean = raw.trim();
    if clean.is_empty() || clean == "—" {
        return 0.0;
    }
    let mut num_end = 0;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            num_end = i + c.len_utf8();
        } else if num_end > 0 && !c.is_whitespace() {
            break;
        }
    }
    if num_end == 0 {
        return 0.0;
    }
    let num = clean[..num_end].trim().parse::<f32>().unwrap_or(0.0);
    let unit = clean[num_end..].trim().to_lowercase();
    if unit.contains("psi") {
        num / 0.14503774
    } else if unit.contains("bar") {
        num * 100.0
    } else {
        num
    }
}

pub fn parse_three_floats(raw: &str) -> Option<[f32; 3]> {
    let mut vals = Vec::new();
    for token in raw.split(|c: char| c == ',' || c == ';' || c == '/' || c == '·' || c.is_whitespace()) {
        let clean = token.trim()
            .trim_end_matches('%')
            .trim_end_matches('F')
            .trim_end_matches('f')
            .trim_end_matches('C')
            .trim_end_matches('c')
            .trim_end_matches('°');
        if !clean.is_empty() {
            if let Ok(v) = clean.parse::<f32>() {
                vals.push(v);
            }
        }
    }
    if vals.len() == 3 {
        Some([vals[0], vals[1], vals[2]])
    } else {
        None
    }
}

fn find_setup_corner<'a>(root: &'a serde_yaml::Value, corner_keys: &[&str]) -> Option<&'a serde_yaml::Value> {
    for parent in &["Tires", "TiresAero", "Chassis", "Suspension"] {
        if let Some(p) = root.get(*parent) {
            for key in corner_keys {
                if let Some(c) = p.get(*key) {
                    return Some(c);
                }
            }
        }
    }
    for key in corner_keys {
        if let Some(c) = root.get(*key) {
            return Some(c);
        }
    }
    if let Some(map) = root.as_mapping() {
        for (_k, v) in map {
            for key in corner_keys {
                if let Some(c) = v.get(*key) {
                    return Some(c);
                }
            }
        }
    }
    None
}

fn parse_corner_yaml(corner_val: &serde_yaml::Value, is_left: bool) -> Option<TireCornerData> {
    if !corner_val.is_mapping() {
        return None;
    }

    let p_str = corner_val.get("LastHotPressure").and_then(|v| v.as_str())
        .or_else(|| corner_val.get("Last Hot Pressure").and_then(|v| v.as_str()))
        .or_else(|| corner_val.get("LastPressure").and_then(|v| v.as_str()))
        .or_else(|| corner_val.get("HotPressure").and_then(|v| v.as_str()))
        .or_else(|| corner_val.get("Hot Pressure").and_then(|v| v.as_str()));
    let pressure = p_str.map(parse_pressure_str_to_kpa).unwrap_or(0.0);

    let temps_str = if is_left {
        corner_val.get("LastTempsOMI").and_then(|v| v.as_str())
            .or_else(|| corner_val.get("Last Temps OMI").and_then(|v| v.as_str()))
            .or_else(|| corner_val.get("TempsOMI").and_then(|v| v.as_str()))
            .or_else(|| corner_val.get("LastTemps").and_then(|v| v.as_str()))
            .or_else(|| corner_val.get("Temps").and_then(|v| v.as_str()))
    } else {
        corner_val.get("LastTempsIMO").and_then(|v| v.as_str())
            .or_else(|| corner_val.get("Last Temps IMO").and_then(|v| v.as_str()))
            .or_else(|| corner_val.get("TempsIMO").and_then(|v| v.as_str()))
            .or_else(|| corner_val.get("LastTemps").and_then(|v| v.as_str()))
            .or_else(|| corner_val.get("Temps").and_then(|v| v.as_str()))
    };

    let is_fahrenheit = temps_str.map_or(false, |s| s.contains('F') || s.contains('f'));
    let parsed_temps = temps_str.and_then(parse_three_floats);

    let (temp_l, temp_m, temp_r) = if let Some(mut t) = parsed_temps {
        if is_fahrenheit || t[0] > 120.0 || t[1] > 120.0 || t[2] > 120.0 {
            t[0] = (t[0] - 32.0) / 1.8;
            t[1] = (t[1] - 32.0) / 1.8;
            t[2] = (t[2] - 32.0) / 1.8;
        }
        (t[0], t[1], t[2])
    } else {
        (0.0, 0.0, 0.0)
    };

    let tread_str = corner_val.get("TreadRemaining").and_then(|v| v.as_str())
        .or_else(|| corner_val.get("Tread Remaining").and_then(|v| v.as_str()))
        .or_else(|| corner_val.get("Wear").and_then(|v| v.as_str()));
    let parsed_wear = tread_str.and_then(parse_three_floats);

    let (wear_l, wear_m, wear_r) = if let Some(w) = parsed_wear {
        (w[0], w[1], w[2])
    } else {
        (0.0, 0.0, 0.0)
    };

    Some(TireCornerData {
        temp_l,
        temp_m,
        temp_r,
        wear_l,
        wear_m,
        wear_r,
        pressure,
    })
}

pub fn parse_car_setup_tires(car_setup: &serde_yaml::Value) -> Option<StintTireSet> {
    let lf_node = find_setup_corner(car_setup, &["LeftFront", "Left Front", "LF"])?;
    let rf_node = find_setup_corner(car_setup, &["RightFront", "Right Front", "RF"])?;
    let lr_node = find_setup_corner(car_setup, &["LeftRear", "Left Rear", "LR"])?;
    let rr_node = find_setup_corner(car_setup, &["RightRear", "Right Rear", "RR"])?;

    let lf = parse_corner_yaml(lf_node, true)?;
    let rf = parse_corner_yaml(rf_node, false)?;
    let lr = parse_corner_yaml(lr_node, true)?;
    let rr = parse_corner_yaml(rr_node, false)?;

    let has_data = lf.temp_m > 5.0 || rf.temp_m > 5.0 || lf.wear_m > 0.1 || rf.wear_m > 0.1 || lf.pressure > 1.0;
    Some(StintTireSet {
        lf,
        rf,
        lr,
        rr,
        has_data,
    })
}

fn convert_unit(val: &str, to_metric: bool) -> String {
    let clean = val.trim();
    if clean.is_empty() || clean == "—" || clean == "..." { return clean.to_string(); }

    // Split numeric prefix and unit suffix (handles "1.65 km", "1.65km", "75 F", "75°F", "24.5 C", "24.5", etc.)
    let mut split_idx = 0;
    let mut has_digit = false;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            has_digit = true;
            split_idx = i + c.len_utf8();
        } else if has_digit && !c.is_whitespace() {
            break;
        }
    }

    if !has_digit {
        return clean.to_string();
    }

    let num_str = clean[..split_idx].trim();
    let unit_str = clean[split_idx..].trim().to_uppercase();

    let num = match num_str.parse::<f32>() {
        Ok(n) => n,
        Err(_) => return clean.to_string(),
    };

    match unit_str.as_str() {
        "C" | "°C" | "DEG C" | "DEGC" | "DEG" | "°" | "" => {
            if to_metric { format!("{:.0}°C", num) }
            else { format!("{:.0}°F", num * 1.8 + 32.0) }
        },
        "F" | "°F" | "DEG F" | "DEGF" => {
            if to_metric { format!("{:.0}°C", (num - 32.0) / 1.8) }
            else { format!("{:.0}°F", num) }
        },
        "M/S" | "M\\S" | "MPS" => {
            if to_metric { format!("{:.0} km/h", num * 3.6) }
            else { format!("{:.0} mph", num * 2.23694) }
        },
        "KPH" | "KM/H" | "KMH" => {
            if to_metric { format!("{:.0} km/h", num) }
            else { format!("{:.0} mph", num * 0.621371) }
        },
        "MPH" => {
            if to_metric { format!("{:.0} km/h", num * 1.60934) }
            else { format!("{:.0} mph", num) }
        },
        "KM" | "KMS" | "KILOMETERS" | "KILOMETRE" | "KILOMETER" => {
            if to_metric { format!("{:.2} km", num) }
            else { format!("{:.2} mi", num * 0.621371) }
        },
        "MI" | "MI." | "MILE" | "MILES" => {
            if to_metric { format!("{:.2} km", num * 1.60934) }
            else { format!("{:.2} mi", num) }
        },
        "M" | "METERS" | "METRES" => {
            if to_metric { format!("{:.0} m", num) }
            else { format!("{:.0} ft", num * 3.28084) }
        },
        "%" | "PCT" | "PERCENT" => {
            let pct_val = if num <= 1.0 && num > 0.0 { num * 100.0 } else { num };
            format!("{:.0}%", pct_val)
        },
        _ => clean.to_string()
    }
}

fn format_wind_dir(raw: &str) -> String {
    let clean = raw.trim();
    if clean.is_empty() || clean == "..." || clean == "—" {
        return String::new();
    }
    let upper = clean.to_uppercase();
    if matches!(
        upper.as_str(),
        "N" | "NE" | "E" | "SE" | "S" | "SW" | "W" | "NW" | "NNE" | "ENE" | "ESE" | "SSE" | "SSW" | "WSW" | "WNW" | "NNW"
    ) {
        return upper;
    }
    let mut num_end = 0;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }
    if num_end > 0 {
        if let Ok(val) = clean[..num_end].parse::<f32>() {
            let unit = clean[num_end..].trim().to_lowercase();
            let deg = if unit.contains("rad") || unit.is_empty() {
                val.to_degrees().rem_euclid(360.0)
            } else {
                val.rem_euclid(360.0)
            };
            let dirs = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
            let idx = (((deg + 22.5) / 45.0).floor() as usize) % 8;
            return dirs[idx].to_string();
        }
    }
    clean.to_string()
}

fn format_time(t: f32) -> String {
    if t <= 0.0 { 
        "-".to_string() 
    } else { 
        let mins = (t / 60.0).floor() as i32;
        let secs = t % 60.0;
        if mins > 0 {
            format!("{}:{:06.3}", mins, secs)
        } else {
            format!("{:.3}", secs)
        }
    }
}

fn get_sample_f32(live: &iracing::telemetry::Sample, key: &'static str) -> f32 {
    match live.get(key) {
        Ok(Value::FLOAT(v)) => v,
        Ok(Value::DOUBLE(v)) => v as f32,
        _ => 0.0,
    }
}

fn read_player_tires(live: &iracing::telemetry::Sample, previous_hot: Option<&StintTireSet>) -> StintTireSet {
    let to_wear = |w: f32| -> f32 {
        if w > 0.0 && w <= 1.0 { w * 100.0 } else { w.max(0.0) }
    };

    let resolve_pressure = |p_live: f32, prev_p: f32| -> f32 {
        if p_live > 1.0 {
            p_live
        } else if prev_p > 1.0 {
            prev_p
        } else {
            0.0
        }
    };

    let prev_lf = previous_hot.map(|p| p.lf.pressure).unwrap_or(0.0);
    let prev_rf = previous_hot.map(|p| p.rf.pressure).unwrap_or(0.0);
    let prev_lr = previous_hot.map(|p| p.lr.pressure).unwrap_or(0.0);
    let prev_rr = previous_hot.map(|p| p.rr.pressure).unwrap_or(0.0);

    let lf = TireCornerData {
        temp_l: get_sample_f32(live, "LFtempCL"),
        temp_m: get_sample_f32(live, "LFtempCM"),
        temp_r: get_sample_f32(live, "LFtempCR"),
        wear_l: to_wear(get_sample_f32(live, "LFwearL")),
        wear_m: to_wear(get_sample_f32(live, "LFwearM")),
        wear_r: to_wear(get_sample_f32(live, "LFwearR")),
        pressure: resolve_pressure(get_sample_f32(live, "LFpressure"), prev_lf),
    };

    let rf = TireCornerData {
        temp_l: get_sample_f32(live, "RFtempCL"),
        temp_m: get_sample_f32(live, "RFtempCM"),
        temp_r: get_sample_f32(live, "RFtempCR"),
        wear_l: to_wear(get_sample_f32(live, "RFwearL")),
        wear_m: to_wear(get_sample_f32(live, "RFwearM")),
        wear_r: to_wear(get_sample_f32(live, "RFwearR")),
        pressure: resolve_pressure(get_sample_f32(live, "RFpressure"), prev_rf),
    };

    let lr = TireCornerData {
        temp_l: get_sample_f32(live, "LRtempCL"),
        temp_m: get_sample_f32(live, "LRtempCM"),
        temp_r: get_sample_f32(live, "LRtempCR"),
        wear_l: to_wear(get_sample_f32(live, "LRwearL")),
        wear_m: to_wear(get_sample_f32(live, "LRwearM")),
        wear_r: to_wear(get_sample_f32(live, "LRwearR")),
        pressure: resolve_pressure(get_sample_f32(live, "LRpressure"), prev_lr),
    };

    let rr = TireCornerData {
        temp_l: get_sample_f32(live, "RRtempCL"),
        temp_m: get_sample_f32(live, "RRtempCM"),
        temp_r: get_sample_f32(live, "RRtempCR"),
        wear_l: to_wear(get_sample_f32(live, "RRwearL")),
        wear_m: to_wear(get_sample_f32(live, "RRwearM")),
        wear_r: to_wear(get_sample_f32(live, "RRwearR")),
        pressure: resolve_pressure(get_sample_f32(live, "RRpressure"), prev_rr),
    };

    let has_data = lf.temp_m > 5.0 || rf.temp_m > 5.0 || lf.wear_m > 0.1 || rf.wear_m > 0.1 || lf.pressure > 1.0;
    StintTireSet {
        lf,
        rf,
        lr,
        rr,
        has_data,
    }
}

fn format_laps_column_tsv(stint: &StintData) -> String {
    let mut out = String::new();
    for &t in stint.valid_laps().iter() {
        out.push_str(&format!("{:.3}\n", t));
    }
    out
}

fn format_stint_laps_spreadsheet_tsv(stint: &StintData) -> String {
    let valid_laps = stint.valid_laps();
    let mut out = String::from("Stint Lap\tLap\tLap Time\tAvg Lap Time\n");
    let mut sum = 0.0;
    for (i, &t) in valid_laps.iter().enumerate() {
        sum += t;
        let avg = sum / (i + 1) as f32;
        let overall_lap = stint.start_lap + (i as i32) + 1;
        out.push_str(&format!("{}\t{}\t{:.3}\t{:.3}\n", i + 1, overall_lap, t, avg));
    }
    out
}

fn format_stint_laps_tsv(stint: &StintData) -> String {
    let mut out = String::new();
    out.push_str("Lap\tTime (s)\tTime (Formatted)\n");
    for (i, &t) in stint.valid_laps().iter().enumerate() {
        out.push_str(&format!("{}\t{:.3}\t{}\n", i + 1, t, format_time(t)));
    }
    out
}

fn format_single_tire_tsv(corner: &str, tire: &TireCornerData, use_metric: bool) -> String {
    let press_str = if tire.pressure > 0.1 {
        if use_metric { format!("{:.2} bar", tire.pressure / 100.0) } else { format!("{:.1} PSI", tire.pressure * 0.145038) }
    } else {
        "—".to_string()
    };
    let temp_str = if tire.temp_m > 1.0 {
        if use_metric {
            format!("{:.0}°·{:.0}°·{:.0}°C", tire.temp_l, tire.temp_m, tire.temp_r)
        } else {
            format!("{:.0}°·{:.0}°·{:.0}°F", tire.temp_l * 1.8 + 32.0, tire.temp_m * 1.8 + 32.0, tire.temp_r * 1.8 + 32.0)
        }
    } else {
        "—".to_string()
    };
    let wear_str = if tire.wear_m > 0.01 {
        format!("{:.0}%·{:.0}%·{:.0}%", tire.wear_l, tire.wear_m, tire.wear_r)
    } else {
        "—".to_string()
    };

    format!("{}\t{}\t{}\t{}", corner, press_str, temp_str, wear_str)
}

fn format_single_tire_temps_tsv(tire: &TireCornerData, use_metric: bool) -> String {
    if tire.temp_m > 1.0 {
        if use_metric {
            format!("{:.0}\t{:.0}\t{:.0}", tire.temp_l, tire.temp_m, tire.temp_r)
        } else {
            format!("{:.0}\t{:.0}\t{:.0}", tire.temp_l * 1.8 + 32.0, tire.temp_m * 1.8 + 32.0, tire.temp_r * 1.8 + 32.0)
        }
    } else {
        "—\t—\t—".to_string()
    }
}

fn format_single_tire_wear_tsv(tire: &TireCornerData) -> String {
    if tire.wear_m > 0.01 {
        format!("{:.0}%\t{:.0}%\t{:.0}%", tire.wear_l, tire.wear_m, tire.wear_r)
    } else {
        "—\t—\t—".to_string()
    }
}

fn format_temps_excel_tsv(tires: &StintTireSet, use_metric: bool) -> String {
    let unit = if use_metric { "°C" } else { "°F" };
    let mut out = String::new();
    out.push_str(&format!("Corner (OMI / IMO)\tZone 1 ({})\tZone 2 ({})\tZone 3 ({})\n", unit, unit, unit));
    out.push_str(&format!("Left Front (LF · OMI)\t{}\n", format_single_tire_temps_tsv(&tires.lf, use_metric)));
    out.push_str(&format!("Right Front (RF · IMO)\t{}\n", format_single_tire_temps_tsv(&tires.rf, use_metric)));
    out.push_str(&format!("Left Rear (LR · OMI)\t{}\n", format_single_tire_temps_tsv(&tires.lr, use_metric)));
    out.push_str(&format!("Right Rear (RR · IMO)\t{}\n", format_single_tire_temps_tsv(&tires.rr, use_metric)));
    out
}

fn format_wear_excel_tsv(tires: &StintTireSet) -> String {
    let mut out = String::new();
    out.push_str("Corner (OMI / IMO)\tZone 1 Wear\tZone 2 Wear\tZone 3 Wear\n");
    out.push_str(&format!("Left Front (LF · OMI)\t{}\n", format_single_tire_wear_tsv(&tires.lf)));
    out.push_str(&format!("Right Front (RF · IMO)\t{}\n", format_single_tire_wear_tsv(&tires.rf)));
    out.push_str(&format!("Left Rear (LR · OMI)\t{}\n", format_single_tire_wear_tsv(&tires.lr)));
    out.push_str(&format!("Right Rear (RR · IMO)\t{}\n", format_single_tire_wear_tsv(&tires.rr)));
    out
}

fn format_tires_excel_tsv(tires: &StintTireSet, use_metric: bool) -> String {
    let mut out = String::new();
    out.push_str("Corner\tHot Pressure\tTemps (OMI / IMO)\tWear (OMI / IMO)\n");
    out.push_str(&format_single_tire_tsv("Left Front (LF)", &tires.lf, use_metric));
    out.push('\n');
    out.push_str(&format_single_tire_tsv("Right Front (RF)", &tires.rf, use_metric));
    out.push('\n');
    out.push_str(&format_single_tire_tsv("Left Rear (LR)", &tires.lr, use_metric));
    out.push('\n');
    out.push_str(&format_single_tire_tsv("Right Rear (RR)", &tires.rr, use_metric));
    out.push('\n');
    out
}

fn format_full_stint_excel_tsv(stint: &StintData, active_buckets: &[usize], use_metric: bool) -> String {
    let mut out = String::new();
    let fuel_conv = if use_metric { 1.0 } else { 0.264172 };
    let fuel_unit = if use_metric { "L" } else { "Gal" };

    out.push_str(&format!("=== STINT S{} SUMMARY ===\n", stint.stint_number));
    out.push_str("Stint #\tLaps Completed\tFastest Lap\tAverage Lap\tAvg Fuel/Lap\tConsistency (σ)\tSetup\tTrack Temp\tAir Temp\n");
    out.push_str(&format!(
        "S{}\t{}\t{}\t{}\t{:.3} {}\t{:.3}\t{}\t{}\t{}\n\n",
        stint.stint_number,
        stint.laps_completed,
        format_time(stint.fast_lap),
        format_time(stint.full_average()),
        stint.fuel_per_lap() * fuel_conv,
        fuel_unit,
        stint.std_dev(),
        stint.setup_name,
        stint.track_temp_at_start,
        stint.air_temp_at_start
    ));

    if !active_buckets.is_empty() {
        out.push_str("=== ROLLING AVERAGES ===\n");
        let headers: Vec<String> = active_buckets.iter().map(|b| format!("AVG {}", b)).collect();
        out.push_str(&headers.join("\t"));
        out.push('\n');
        let vals: Vec<String> = active_buckets.iter().map(|b| {
            let avg = stint.avg_first_n(*b);
            if avg > 0.0 { format_time(avg) } else { "—".to_string() }
        }).collect();
        out.push_str(&vals.join("\t"));
        out.push_str("\n\n");
    }

    if stint.tires.has_data {
        out.push_str("=== 4-CORNER POST-STINT TIRES ===\n");
        out.push_str(&format_tires_excel_tsv(&stint.tires, use_metric));
        out.push('\n');
    }

    out.push_str("=== LAP-BY-LAP TELEMETRY ===\n");
    out.push_str(&format_stint_laps_tsv(stint));

    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridSortKey {
    Position,
    CarNumber,
    DriverName,
    StintNumber,
    LapsCompleted,
    LastLap,
    FastestLap,
    CurrentAverage,
    AvgBucket(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

struct WarRoomApp {
    state: Arc<Mutex<WarRoomState>>,
    settings: AppSettings,
    logo_texture: Option<egui::TextureHandle>,
    show_settings_modal: bool,
    show_telemetry_debugger: bool,
    selected_driver_car_idx: Option<usize>,
    selected_stint_num: Option<i32>,
    show_stint_analyzer_modal: bool,
    custom_bucket_input: String,
    toast_message: Option<(String, std::time::Instant)>,
    grid_sort_key: GridSortKey,
    grid_sort_dir: SortDirection,
    driver_search_query: String,
    updater: updater::UpdateChecker,
    show_release_notes_modal: bool,
    release_notes_custom_content: Option<String>,
    preview_overlay_hud: bool,
}

fn load_logo_texture(ctx: &egui::Context, is_dark: bool) -> Option<egui::TextureHandle> {
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
                                || !s.drivers.is_empty()
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
                                    s.track_temp_raw = format!("{:.0} C", tt);
                                }
                            } else if let Ok(Value::FLOAT(tt_crew)) = live.get("TrackTempCrew") {
                                if tt_crew > -40.0 && tt_crew < 130.0 {
                                    s.track_temp_raw = format!("{:.0} C", tt_crew);
                                }
                            }

                            if let Ok(Value::FLOAT(at)) = live.get("AirTemp") {
                                if at > -40.0 && at < 130.0 {
                                    s.air_temp_raw = format!("{:.0} C", at);
                                }
                            }

                            if let Ok(Value::FLOAT(wv)) = live.get("WindVel") {
                                if wv >= 0.0 && wv < 100.0 {
                                    s.wind_speed_raw = format!("{:.0} m/s", wv);
                                }
                            }
                            if let Ok(Value::FLOAT(wd)) = live.get("WindDir") {
                                if wd.is_finite() {
                                    s.wind_dir_raw = format!("{:.3} rad", wd);
                                }
                            }
                            if let Ok(Value::FLOAT(rh)) = live.get("RelativeHumidity") {
                                if rh >= 0.0 && rh <= 100.0 {
                                    let rh_pct = if rh <= 1.0 && rh > 0.0 { rh * 100.0 } else { rh };
                                    s.relative_humidity_raw = format!("{:.0}%", rh_pct);
                                }
                            }

                            // Read Session Laps Remaining
                            if let Ok(Value::INT(remain)) = live.get("SessionLapsRemain") {
                                if remain > 0 && remain < 10000 {
                                    s.session_laps_remain = remain;
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
        if settings.updates.check_on_startup {
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
            custom_bucket_input: String::new(),
            toast_message: None,
            grid_sort_key: GridSortKey::Position,
            grid_sort_dir: SortDirection::Ascending,
            driver_search_query: String::new(),
            updater,
            show_release_notes_modal: false,
            release_notes_custom_content: None,
            preview_overlay_hud: false,
        }
    }

    fn render_telemetry_debugger(&mut self, ctx: &egui::Context, s: &WarRoomState) {
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
                        if ui.add(egui::Button::new(egui::RichText::new("✕ Close").color(white).size(12.0)).fill(elevated)).clicked() {
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

    fn render_release_notes_modal(&mut self, ctx: &egui::Context) {
        if !self.show_release_notes_modal {
            return;
        }

        let primary = self.settings.primary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();
        let panel_bg = self.settings.panel_color();
        let border = self.settings.divider_color();

        let embedded_notes = include_str!("../RELEASE_NOTES.md");
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
                                    let _ = updater::apply_update_and_restart();
                                }
                            }
                            _ => {}
                        }
                    });
                });
            });
    }

    fn render_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.show_settings_modal { return; }

        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let muted = self.settings.text_muted();
        let elevated = self.settings.card_elevated_color();

        egui::Window::new("Settings & Layouts")
            .fixed_size([570.0, 640.0])
            .vscroll(true)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                
                ui.label(egui::RichText::new("VIEW MODE").color(muted).size(11.0).extra_letter_spacing(0.8));
                ui.horizontal(|ui| {
                    if ui.selectable_label(self.settings.view_mode == ActiveViewMode::CockpitEngineer, "Cockpit Stint & Fuel Analyzer").clicked() {
                        self.settings.view_mode = ActiveViewMode::CockpitEngineer;
                        self.settings.save();
                    }
                    if ui.selectable_label(self.settings.view_mode == ActiveViewMode::GridTable, "Multi-Driver Stint Grid").clicked() {
                        self.settings.view_mode = ActiveViewMode::GridTable;
                        self.settings.save();
                    }
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(egui::RichText::new("LAYOUT PRESETS").color(muted).size(11.0).extra_letter_spacing(0.8));
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

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(egui::RichText::new("ROLLING LAP AVERAGE BUCKETS").color(muted).size(11.0).extra_letter_spacing(0.8));
                ui.label(egui::RichText::new("Select which lap averages to display in stint tables:").color(muted).size(12.0));
                ui.add_space(4.0);

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
                                .size(12.0)
                        )
                        .fill(if is_active { with_alpha(secondary, 180) } else { elevated })
                        .rounding(egui::Rounding::same(6.0));

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

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Custom bucket:").size(12.0).color(muted));
                    let edit = ui.add(egui::TextEdit::singleline(&mut self.custom_bucket_input).desired_width(55.0).hint_text("e.g. 12"));
                    let enter_pressed = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.add(egui::Button::new(egui::RichText::new("+ Add").color(white).size(12.0)).fill(elevated)).clicked() || enter_pressed {
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
                            egui::Button::new(egui::RichText::new("Reset defaults").color(muted).size(11.5))
                                .fill(elevated)
                        ).clicked() {
                            self.settings.custom_avg_buckets = vec![5, 10, 15, 20, 25, 30, 35, 40, 50, 60];
                            self.settings.active_avg_buckets = vec![5, 10, 15, 20, 25, 30];
                            self.settings.save();
                        }
                    });
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(egui::RichText::new("GRID COLUMNS").color(muted).size(11.0).extra_letter_spacing(0.8));
                egui::Grid::new("column_grid").num_columns(3).spacing([20.0, 6.0]).show(ui, |ui| {
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

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(egui::RichText::new("IN-CAR OVERLAY & HUD MODULES").color(muted).size(11.0).extra_letter_spacing(0.8));
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.settings.overlay.enabled, "Auto-show overlay in car").changed() {
                        self.settings.save();
                    }
                    if ui.checkbox(&mut self.settings.overlay.always_on_top, "Always on top").changed() {
                        self.settings.save();
                    }
                    let preview_btn_text = if self.preview_overlay_hud { "Hide HUD Preview" } else { "Show HUD Preview" };
                    if ui.selectable_label(self.preview_overlay_hud, preview_btn_text).clicked() {
                        self.preview_overlay_hud = !self.preview_overlay_hud;
                    }
                });

                ui.add_space(3.0);
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
                });

                ui.add_space(3.0);
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

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("HUD Scale").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.overlay.hud_scale, 0.75..=1.50).custom_formatter(|n, _| format!("{:.0}%", n * 100.0))).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("HUD Opacity").color(muted));
                    if ui.add(egui::Slider::new(&mut self.settings.overlay.opacity, 0.20..=1.0)).changed() {
                        self.settings.save();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Pit safety margin").color(muted));
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
                        if ui.add(egui::Button::new(egui::RichText::new("Reset").color(muted).size(11.5)).fill(elevated)).clicked() {
                            self.settings.overlay.position = None;
                            self.settings.save();
                        }
                    }
                });

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(egui::RichText::new("THEME & UNITS").color(muted).size(11.0).extra_letter_spacing(0.8));
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Theme:").color(muted));
                    if ui.selectable_label(self.settings.theme == ThemePreset::S4SGoldBlue, "S4S Gold & Blue (Dark)").clicked() {
                        self.settings.theme = ThemePreset::S4SGoldBlue;
                        self.settings.save();
                        setup_custom_style(ctx, &self.settings);
                        self.logo_texture = load_logo_texture(ctx, self.settings.is_dark_theme());
                    }
                    if ui.selectable_label(self.settings.theme == ThemePreset::S4SGoldBlueLight, "S4S Daylight (Light Mode)").clicked() {
                        self.settings.theme = ThemePreset::S4SGoldBlueLight;
                        self.settings.save();
                        setup_custom_style(ctx, &self.settings);
                        self.logo_texture = load_logo_texture(ctx, self.settings.is_dark_theme());
                    }
                    if ui.selectable_label(self.settings.theme == ThemePreset::StealthDark, "Stealth Monochrome").clicked() {
                        self.settings.theme = ThemePreset::StealthDark;
                        self.settings.save();
                        setup_custom_style(ctx, &self.settings);
                        self.logo_texture = load_logo_texture(ctx, self.settings.is_dark_theme());
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Units:").color(muted));
                    if ui.selectable_label(!self.settings.use_metric, "Imperial (mph / °F / gal)").clicked() {
                        self.settings.use_metric = false;
                        self.settings.save();
                    }
                    if ui.selectable_label(self.settings.use_metric, "Metric (km/h / °C / L)").clicked() {
                        self.settings.use_metric = true;
                        self.settings.save();
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(egui::RichText::new("UPDATES & RELEASE CHANNEL").color(muted).size(11.0).extra_letter_spacing(0.8));
                ui.horizontal(|ui| {
                    let version_label = format!("Installed Version: v{}", updater::CURRENT_VERSION);
                    ui.label(egui::RichText::new(version_label).color(white).size(12.5));
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

                ui.add_space(3.0);
                if ui.checkbox(&mut self.settings.updates.check_on_startup, "Check for updates automatically on launch").changed() {
                    self.settings.save();
                }
                if ui.checkbox(&mut self.settings.updates.include_prereleases, "Include Beta / Staging pre-releases").changed() {
                    self.settings.save();
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let check_btn = egui::Button::new(egui::RichText::new("Check for Updates Now").color(white).size(12.0))
                        .fill(elevated);
                    if ui.add(check_btn).clicked() {
                        self.updater.trigger_check(self.settings.updates.include_prereleases);
                    }

                    if ui.add(egui::Button::new(egui::RichText::new("📖 What's New / Notes").color(white).size(12.0)).fill(elevated)).clicked() {
                        self.show_release_notes_modal = true;
                    }

                    let updater_status = self.updater.status.clone();
                    match &updater_status {
                        updater::UpdateStatus::Idle => {},
                        updater::UpdateStatus::Checking => {
                            ui.spinner();
                            ui.label(egui::RichText::new("Checking GitHub Releases...").color(muted).size(12.0));
                        },
                        updater::UpdateStatus::UpToDate => {
                            ui.label(egui::RichText::new("✓ Up to date").color(egui::Color32::from_rgb(75, 225, 110)).size(12.0));
                        },
                        updater::UpdateStatus::UpdateAvailable(info) => {
                            let text = format!("Update {} available!", info.tag_name);
                            ui.label(egui::RichText::new(text).color(primary).strong().size(12.0));
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
                            ui.label(egui::RichText::new(format!("Downloading: {:.0}% ({:.1}/{:.1} MB)", percent, mb_down, mb_tot)).color(primary).size(12.0));
                        },
                        updater::UpdateStatus::ReadyToRestart { .. } => {
                            if ui.add(egui::Button::new(egui::RichText::new("Restart to Apply 🔄").color(egui::Color32::BLACK).strong().size(11.5)).fill(self.settings.status_green())).clicked() {
                                let _ = updater::apply_update_and_restart();
                            }
                        },
                        updater::UpdateStatus::Error(err) => {
                            ui.label(egui::RichText::new(format!("⚠ {}", err)).color(egui::Color32::from_rgb(255, 120, 120)).size(11.0));
                        }
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized(egui::vec2(120.0, 32.0), egui::Button::new(egui::RichText::new("Done").color(egui::Color32::BLACK).strong()).fill(primary)).clicked() {
                        self.settings.save();
                        self.show_settings_modal = false;
                    }
                });
            });
    }
}

fn render_tire_corner_card(
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

fn render_stint_lap_chart(
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

impl WarRoomApp {
    fn render_stint_analyzer_modal(&mut self, ctx: &egui::Context, s: &WarRoomState) {
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

        let target_car_idx = self.selected_driver_car_idx.unwrap_or(s.player_car_idx);
        let target_driver = s.drivers.get(&target_car_idx).or_else(|| s.drivers.get(&s.player_car_idx));
        let (drv_ref, player_stints) = match target_driver {
            Some(d) if !d.stints.is_empty() => (d, &d.stints),
            _ => {
                egui::Window::new("Stint Telemetry Analyzer")
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .collapsible(false)
                    .show(ctx, |ui| {
                        ui.label(egui::RichText::new("No stints recorded yet for this driver. Complete laps on track to view stint telemetry.").color(muted));
                        ui.add_space(8.0);
                        if ui.add(egui::Button::new(egui::RichText::new("Close").color(white)).fill(elevated)).clicked() {
                            self.show_stint_analyzer_modal = false;
                        }
                    });
                return;
            }
        };

        let selected_num = self.selected_stint_num.unwrap_or_else(|| {
            player_stints.last().map(|st| st.stint_number).unwrap_or(1)
        });

        let stint = player_stints.iter().find(|st| st.stint_number == selected_num).unwrap_or_else(|| player_stints.last().unwrap());

        let screen_rect = ctx.screen_rect();
        let win_w = (screen_rect.width() * 0.75).clamp(720.0, 1050.0);
        let win_h = (screen_rect.height() * 0.82).clamp(520.0, 850.0);

        let mut is_open = self.show_stint_analyzer_modal;
        let mut close_modal = false;
        let mut updated_notes: Option<(i32, String)> = None;

        egui::Window::new(format!("Stint S{} · #{} {} · Telemetry & Lap Log", stint.stint_number, drv_ref.car_num, drv_ref.driver_name))
            .id(egui::Id::new("stint_analyzer_modal_window"))
            .open(&mut is_open)
            .default_size([win_w, win_h])
            .default_pos([screen_rect.min.x + 40.0, screen_rect.min.y + 40.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close_modal = true;
                }

                // Top Header Row with Stint tabs + Close button
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("STINT").color(muted).size(11.0).extra_letter_spacing(0.8));
                    for st in player_stints.iter() {
                        let is_sel = st.stint_number == selected_num;
                        let tab_title = if st.is_active {
                            format!("S{} · Live ({})", st.stint_number, st.laps_completed)
                        } else {
                            format!("S{} ({})", st.stint_number, st.laps_completed)
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

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_sized(egui::vec2(72.0, 26.0), egui::Button::new(egui::RichText::new("✕ Close").color(white).size(12.0)).fill(elevated)).clicked() {
                            close_modal = true;
                        }
                    });
                });

                ui.add_space(6.0);

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
                                        for (i, &t) in valid_laps.iter().enumerate() {
                                            sum_lap_times += t;
                                            let cum_avg = sum_lap_times / (i + 1) as f32;
                                            let overall_lap = stint.start_lap + (i as i32) + 1;
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
            });

        if let Some((s_num, new_notes)) = updated_notes {
            if let Ok(mut state_guard) = self.state.lock() {
                if let Some(drv) = state_guard.drivers.get_mut(&target_car_idx) {
                    if let Some(st) = drv.stints.iter_mut().find(|st| st.stint_number == s_num) {
                        st.notes = new_notes;
                    }
                }
            }
        }

        if !is_open || close_modal {
            self.show_stint_analyzer_modal = false;
        }
    }

    fn render_cockpit_view(&mut self, ui: &mut egui::Ui, s: &WarRoomState) {
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

        let card_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10));

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
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("STINT LOG").color(muted).size(11.0).extra_letter_spacing(1.0));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(
                                    egui::Button::new(egui::RichText::new("Buckets ▾").color(white).size(11.5))
                                        .fill(elevated)
                                        .rounding(egui::Rounding::same(6.0))
                                ).clicked() {
                                    self.show_settings_modal = true;
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
                                .column(Column::initial(95.0).at_least(85.0))
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
                                        let row_color = if stint.is_active { white } else { egui::Color32::from_rgb(184, 191, 204) };
                                        let is_selected = self.selected_stint_num == Some(stint.stint_number);

                                        body.row(30.0, |mut row| {
                                            row.col(|ui| {
                                                ui.push_id(("cockpit_stint_col", stint.stint_number), |ui| {
                                                    ui.horizontal(|ui| {
                                                        let resp = ui.selectable_label(is_selected, egui::RichText::new(format!("S{}", stint.stint_number)).color(if is_selected { primary } else { white }).strong().size(13.0));
                                                        if resp.double_clicked() {
                                                            let tsv = format_full_stint_excel_tsv(stint, &active_buckets, is_metric);
                                                            ui.output_mut(|o| o.copied_text = tsv);
                                                            self.toast_message = Some((format!("Copied Stint S{} for Excel!", stint.stint_number), std::time::Instant::now()));
                                                        } else if resp.clicked() {
                                                            self.selected_driver_car_idx = Some(s.player_car_idx);
                                                            self.selected_stint_num = Some(stint.stint_number);
                                                            self.show_stint_analyzer_modal = true;
                                                        }
                                                        if stint.is_active {
                                                            egui::Frame::none()
                                                                .fill(with_alpha(green, 32))
                                                                .rounding(egui::Rounding::same(4.0))
                                                                .inner_margin(egui::Margin::symmetric(5.0, 1.5))
                                                                .show(ui, |ui| {
                                                                    ui.label(egui::RichText::new("Live").color(green).size(10.5).strong());
                                                                });
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
                                                        self.toast_message = Some((format!("Copied S{} Laps for Excel", stint.stint_number), std::time::Instant::now()));
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
                                                            self.toast_message = Some((format!("Copied Stint S{} for Excel", stint.stint_number), std::time::Instant::now()));
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
                                            ui.label(egui::RichText::new(window_str).color(egui::Color32::from_rgb(143, 180, 238)).size(13.0).strong());
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
                            let s_num = selected_stint.map_or(1, |st| st.stint_number);

                            let empty_tires = StintTireSet::default();
                            let tires_to_show = if stint_has_tires {
                                selected_stint.map(|st| &st.tires).unwrap_or(&empty_tires)
                            } else {
                                &empty_tires
                            };

                            ui.horizontal(|ui| {
                                let header_lbl = if stint_has_tires {
                                    format!("TIRES · S{}", s_num)
                                } else {
                                    "TIRES · AWAITING PIT".to_string()
                                };
                                ui.label(egui::RichText::new(header_lbl).color(muted).size(11.0).extra_letter_spacing(1.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if stint_has_tires {
                                        ui.menu_button(egui::RichText::new("Copy ▾").color(muted).size(11.5), |ui| {
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
                                    render_tire_corner_card(ui, "LF", &tires_to_show.lf, is_metric, primary, secondary, white);
                                    ui.add_space(6.0);
                                    render_tire_corner_card(ui, "LR", &tires_to_show.lr, is_metric, primary, secondary, white);
                                });
                                tire_cols[1].vertical(|ui| {
                                    render_tire_corner_card(ui, "RF", &tires_to_show.rf, is_metric, primary, secondary, white);
                                    ui.add_space(6.0);
                                    render_tire_corner_card(ui, "RR", &tires_to_show.rr, is_metric, primary, secondary, white);
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

    fn render_grid_table_view(&mut self, ui: &mut egui::Ui, s: &WarRoomState) {
        let primary = self.settings.primary_accent();
        let secondary = self.settings.secondary_accent();
        let white = self.settings.text_color();
        let panel_bg = self.settings.panel_color();
        let cols = &self.settings.columns;
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
            stint_is_active: bool,
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
                        stint_is_active: false,
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
                        stint_is_active: stint.is_active,
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
        let card_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 10));

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
                                ui.label(egui::RichText::new(format_time(session_fastest_lap)).color(egui::Color32::from_rgb(209, 168, 255)).strong().size(12.5));
                            });
                        ui.label(egui::RichText::new(format!("({})", session_fastest_holder)).color(muted).size(11.5));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(
                            egui::Button::new(egui::RichText::new("Analyze stint →").color(egui::Color32::from_rgb(20, 18, 10)).size(11.5).strong())
                                .fill(primary)
                                .rounding(egui::Rounding::same(6.0))
                        ).clicked() {
                            self.show_stint_analyzer_modal = true;
                        }

                        if ui.add(
                            egui::Button::new(egui::RichText::new("Buckets ▾").color(white).size(11.5))
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

                            let text_col = if is_player || e.stint_is_active {
                                white
                            } else {
                                egui::Color32::from_rgb(184, 191, 204)
                            };

                            body.row(30.0, |mut row| {
                                // POS Badge
                                if cols.pos {
                                    row.col(|ui| {
                                        let pos_str = if e.pos > 0 { format!("P{}", e.pos) } else { "—".to_string() };
                                        let (badge_bg, badge_fg) = match e.pos {
                                            1 => (with_alpha(primary, 42), primary),
                                            2 => (egui::Color32::from_rgba_unmultiplied(220, 225, 235, 34), egui::Color32::from_rgb(220, 225, 235)),
                                            3 => (egui::Color32::from_rgba_unmultiplied(205, 133, 63, 40), egui::Color32::from_rgb(230, 165, 100)),
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
                                            let tag = if e.stint_is_active { " · Live" } else if e.is_on_pit_road { " · Pit" } else { "" };
                                            let stint_btn_text = format!("S{}{}", e.stint_num, tag);

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
                                                        ui.label(egui::RichText::new(fast_str).color(egui::Color32::from_rgb(209, 168, 255)).strong().size(12.0));
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
                                                            ui.label(egui::RichText::new(val_str).color(egui::Color32::from_rgb(209, 168, 255)).strong().size(12.0));
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

    fn render_floating_overlay_hud(&mut self, ctx: &egui::Context, state: &WarRoomState) {
        if !self.settings.overlay.enabled && !self.preview_overlay_hud {
            return;
        }

        let is_preview = (!state.is_connected || !state.player_is_in_car) && self.preview_overlay_hud;
        if (!state.is_connected || !state.player_is_in_car) && !is_preview {
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

        let opacity = self.settings.overlay.opacity.clamp(0.20, 1.0);
        let always_on_top = self.settings.overlay.always_on_top;
        let safety_margin = self.settings.overlay.safety_margin_laps;

        let scale = self.settings.overlay.hud_scale.clamp(0.75, 1.50);
        let show_pace = self.settings.overlay.show_pace_card;
        let show_fuel = self.settings.overlay.show_fuel_card;
        let show_rc = self.settings.overlay.show_race_control_card;

        // If user unchecked everything, default back to showing all cards so window isn't empty
        let (show_pace, show_fuel, show_rc) = if !show_pace && !show_fuel && !show_rc {
            (true, true, true)
        } else {
            (show_pace, show_fuel, show_rc)
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

            let player_lap = player.map_or(0, |p| p.total_laps_completed);
            let pit_target_lap = player_lap + (laps_of_fuel as i32).max(0);

            let laps_remain = if state.session_laps_remain > 0 && state.session_laps_remain < 10000 {
                state.session_laps_remain
            } else {
                (state.session_laps_total - player_lap).max(0)
            };

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
                ("🏳 White Flag", white)
            } else {
                ("● Green", green)
            };

            (stint_num, stint_lap_count, stint_avg, last_lap, fast_lap, comp_header, comp_last_lap, comp_sub, delta_opt, fuel_level_ltr, laps_of_fuel, pit_target_lap, laps_remain, player_lap, fuel_burn_rate_ltr, fuel_to_add_display, has_fuel_calc, inc_str, inc_color, caution_str, caution_color, flag_text, flag_color, player_pos)
        };

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
        if show_rc {
            base_h += 29.0;
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

            let bg_alpha = (opacity * 255.0) as u8;
            let bg_color = egui::Color32::from_rgba_unmultiplied(11, 13, 18, bg_alpha);
            let card_fill = egui::Color32::from_rgba_unmultiplied(19, 22, 29, (bg_alpha as f32 * 0.92) as u8);
            let card_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 14));

            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(bg_color).rounding(12.0).inner_margin(8.0 * scale).stroke(card_stroke))
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0 * scale, (5.0 * scale).max(3.0));

                    // --- SECTION 1: STINT & ROLLING PACE ---
                    if show_pace {
                        egui::Frame::none()
                            .fill(card_fill)
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
                                        let rem_str = if laps_remain > 0 {
                                            format!("{} laps", laps_remain)
                                        } else {
                                            "Unlimited".to_string()
                                        };
                                        ui.label(egui::RichText::new(rem_str).font(display_font(hero_font_size)).color(white));
                                        ui.label(egui::RichText::new(format!("Lap {} done", player_lap)).color(muted).size(sub_font_size));
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

                    // --- SECTION 3: RACE CONTROL, INCIDENTS & CAUTIONS BAR ---
                    if show_rc {
                        egui::Frame::none()
                            .fill(card_fill)
                            .rounding(8.0)
                            .inner_margin(egui::Margin::symmetric(card_padding_x, (5.0 * scale).max(3.0)))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0 * scale;
                                    ui.label(egui::RichText::new(&inc_str).color(inc_color).size(rc_font_size).strong());

                                    ui.add_space(10.0 * scale);

                                    ui.label(egui::RichText::new(&caution_str).color(caution_color).size(rc_font_size));

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.label(egui::RichText::new(flag_text).color(flag_color).size(rc_font_size).strong());
                                    });
                                });
                            });
                    }
                });

            let current_size = ctx.input(|i| i.viewport().inner_rect.map(|r| [r.width(), r.height()]));
            if let Some([cw, ch]) = current_size {
                if (cw - target_width).abs() > 2.0 || (ch - target_height).abs() > 2.0 {
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(target_width, target_height)));
                }
            }
        });
    }
}

impl eframe::App for WarRoomApp {
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

                // Segmented View Switcher (Cockpit | Grid)
                let is_cockpit = self.settings.view_mode == ActiveViewMode::CockpitEngineer;
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
                            self.settings.save();
                        }

                        let g_btn = egui::Button::new(
                            egui::RichText::new("Grid")
                                .size(12.5)
                                .color(if !is_cockpit { white } else { muted })
                        )
                        .fill(if !is_cockpit { self.settings.panel_color() } else { egui::Color32::TRANSPARENT })
                        .rounding(egui::Rounding::same(6.0));
                        if ui.add(g_btn).clicked() {
                            self.settings.view_mode = ActiveViewMode::GridTable;
                            self.settings.save();
                        }
                    });

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

                let wind_full = if wind_dir_str.is_empty() {
                    wind_spd_str
                } else {
                    format!("{} {}", wind_spd_str, wind_dir_str)
                };
                let cond_summary = format!(
                    "☀ {} · Air {} · Track {} · Wind {} · RH {}",
                    sky_str, air_str, track_str, wind_full, rh_str
                );
                egui::Frame::none().fill(pill_bg).rounding(pill_round).inner_margin(pill_margin).show(ui, |ui| {
                    ui.label(egui::RichText::new(cond_summary).color(muted).size(12.0));
                });

                // Right-side controls: Live pill, Overlay, Overflow menu (⋯), Settings (⚙)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_sized(egui::vec2(32.0, 30.0), egui::Button::new(egui::RichText::new("⚙").color(white).size(14.0)).fill(elevated)).on_hover_text("Settings & Layouts").clicked() {
                        self.show_settings_modal = true;
                    }

                    ui.menu_button(egui::RichText::new("⋯").color(white).size(15.0), |ui| {
                        ui.set_min_width(210.0);
                        if ui.button("📖 What's New & Release Notes").clicked() {
                            self.show_release_notes_modal = true;
                            ui.close_menu();
                        }
                        if ui.button("Stint Telemetry Analyzer").clicked() {
                            self.show_stint_analyzer_modal = true;
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
                        if ui.button("🔄 Reset / Clear Session Stints").on_hover_text("Cleanly resets stint counts, lap numbers, and best lap times for a new session or run, while preserving baseline fuel burn rate.").clicked() {
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
                                let icon = if info_clone.is_prerelease { "📢" } else { "🚀" };
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
                                    if ui.button(egui::RichText::new("✕ Dismiss").color(muted).size(11.5)).clicked() {
                                        self.updater.dismiss();
                                    }
                                    if ui.button(egui::RichText::new("Skip this version").color(muted).size(11.5)).clicked() {
                                        self.settings.updates.ignored_version = Some(info_clone.tag_name.clone());
                                        self.settings.save();
                                        self.updater.dismiss();
                                    }
                                    if ui.button(egui::RichText::new("What's New 📖").color(white).size(12.0)).clicked() {
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
                                    format!("📥 Downloading {}: {:.0}% ({:.1} / {:.1} MB)", tag, percent, mb_down, mb_tot)
                                } else {
                                    format!("📥 Downloading {}: {:.1} MB", tag, mb_down)
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
                                        egui::RichText::new("Restart to Apply 🔄").color(egui::Color32::BLACK).strong().size(12.0)
                                    ).fill(self.settings.status_green());
                                    if ui.add(restart_btn).clicked() {
                                        let _ = updater::apply_update_and_restart();
                                    }
                                });
                            }
                            updater::UpdateStatus::Error(err) => {
                                ui.label(egui::RichText::new(format!("⚠ Update notice: {}", err)).color(egui::Color32::from_rgb(255, 120, 120)).size(12.0));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button(egui::RichText::new("✕ Dismiss").color(muted).size(11.5)).clicked() {
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
                }
            });

        self.render_settings_modal(ctx);
        self.render_telemetry_debugger(ctx, &current_state);
        self.render_stint_analyzer_modal(ctx, &current_state);
        self.render_release_notes_modal(ctx);
        self.render_floating_overlay_hud(ctx, &current_state);

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

impl WarRoomState {
    pub fn clone_state_for_ui(&self) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(parsed.fuel_hero_metric, FuelHeroMetric::FuelAmount);
        assert!((parsed.hud_scale - 1.0).abs() < 0.001);

        // 2. Custom settings serialization & roundtrip
        let mut custom = parsed;
        custom.fuel_hero_metric = FuelHeroMetric::LapsToEmpty;
        custom.show_pace_card = false;
        custom.hud_scale = 1.20;

        let serialized = serde_json::to_string(&custom).expect("Must serialize");
        let roundtrip: settings::OverlaySettings = serde_json::from_str(&serialized).expect("Roundtrip must parse");
        assert_eq!(roundtrip.fuel_hero_metric, FuelHeroMetric::LapsToEmpty);
        assert!(!roundtrip.show_pace_card);
        assert!(roundtrip.show_fuel_card);
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
}


