use std::collections::HashMap;
use crate::stint_db;
use crate::telemetry::{is_real_track_name, read_player_tires};

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

    pub fn has_wear_or_temps(&self) -> bool {
        self.lf.temp_m > 5.0 || self.rf.temp_m > 5.0 || self.lf.wear_m > 0.1 || self.rf.wear_m > 0.1
    }

    pub fn should_trigger_popup(&self, new_tires: &StintTireSet) -> bool {
        if !new_tires.has_wear_or_temps() {
            return false;
        }
        if !self.has_data || !self.has_wear_or_temps() {
            return true;
        }
        (self.lf.wear_m - new_tires.lf.wear_m).abs() > 0.5
            || (self.rf.wear_m - new_tires.rf.wear_m).abs() > 0.5
            || (self.lr.wear_m - new_tires.lr.wear_m).abs() > 0.5
            || (self.rr.wear_m - new_tires.rr.wear_m).abs() > 0.5
            || (self.lf.temp_m - new_tires.lf.temp_m).abs() > 1.0
            || (self.rf.temp_m - new_tires.rf.temp_m).abs() > 1.0
            || (self.lr.temp_m - new_tires.lr.temp_m).abs() > 1.0
            || (self.rr.temp_m - new_tires.rr.temp_m).abs() > 1.0
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct StintData {
    pub stint_number: i32,
    pub start_lap: i32,
    pub end_lap: i32,
    pub out_lap_completed: bool,
    pub awaiting_pace_lap_sf: bool,
    pub awaiting_pit_out_lap_sf: bool,
    pub laps_completed: i32,
    pub lap_times: Vec<f32>,
    /// iRacing session lap number (`CarIdxLapCompleted` at the S/F crossing) for each entry in `lap_times`.
    /// Older saved stints may not have this; `numbered_valid_laps()` falls back to `start_lap + i + 1`.
    pub lap_numbers: Vec<i32>,
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
    pub is_imported: bool,
    pub saved_id: String,
    pub saved_label: String,
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
            lap_numbers: Vec::new(),
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
            is_imported: false,
            saved_id: String::new(),
            saved_label: String::new(),
        }
    }
}

pub const MAX_VALID_LAP_TIME_SECS: f32 = 1800.0; // 30 minutes: accommodates Nürburgring Nordschleife, Le Mans, wet conditions & incident limps

impl StintData {
    pub fn short_label(&self) -> String {
        if self.is_imported {
            let idx = if self.stint_number > 100 {
                self.stint_number - 100
            } else {
                self.stint_number.max(1)
            };
            format!("IMP {}", idx)
        } else {
            format!("S{}", self.stint_number)
        }
    }

    pub fn valid_laps(&self) -> Vec<f32> {
        self.lap_times.iter().copied().filter(|&t| t > 5.0 && t < MAX_VALID_LAP_TIME_SECS).collect()
    }

    /// Same laps as `valid_laps()`, paired with the iRacing session lap number each time belongs to.
    pub fn numbered_valid_laps(&self) -> Vec<(i32, f32)> {
        let has_numbers = self.lap_numbers.len() == self.lap_times.len();
        self.lap_times
            .iter()
            .copied()
            .enumerate()
            .filter(|&(_, t)| t > 5.0 && t < MAX_VALID_LAP_TIME_SECS)
            .map(|(i, t)| {
                let lap_num = if has_numbers { self.lap_numbers[i] } else { self.start_lap + (i as i32) + 1 };
                (lap_num, t)
            })
            .collect()
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
    /// Session time when `CarIdxLapCompleted` incremented but `CarIdxLastLapTime` has not (yet) changed.
    /// iRacing publishes the new lap time a few frames after the lap count; if it never changes the
    /// lap was genuinely identical to the previous one and is recorded after a grace period.
    pub pending_lap_count_since: f64,
    /// Session time when a lap was recorded from a lap-time change BEFORE the lap count incremented,
    /// so the late count increment is not treated as another lap.
    pub lap_time_ahead_since: f64,
}

impl DriverState {
    pub fn has_live_stints(&self) -> bool {
        self.stints.iter().any(|s| !s.is_imported)
    }

    pub fn last_live_stint(&self) -> Option<&StintData> {
        self.stints.iter().rev().find(|s| !s.is_imported)
    }

    pub fn last_live_stint_mut(&mut self) -> Option<&mut StintData> {
        self.stints.iter_mut().rev().find(|s| !s.is_imported)
    }

    pub fn pop_last_live_stint(&mut self) {
        if let Some(pos) = self.stints.iter().rposition(|s| !s.is_imported) {
            self.stints.remove(pos);
        }
    }

    pub fn next_live_stint_number(&self) -> i32 {
        self.stints
            .iter()
            .filter(|s| !s.is_imported)
            .map(|s| s.stint_number)
            .max()
            .unwrap_or(0)
            + 1
    }

    pub fn next_imported_stint_number(&self) -> i32 {
        self.stints
            .iter()
            .filter(|s| s.is_imported)
            .map(|s| s.stint_number)
            .max()
            .unwrap_or(100)
            + 1
    }

    pub fn estimated_fuel_burn_ltr(&self) -> f32 {
        // 1. If active live stint has at least 1 lap recorded, use active stint rate
        if let Some(active) = self.stints.iter().rev().find(|s| !s.is_imported && s.is_active) {
            let b = active.fuel_per_lap();
            if b > 0.001 {
                return b;
            }
        }
        // 2. Fallback to previous closed live stint with valid fuel data in this session
        for stint in self.stints.iter().rev().filter(|s| !s.is_imported) {
            let b = stint.fuel_per_lap();
            if b > 0.001 {
                return b;
            }
        }
        // 3. Fallback to session-wide valid lap burn average across all live stints in this session
        let all_burns: Vec<f32> = self.stints.iter()
            .filter(|s| !s.is_imported)
            .flat_map(|s| s.lap_fuel_burns.iter().copied())
            .filter(|&b| b > 0.005 && b < 25.0)
            .collect();
        if !all_burns.is_empty() {
            return all_burns.iter().sum::<f32>() / all_burns.len() as f32;
        }
        // 4. Fallback to historical fuel burn rate from earlier sessions or Stint Database
        if self.historical_fuel_burn_rate > 0.001 {
            return self.historical_fuel_burn_rate;
        }
        // 5. Fallback to any re-imported stint with valid fuel burn
        for stint in self.stints.iter().rev().filter(|s| s.is_imported) {
            let b = stint.fuel_per_lap();
            if b > 0.001 {
                return b;
            }
        }
        0.0
    }
}

#[derive(Clone)]
pub struct WarRoomState {
    pub is_connected: bool,
    pub session_run_token: u64,
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
    pub tire_popup_trigger_seq: u64,
    pub tire_popup_stint_num: i32,
    pub player_is_in_car: bool,
    pub player_yaw_north: f32,
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
            session_run_token: stint_db::current_unix_millis(),
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
            tire_popup_trigger_seq: 0,
            tire_popup_stint_num: 1,
            player_is_in_car: false,
            player_yaw_north: 0.0,
            player_incidents: 0,
            incident_limit: 0,
            caution_count: 0,
            caution_laps: 0,
            is_under_caution: false,
            is_oval: false,
            was_race_green: false,
            had_caution_since_last_green: bool::default(),
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
    pub fn import_saved_stint(&mut self, record: &stint_db::SavedStintRecord) -> i32 {
        // Populate session track/car/weather metadata if we are currently in Standby/Offline
        if !is_real_track_name(&self.track_name) && !stint_db::is_placeholder_track(&record.track_name) {
            self.track_name = record.track_name.clone();
            if !record.track_length.is_empty() && record.track_length != "—" {
                self.track_length = record.track_length.clone();
            }
            if !record.car_name.is_empty() && record.car_name != "—" {
                self.car_name = record.car_name.clone();
            }
            if !record.session_type.is_empty() && record.session_type != "—" {
                self.session_type = record.session_type.clone();
            }
            if !record.track_state.is_empty() && record.track_state != "—" {
                self.track_state = record.track_state.clone();
            }
            if !record.air_temp.is_empty() && record.air_temp != "—" {
                self.air_temp_raw = record.air_temp.clone();
            }
            if !record.track_temp.is_empty() && record.track_temp != "—" {
                self.track_temp_raw = record.track_temp.clone();
            }
        }

        let player_idx = self.player_car_idx;
        let drv = self.drivers.entry(player_idx).or_insert_with(|| DriverState {
            car_idx: player_idx,
            driver_name: if !record.driver_name.is_empty() {
                record.driver_name.clone()
            } else {
                "Player Car".to_string()
            },
            car_num: if !record.car_num.is_empty() && record.car_num != "0" {
                record.car_num.clone()
            } else {
                "1".to_string()
            },
            irating: "—".to_string(),
            position: 1,
            is_on_pit_road: true,
            is_in_world: false,
            is_initialized: false,
            total_laps_completed: 0,
            last_processed_lap: 0,
            last_known_lap_time: 0.0,
            last_sf_crossing_time: 0.0,
            exited_pits_on_current_lap: false,
            ignored_lap_sdk: -1,
            setup_name: if !record.setup_name.is_empty() {
                record.setup_name.clone()
            } else {
                "Baseline".to_string()
            },
            stints: Vec::new(),
            historical_fuel_burn_rate: 0.0,
            pending_lap_count_since: 0.0,
            lap_time_ahead_since: 0.0,
        });

        // If this exact saved stint is already imported, return its existing stint_number
        if !record.id.is_empty() {
            if let Some(existing) = drv.stints.iter().find(|st| st.is_imported && st.saved_id == record.id) {
                return existing.stint_number;
            }
        }

        let imp_num = drv.next_imported_stint_number();
        let mut imp_stint = record.stint.clone();
        imp_stint.stint_number = imp_num;
        imp_stint.is_active = false;
        imp_stint.is_imported = true;
        imp_stint.saved_id = record.id.clone();
        imp_stint.saved_label = record.short_badge_label();
        if imp_stint.setup_name.is_empty() || imp_stint.setup_name == "—" || imp_stint.setup_name == "Baseline" {
            if !record.setup_name.is_empty() {
                imp_stint.setup_name = record.setup_name.clone();
            }
        }
        if imp_stint.track_temp_at_start.is_empty() || imp_stint.track_temp_at_start == "—" {
            imp_stint.track_temp_at_start = record.track_temp.clone();
        }
        if imp_stint.air_temp_at_start.is_empty() || imp_stint.air_temp_at_start == "—" {
            imp_stint.air_temp_at_start = record.air_temp.clone();
        }

        let burn = imp_stint.fuel_per_lap();
        if drv.historical_fuel_burn_rate <= 0.001 && burn > 0.005 {
            drv.historical_fuel_burn_rate = burn;
        }

        // Insert imported stints before any live stints so live stints remain at the end of drv.stints
        let insert_idx = drv.stints.iter().position(|st| !st.is_imported).unwrap_or(drv.stints.len());
        drv.stints.insert(insert_idx, imp_stint);
        imp_num
    }

    pub fn remove_imported_stint(&mut self, stint_number: i32) -> bool {
        let player_idx = self.player_car_idx;
        if let Some(drv) = self.drivers.get_mut(&player_idx) {
            if let Some(pos) = drv.stints.iter().position(|st| st.is_imported && st.stint_number == stint_number) {
                drv.stints.remove(pos);
                return true;
            }
        }
        false
    }

    pub fn reset_session_data(&mut self, manual: bool) {
        // 0. Persist any completed live player stints to the local Stint Database before clearing
        stint_db::auto_save_session_player_stints(self);

        // 1. Archive current session data before clearing if any laps or stints exist
        if !self.drivers.is_empty() && self.drivers.values().any(|d| d.has_live_stints() || d.total_laps_completed > 0) {
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
            if manual {
                driver.stints.clear();
            } else {
                driver.stints.retain(|st| st.is_imported);
            }
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
        self.session_run_token = stint_db::current_unix_millis();
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
        self.tire_popup_trigger_seq = 0;
        self.tire_popup_stint_num = 1;
        self.player_yaw_north = 0.0;

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
            if self.drivers.values().any(|d| d.has_live_stints() || d.total_laps_completed > 0) {
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
                pending_lap_count_since: 0.0,
                lap_time_ahead_since: 0.0,
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
                driver.total_laps_completed = current_lap_sdk.max(0);
                driver.last_known_lap_time = last_lap_time;
                driver.is_on_pit_road = is_pit;
                driver.is_in_world = in_world;

                // If car is ALREADY on track driving when app connects (and in a race, only if green), start Stint 1 immediately!
                if !is_pit && in_world && !driver.has_live_stints() && is_race_green && !green_just_started {
                    let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                        let eff = driver.estimated_fuel_burn_ltr();
                        if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                    } else { 0.0 };
                    let prior_burn_rate = driver.estimated_fuel_burn_ltr();

                    driver.stints.push(StintData {
                        stint_number: 1,
                        start_lap: current_lap_sdk.max(0),
                        end_lap: current_lap_sdk.max(0),
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
                        ..Default::default()
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

            if let Some(active) = driver.last_live_stint_mut() {
                if active.is_active && (active.setup_name.is_empty() || active.setup_name == "—" || active.setup_name == "Baseline") {
                    if active_setup != "Baseline" {
                        active.setup_name = active_setup.clone();
                    }
                }
            }

            // Sample active dynamic on-track pressures and fuel for player car
            if idx == player_idx && !is_pit && in_world {
                let [cur_lf_p, cur_rf_p, cur_lr_p, cur_rr_p] = frame.player_live_pressures;

                if let Some(active) = driver.last_live_stint_mut() {
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

                if let Some(active) = driver.last_live_stint_mut() {
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
                    driver.pop_last_live_stint();
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
                if let Some(active) = driver.last_live_stint_mut() {
                    active.is_active = false;
                    active.end_lap = current_lap_sdk;
                }
                if driver.last_live_stint().map_or(false, |last| last.laps_completed == 0) {
                    driver.pop_last_live_stint();
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

                let new_stint_num = driver.next_live_stint_number();
                let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                    let eff = driver.estimated_fuel_burn_ltr();
                    if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                } else { 0.0 };
                let prior_burn_rate = driver.estimated_fuel_burn_ltr();

                driver.stints.push(StintData {
                    stint_number: new_stint_num,
                    start_lap: current_lap_sdk.max(0),
                    end_lap: current_lap_sdk.max(0),
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
                    ..Default::default()
                });
                driver.last_processed_lap = current_lap_sdk;
                driver.total_laps_completed = current_lap_sdk.max(0);
                driver.last_known_lap_time = last_lap_time;

                if idx == player_idx {
                    pending_logs.push(format!("[GREEN FLAG] Stint S{} Started at Lap {}", new_stint_num, current_lap_sdk));
                }
                state_changed = true;
            }

            // 1. CAR EXITED PITS / ENTERED TRACK (Begin Stint if green)
            if (was_in_pit && !is_pit && in_world) || (!was_in_world && in_world && !is_pit) {
                if is_race_green && !green_just_started {
                    if let Some(active) = driver.last_live_stint_mut() {
                        active.is_active = false;
                        active.end_lap = current_lap_sdk.max(0);
                    }

                    // Discard previous 0-lap stint if aborted before completing any timed laps
                    if driver.last_live_stint().map_or(false, |last| last.laps_completed == 0) {
                        driver.pop_last_live_stint();
                    }

                    let new_stint_num = driver.next_live_stint_number();
                    let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                        let eff = driver.estimated_fuel_burn_ltr();
                        if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                    } else { 0.0 };
                    let prior_burn_rate = driver.estimated_fuel_burn_ltr();
                    let awaiting_pit_out_lap_sf = is_race_session && current_lap_sdk > 0;

                    driver.stints.push(StintData {
                        stint_number: new_stint_num,
                        start_lap: current_lap_sdk.max(0),
                        end_lap: current_lap_sdk.max(0),
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
                        ..Default::default()
                    });
                    if idx == player_idx {
                        pending_logs.push(format!("[PIT EXIT] Stint S{} Started at Lap {}", new_stint_num, current_lap_sdk));
                    }
                    driver.last_processed_lap = current_lap_sdk;
                    driver.total_laps_completed = current_lap_sdk.max(0);
                    driver.last_known_lap_time = last_lap_time;
                }
                state_changed = true;
            }

            // 2. CAR ENTERED PITS OR EXITED CAR (Stint Ends / Reset)
            if (!was_in_pit && is_pit) || (was_in_world && !in_world) {
                let mut aborted = false;
                let mut stint_num = 0;
                let mut completed_laps = 0;

                if let Some(active) = driver.last_live_stint_mut() {
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
                    driver.pop_last_live_stint();
                    if idx == player_idx {
                        pending_logs.push(format!("[ABORTED RUN] Stint S{} had 0 completed laps; discarded to prevent empty stint line.", stint_num));
                    }
                } else if stint_num > 0 && idx == player_idx {
                    pending_logs.push(format!("[PIT ENTRY] Stint S{} Closed (Total {} Laps) · Awaiting Pit Stop Service", stint_num, completed_laps));
                }

                if is_race_green {
                    driver.last_processed_lap = current_lap_sdk;
                    driver.total_laps_completed = current_lap_sdk.max(0);
                    driver.last_known_lap_time = last_lap_time;
                }
                state_changed = true;
            }

            // 3. NEW LAP TIME / COUNT EMITTED BY IRACING ON TRACK (Direct SDK Fire)
            // iRacing times laps to 0.1 ms, so any change larger than 0.05 ms is a new lap time. (A 0.5 ms threshold used to
            // silently drop a lap whenever two consecutive laps were within half a millisecond, e.g. 33.1875 → 33.1871.)
            let is_new_lap_time = (last_lap_time - driver.last_known_lap_time).abs() > 0.00005 && last_lap_time > 3.0;
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
                            if let Some(last_stint) = driver.last_live_stint_mut() {
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
                    driver.total_laps_completed = current_lap_sdk.max(0);
                    state_changed = true;
                }
            } else if current_lap_sdk > 0 && current_lap_sdk == driver.ignored_lap_sdk && (is_new_lap_time || is_new_lap_count) {
                // 3B. Delayed last_lap_time update for a caution/pace/out-lap that was already ignored at S/F
                driver.last_known_lap_time = last_lap_time;
                driver.last_processed_lap = current_lap_sdk;
                driver.total_laps_completed = current_lap_sdk.max(0);
            } else if (is_new_lap_count || is_new_lap_time) && !is_pit && in_world && driver.last_live_stint().map_or(false, |st| st.is_active && (st.awaiting_pace_lap_sf || st.awaiting_pit_out_lap_sf)) {
                // 3C. First S/F crossing after a caution restart (finishing pace lap) or race green-flag pit exit (finishing pit out-lap)
                if let Some(active) = driver.stints.iter_mut().rev().find(|st| !st.is_imported) {
                    let was_pace_lap = active.awaiting_pace_lap_sf;
                    active.awaiting_pace_lap_sf = false;
                    active.awaiting_pit_out_lap_sf = false;
                    active.out_lap_completed = true;
                    active.start_lap = current_lap_sdk.max(0);
                    active.end_lap = current_lap_sdk.max(0);
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
                driver.total_laps_completed = current_lap_sdk.max(0);
                state_changed = true;
            } else {
                // 3D. UNTIMED OUT-LAP COMPLETION (Start/Finish crossed for the first time since pit exit in Practice/Qualifying)
                let mut out_lap_handled_this_frame = false;
                if is_new_lap_count && !is_pit && in_world && !is_new_lap_time {
                    if let Some(active) = driver.stints.iter_mut().rev().find(|st| !st.is_imported) {
                        if active.is_active && !active.out_lap_completed {
                            active.out_lap_completed = true;
                            // The out-lap consumed session lap `current_lap_sdk`; Flying Lap 1 will be session lap `current_lap_sdk + 1`
                            active.start_lap = current_lap_sdk.max(0);
                            active.end_lap = current_lap_sdk.max(0);
                            active.prev_crossing_fuel = current_fuel_sample;
                            active.crossing_fuel_level = current_fuel_sample;
                            if active.latched_fuel_burn_rate > 0.001 {
                                active.laps_of_fuel_crossing = (current_fuel_sample / active.latched_fuel_burn_rate).max(0.0);
                            }
                            driver.last_processed_lap = current_lap_sdk;
                            driver.total_laps_completed = current_lap_sdk.max(0);
                            // If iRacing publishes an out-lap time a few frames late, 3B absorbs it instead of counting it as Lap 1
                            driver.ignored_lap_sdk = current_lap_sdk;
                            driver.pending_lap_count_since = 0.0;
                            out_lap_handled_this_frame = true;
                            if idx == player_idx {
                                pending_logs.push(format!("[OUT LAP COMPLETED] Stint S{}: Crossed start/finish at Lap {}. Latched fuel at {:.2} L for Flying Lap 1", active.stint_number, current_lap_sdk, current_fuel_sample));
                            }
                            state_changed = true;
                        }
                    }
                }

                // Decide whether a completed lap should be recorded this frame.
                // `CarIdxLapCompleted` is the authority for "a lap was completed"; `CarIdxLastLapTime` normally
                // changes a few frames later. A lap is recorded when:
                //   a) the lap time changes (with or without the count having incremented yet), or
                //   b) the count incremented but the time stayed identical for LAP_TIME_GRACE_S seconds —
                //      i.e. two consecutive laps with the exact same time (previously these were silently dropped).
                const LAP_TIME_GRACE_S: f64 = 3.0;
                let mut record_identical_lap = false;
                if is_pit || !in_world {
                    driver.pending_lap_count_since = 0.0;
                } else if is_new_lap_count && !is_new_lap_time && !out_lap_handled_this_frame {
                    let count_catching_up = driver.lap_time_ahead_since > 0.0
                        && (frame.cur_session_time - driver.lap_time_ahead_since) <= LAP_TIME_GRACE_S;
                    if count_catching_up {
                        // Lap was already recorded when its time arrived ahead of the count; just sync the count
                        driver.lap_time_ahead_since = 0.0;
                        driver.pending_lap_count_since = 0.0;
                        driver.last_processed_lap = current_lap_sdk;
                        driver.total_laps_completed = current_lap_sdk.max(0);
                    } else if driver.pending_lap_count_since <= 0.0 {
                        driver.pending_lap_count_since = frame.cur_session_time;
                    } else if (frame.cur_session_time - driver.pending_lap_count_since) >= LAP_TIME_GRACE_S {
                        if last_lap_time > 3.0 {
                            record_identical_lap = true;
                        } else {
                            // Lap completed without a usable time (e.g. invalidated): consume the count, record nothing
                            driver.pending_lap_count_since = 0.0;
                            driver.last_processed_lap = current_lap_sdk;
                            driver.total_laps_completed = current_lap_sdk.max(0);
                        }
                    }
                }

                if (is_new_lap_time || record_identical_lap) && !is_pit && in_world {
                    // Session lap number this time belongs to. If the time arrived before the count incremented,
                    // `CarIdxLapCompleted` is still one behind.
                    let completed_lap_num = if is_new_lap_count || record_identical_lap { current_lap_sdk } else { current_lap_sdk + 1 };
                    if !driver.has_live_stints() || !driver.last_live_stint().map_or(false, |st| st.is_active) {
                        let new_stint_num = driver.next_live_stint_number();
                        let initial_laps_of_fuel = if current_fuel_sample > 0.0 {
                            let eff = driver.estimated_fuel_burn_ltr();
                            if eff > 0.001 { (current_fuel_sample / eff).max(0.0) } else { 0.0 }
                        } else { 0.0 };
                        let prior_burn_rate = driver.estimated_fuel_burn_ltr();

                        driver.stints.push(StintData {
                            stint_number: new_stint_num,
                            start_lap: (completed_lap_num - 1).max(0),
                            end_lap: completed_lap_num.max(0),
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
                            ..Default::default()
                        });
                    }

                    if let Some(active) = driver.stints.iter_mut().rev().find(|st| !st.is_imported) {
                        if last_lap_time > 3.0 && last_lap_time < MAX_VALID_LAP_TIME_SECS {
                            let was_uncompleted_out_lap = !active.out_lap_completed;
                            active.out_lap_completed = true;
                            active.end_lap = completed_lap_num.max(0);

                            if was_uncompleted_out_lap {
                                // The sim timed the out-lap itself. An out-lap is never a flying lap, so it is not added to
                                // the stint lap list (same as the untimed out-lap path 3D and the race pit-out path 3C).
                                active.start_lap = completed_lap_num.max(0);
                                if idx == player_idx && current_fuel_sample > 0.0 {
                                    // Latch fuel at start/finish line so Lap 1 is measured cleanly
                                    active.prev_crossing_fuel = current_fuel_sample;
                                    active.crossing_fuel_level = current_fuel_sample;
                                    if active.latched_fuel_burn_rate > 0.001 {
                                        active.laps_of_fuel_crossing = (current_fuel_sample / active.latched_fuel_burn_rate).max(0.0);
                                    }
                                }
                                if idx == player_idx {
                                    pending_logs.push(format!("[OUT LAP COMPLETED] Stint S{}: Timed out-lap {} ({:.3}s) dropped. Latched fuel at {:.2} L for Flying Lap 1", active.stint_number, completed_lap_num, last_lap_time, current_fuel_sample));
                                }
                            } else {
                                // Keep lap_numbers aligned 1:1 with lap_times
                                while active.lap_numbers.len() < active.lap_times.len() {
                                    let fill = active.start_lap + active.lap_numbers.len() as i32 + 1;
                                    active.lap_numbers.push(fill);
                                }
                                active.lap_times.push(last_lap_time);
                                active.lap_numbers.push(completed_lap_num);
                                active.last_lap = last_lap_time;
                                active.laps_completed = active.lap_times.len() as i32;

                                if active.fast_lap <= 0.0 || last_lap_time < active.fast_lap {
                                    active.fast_lap = last_lap_time;
                                }
                                if active.fuel_start <= 0.0 && current_fuel_sample > 0.0 {
                                    active.fuel_start = current_fuel_sample;
                                }

                                // Latch fuel at start/finish line crossing: calculate exact timed-lap burn delta
                                if idx == player_idx && current_fuel_sample > 0.0 {
                                    let is_anomalous_slow_lap = active.fast_lap > 3.0 && last_lap_time > active.fast_lap * 1.35;

                                    if !is_anomalous_slow_lap {
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
                                    let how = if record_identical_lap {
                                        " [identical to previous lap time]"
                                    } else if !is_new_lap_count {
                                        " [time arrived before lap count]"
                                    } else {
                                        ""
                                    };
                                    pending_logs.push(format!("[SDK LAP EMISSION] S{} Lap {} (Session Lap {}): Time = {:.3} (Prev = {:.3}){}", active.stint_number, active.laps_completed, completed_lap_num, last_lap_time, driver.last_known_lap_time, how));
                                }
                            }
                            state_changed = true;
                        }
                    }
                    driver.pending_lap_count_since = 0.0;
                    driver.lap_time_ahead_since = if is_new_lap_count || record_identical_lap { 0.0 } else { frame.cur_session_time };
                    driver.last_known_lap_time = last_lap_time;
                    driver.last_processed_lap = current_lap_sdk;
                    driver.total_laps_completed = current_lap_sdk.max(0);
                }
            }

            // Any branch above that consumed the lap count clears a pending identical-time lap
            if current_lap_sdk <= driver.last_processed_lap {
                driver.pending_lap_count_since = 0.0;
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
                let has_completed_laps = player_drv_opt.and_then(|d| d.last_live_stint()).map_or(false, |st| st.laps_completed > 0);

                if has_completed_laps {
                    let prev_hot = player_drv_opt.and_then(|d| d.last_live_stint()).map(|st| &st.tires);
                    let pit_tires = read_player_tires(live, prev_hot);
                    if pit_tires.is_valid() {
                        self.latest_tire_data = pit_tires.clone();
                        let mut triggered_stint_num = None;
                        if let Some(player_drv) = self.drivers.get_mut(&player_idx) {
                            if let Some(last_stint) = player_drv.last_live_stint_mut() {
                                if !last_stint.is_active {
                                    if last_stint.tires.should_trigger_popup(&pit_tires) {
                                        triggered_stint_num = Some(last_stint.stint_number);
                                    }
                                    last_stint.tires = pit_tires;
                                    state_changed = true;
                                }
                            }
                        }
                        if let Some(s_num) = triggered_stint_num {
                            self.tire_popup_trigger_seq += 1;
                            self.tire_popup_stint_num = s_num;
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

impl WarRoomState {
    pub fn clone_state_for_ui(&self) -> Self {
        self.clone()
    }
}
