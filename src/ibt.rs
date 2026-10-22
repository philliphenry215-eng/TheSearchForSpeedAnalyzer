use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, Encoding};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

// ============================================================================
// 1. CONSTANTS & CONVERSION FACTORS
// ============================================================================

pub const MPS_TO_MPH: f32 = 2.236936;
pub const MPS_TO_KPH: f32 = 3.6;
pub const METERS_TO_MM: f32 = 1000.0;
pub const METERS_TO_INCHES: f32 = 39.3700787;
pub const MM_TO_INCHES: f32 = 0.0393700787;
pub const KPA_TO_PSI: f32 = 0.145038;
pub const RAD_TO_DEG: f32 = 180.0 / std::f32::consts::PI;

pub const IRSDK_HEADER_SIZE: usize = 112;
pub const IRSDK_DISK_SUB_HEADER_SIZE: usize = 32;
pub const IRSDK_VAR_HEADER_SIZE: usize = 144;

/// Resolves the default iRacing telemetry directory: `%USERPROFILE%\Documents\iRacing\telemetry\`
pub fn default_telemetry_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(profile).join("Documents").join("iRacing").join("telemetry");
        return p;
    }
    if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        return PathBuf::from(format!("{}{}", drive, path))
            .join("Documents")
            .join("iRacing")
            .join("telemetry");
    }
    PathBuf::from("Documents/iRacing/telemetry")
}

#[derive(Clone, Debug)]
pub struct IbtFileEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub file_size_bytes: u64,
    pub modified_epoch_secs: u64,
    pub modified_display: String,
}

impl IbtFileEntry {
    pub fn size_mb(&self) -> f32 {
        self.file_size_bytes as f32 / (1024.0 * 1024.0)
    }
}

/// Scans a directory for `.ibt` files and returns them sorted by most recently modified first.
pub fn scan_ibt_directory(dir: &Path) -> Vec<IbtFileEntry> {
    let mut entries = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return entries,
    };

    for entry_res in read_dir.flatten() {
        let path = entry_res.path();
        if !path.is_file() {
            continue;
        }
        let is_ibt = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.eq_ignore_ascii_case("ibt"))
            .unwrap_or(false);
        if !is_ibt {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("telemetry.ibt")
            .to_string();

        let (file_size_bytes, modified_epoch_secs) = match entry_res.metadata() {
            Ok(meta) => {
                let sz = meta.len();
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                (sz, mtime)
            }
            Err(_) => (0, 0),
        };

        let modified_display = if modified_epoch_secs > 0 {
            format_unix_timestamp(modified_epoch_secs as i64)
        } else {
            "—".to_string()
        };

        entries.push(IbtFileEntry {
            path,
            file_name,
            file_size_bytes,
            modified_epoch_secs,
            modified_display,
        });
    }

    entries.sort_by(|a, b| b.modified_epoch_secs.cmp(&a.modified_epoch_secs));
    entries
}

pub fn format_unix_timestamp(ts: i64) -> String {
    if ts <= 0 {
        return "—".to_string();
    }
    // Civil date calculation (Howard Hinnant's algorithm, UTC)
    let days = ts.div_euclid(86400);
    let secs_of_day = ts.rem_euclid(86400);
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, m, d, hour, minute)
}

// ============================================================================
// 2. BINARY .IBT STRUCTURES & METADATA
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IbtVarType {
    Char,     // 0 (1 byte)
    Bool,     // 1 (1 byte)
    Int,      // 2 (4 bytes, i32)
    BitField, // 3 (4 bytes, u32)
    Float,    // 4 (4 bytes, f32)
    Double,   // 5 (8 bytes, f64)
}

impl IbtVarType {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            0 => Some(Self::Char),
            1 => Some(Self::Bool),
            2 => Some(Self::Int),
            3 => Some(Self::BitField),
            4 => Some(Self::Float),
            5 => Some(Self::Double),
            _ => None,
        }
    }

    pub fn byte_size(&self) -> usize {
        match self {
            Self::Char | Self::Bool => 1,
            Self::Int | Self::BitField | Self::Float => 4,
            Self::Double => 8,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IbtChannelInfo {
    pub name: String,
    pub desc: String,
    pub unit: String,
    pub var_type: IbtVarType,
    pub offset: usize,
    pub count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct IbtSessionMetadata {
    pub file_name: String,
    pub file_size_bytes: usize,
    pub track_display_name: String,
    pub track_config_name: String,
    pub track_length: String,
    pub track_type: String,
    pub car_name: String,
    pub car_number: String,
    pub driver_name: String,
    pub irating: String,
    pub setup_name: String,
    pub session_date: String,
    pub session_type: String,
    pub weather_type: String,
    pub skies: String,
    pub air_temp: String,
    pub track_temp: String,
    pub wind_speed: String,
    pub wind_dir: String,
    pub relative_humidity: String,
    pub tick_rate: i32,
    pub total_ticks: usize,
    pub total_laps_header: i32,
    pub session_duration_secs: f64,
    pub channel_count: usize,
}

#[derive(Clone, Debug)]
pub struct LapDescriptor {
    pub lap_index: usize,
    pub lap_number: i32,
    pub start_tick: usize,
    pub end_tick: usize, // exclusive
    pub tick_count: usize,
    pub duration_secs: f32,
    pub is_out_lap: bool,
    pub is_in_lap: bool,
    pub has_off_track: bool,
    pub is_complete: bool,
    pub is_valid_flying: bool,
    pub is_personal_best: bool,
    pub min_dist_pct: f32,
    pub max_dist_pct: f32,
}

impl LapDescriptor {
    pub fn status_badge(&self) -> &'static str {
        if self.is_personal_best {
            "PB ★"
        } else if self.is_out_lap {
            "OUT LAP"
        } else if self.is_in_lap {
            "IN LAP"
        } else if !self.is_complete {
            "INCOMPLETE"
        } else if self.has_off_track {
            "OFF-TRACK"
        } else {
            "VALID"
        }
    }
}

// ============================================================================
// 3. PROCESSED TELEMETRY DATA CONTRACTS (DECOUPLED PURE ARRAYS & SCALARS)
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct ChannelStats {
    pub min: f32,
    pub max: f32,
    pub avg: f32,
}

impl ChannelStats {
    pub fn from_slice(data: &[f32]) -> Self {
        if data.is_empty() {
            return Self::default();
        }
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        let mut sum = 0.0_f64;
        let mut count = 0usize;
        for &v in data {
            if v.is_finite() {
                if v < min {
                    min = v;
                }
                if v > max {
                    max = v;
                }
                sum += v as f64;
                count += 1;
            }
        }
        if count == 0 {
            Self::default()
        } else {
            Self {
                min,
                max,
                avg: (sum / count as f64) as f32,
            }
        }
    }
}

/// Section 3.A: Driver Inputs & Vehicle Dynamics
#[derive(Clone, Debug, Default)]
pub struct DriverInputsAnalysis {
    pub speed_mps: Option<Vec<f32>>,
    pub speed_mph: Option<Vec<f32>>,
    pub speed_kph: Option<Vec<f32>>,
    pub throttle_pct: Option<Vec<f32>>,
    pub brake_pct: Option<Vec<f32>>,
    pub clutch_pct: Option<Vec<f32>>,
    pub steering_rad: Option<Vec<f32>>,
    pub steering_deg: Option<Vec<f32>>,
    pub gear: Option<Vec<i32>>,
    pub rpm: Option<Vec<f32>>,
    pub lat_accel_g: Option<Vec<f32>>,
    pub long_accel_g: Option<Vec<f32>>,

    pub speed_mph_stats: ChannelStats,
    pub speed_kph_stats: ChannelStats,
    pub throttle_stats: ChannelStats,
    pub brake_stats: ChannelStats,
    pub steering_deg_stats: ChannelStats,
    pub rpm_stats: ChannelStats,
    pub full_throttle_pct_of_lap: f32,
    pub braking_pct_of_lap: f32,
    pub coasting_pct_of_lap: f32,
}

/// Section 3.B: Suspension Dynamics, Ride Heights, Dynamic Pitch Rake & Side Rake
#[derive(Clone, Debug, Default)]
pub struct SuspensionAnalysis {
    // Ride heights stored in millimeters (mm); multiply by MM_TO_INCHES for imperial inches
    pub lf_rh_mm: Option<Vec<f32>>,
    pub rf_rh_mm: Option<Vec<f32>>,
    pub lr_rh_mm: Option<Vec<f32>>,
    pub rr_rh_mm: Option<Vec<f32>>,
    pub cfsr_rh_mm: Option<Vec<f32>>, // Center floor / splitter ride height if available

    // Derived dynamic pitch & roll arrays (in mm)
    pub front_rh_mm: Option<Vec<f32>>,   // (LF + RF) / 2
    pub rear_rh_mm: Option<Vec<f32>>,    // (LR + RR) / 2
    pub rake_mm: Option<Vec<f32>>,       // Rear RH - Front RH (+ = tail-up / nose-down)
    pub left_rh_mm: Option<Vec<f32>>,    // (LF + LR) / 2
    pub right_rh_mm: Option<Vec<f32>>,   // (RF + RR) / 2
    pub side_rake_mm: Option<Vec<f32>>,  // Left RH - Right RH
    pub front_split_mm: Option<Vec<f32>>, // LF - RF
    pub rear_split_mm: Option<Vec<f32>>,  // LR - RR

    // Scalar statistics (in mm)
    pub lf_stats_mm: ChannelStats,
    pub rf_stats_mm: ChannelStats,
    pub lr_stats_mm: ChannelStats,
    pub rr_stats_mm: ChannelStats,
    pub cfsr_stats_mm: Option<ChannelStats>,
    pub front_rh_stats_mm: ChannelStats,
    pub rear_rh_stats_mm: ChannelStats,
    pub rake_stats_mm: ChannelStats,
    pub side_rake_stats_mm: ChannelStats,
    pub front_split_stats_mm: ChannelStats,
    pub rear_split_stats_mm: ChannelStats,
}

/// Section 3.C: Shock / Damper Velocity Distribution & Histograms
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShockVelocitySource {
    Unavailable,
    NativeChannel,
    DerivedFromDeflection,
}

impl ShockVelocitySource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unavailable => "Unavailable",
            Self::NativeChannel => "Native (*shockVel)",
            Self::DerivedFromDeflection => "Derived (ΔDefl/Δt + Low-Pass Filter)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamperRegime {
    HighSpeedRebound,     // < -50 mm/s
    LowSpeedRebound,      // -50 to 0 mm/s
    LowSpeedCompression,  // 0 to +50 mm/s
    HighSpeedCompression, // > +50 mm/s
}

#[derive(Clone, Debug)]
pub struct ShockHistogramBin {
    pub min_mms: f32,
    pub max_mms: f32,
    pub center_mms: f32,
    pub count: usize,
    pub time_pct: f32,
    pub regime: DamperRegime,
}

#[derive(Clone, Debug)]
pub struct ShockCornerData {
    pub corner_name: &'static str,
    pub source: ShockVelocitySource,
    pub deflection_mm: Option<Vec<f32>>,
    pub velocity_mms: Option<Vec<f32>>,
    pub bins: Vec<ShockHistogramBin>,
    pub total_binned_pct: f32,
    pub low_speed_compression_pct: f32,  // 0 to +50 mm/s
    pub low_speed_rebound_pct: f32,      // -50 to 0 mm/s
    pub high_speed_compression_pct: f32, // > +50 mm/s
    pub high_speed_rebound_pct: f32,     // < -50 mm/s
    pub avg_compression_mms: f32,
    pub avg_rebound_mms: f32,
    pub peak_compression_mms: f32,
    pub peak_rebound_mms: f32,
}

impl Default for ShockCornerData {
    fn default() -> Self {
        Self {
            corner_name: "",
            source: ShockVelocitySource::Unavailable,
            deflection_mm: None,
            velocity_mms: None,
            bins: Vec::new(),
            total_binned_pct: 0.0,
            low_speed_compression_pct: 0.0,
            low_speed_rebound_pct: 0.0,
            high_speed_compression_pct: 0.0,
            high_speed_rebound_pct: 0.0,
            avg_compression_mms: 0.0,
            avg_rebound_mms: 0.0,
            peak_compression_mms: 0.0,
            peak_rebound_mms: 0.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ShockAnalysis {
    pub lf: ShockCornerData,
    pub rf: ShockCornerData,
    pub lr: ShockCornerData,
    pub rr: ShockCornerData,
    pub has_any_shock_data: bool,
}

/// Section 3.D: Tire Surface / Garage Hot / Carcass Temperatures & Pressures
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TireTempSource {
    Unavailable,
    Surface,        // *tempL / *tempM / *tempR (Live Outer Surface)
    GarageOrPitHot, // CarSetup YAML (LastTempsOMI/IMO) or Pit-Stall Latch
    Carcass,        // *tempCL / *tempCM / *tempCR (Raw Telemetry Channel)
}

impl TireTempSource {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unavailable => "Unavailable",
            Self::Surface => "Outer Surface (*tempL/M/R)",
            Self::GarageOrPitHot => "Pit-Stop / Garage Hot Reading (OMI / IMO)",
            Self::Carcass => "Telemetry Channel (*tempCL/CM/CR)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TireCornerAnalysis {
    pub corner_name: &'static str,
    pub is_left_side: bool,
    pub temp_source: TireTempSource,

    // Canonical telemetry left/middle/right arrays in Fahrenheit (°F)
    pub temp_cl_f: Option<Vec<f32>>,
    pub temp_cm_f: Option<Vec<f32>>,
    pub temp_cr_f: Option<Vec<f32>>,

    // Orientation-resolved arrays in Fahrenheit (°F)
    // LF/LR: Inside = CR, Center = CM, Outside = CL
    // RF/RR: Inside = CL, Center = CM, Outside = CR
    pub inside_temp_f: Option<Vec<f32>>,
    pub center_temp_f: Option<Vec<f32>>,
    pub outside_temp_f: Option<Vec<f32>>,

    // Spread = T_inside - T_outside (°F)
    pub inside_outside_spread_f: Option<Vec<f32>>,
    // Center Bias = T_center - (T_inside + T_outside) / 2.0 (°F)
    pub center_wear_bias_f: Option<Vec<f32>>,

    // Pressure array in PSI (kPa * 0.145038)
    pub pressure_psi: Option<Vec<f32>>,

    // Scalar statistics (°F & PSI)
    pub inside_stats_f: ChannelStats,
    pub center_stats_f: ChannelStats,
    pub outside_stats_f: ChannelStats,
    pub spread_stats_f: ChannelStats,
    pub center_bias_stats_f: ChannelStats,
    pub pressure_stats_psi: ChannelStats,
    pub cold_pressure_psi: f32,
    pub start_pressure_psi: f32,
    pub end_pressure_psi: f32,
    pub peak_pressure_psi: f32,
    pub pressure_build_psi: f32,

    // Tread remaining percentages (0 - 100%)
    pub wear_outside_pct: f32,
    pub wear_center_pct: f32,
    pub wear_inside_pct: f32,
}

impl Default for TireCornerAnalysis {
    fn default() -> Self {
        Self {
            corner_name: "",
            is_left_side: true,
            temp_source: TireTempSource::Unavailable,
            temp_cl_f: None,
            temp_cm_f: None,
            temp_cr_f: None,
            inside_temp_f: None,
            center_temp_f: None,
            outside_temp_f: None,
            inside_outside_spread_f: None,
            center_wear_bias_f: None,
            pressure_psi: None,
            inside_stats_f: ChannelStats::default(),
            center_stats_f: ChannelStats::default(),
            outside_stats_f: ChannelStats::default(),
            spread_stats_f: ChannelStats::default(),
            center_bias_stats_f: ChannelStats::default(),
            pressure_stats_psi: ChannelStats::default(),
            cold_pressure_psi: 0.0,
            start_pressure_psi: 0.0,
            end_pressure_psi: 0.0,
            peak_pressure_psi: 0.0,
            pressure_build_psi: 0.0,
            wear_outside_pct: 100.0,
            wear_center_pct: 100.0,
            wear_inside_pct: 100.0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LapTireAnalysis {
    pub lf: TireCornerAnalysis,
    pub rf: TireCornerAnalysis,
    pub lr: TireCornerAnalysis,
    pub rr: TireCornerAnalysis,
    pub has_temp_data: bool,
    pub has_pressure_data: bool,
}

/// Complete Processed Dataset for a Single Lap
#[derive(Clone, Debug)]
pub struct LapAnalysis {
    pub descriptor: LapDescriptor,
    pub tick_rate: i32,
    pub elapsed_time_s: Vec<f32>,
    pub lap_dist_pct: Vec<f32>,
    pub lap_dist_m: Vec<f32>,
    pub inputs: DriverInputsAnalysis,
    pub suspension: SuspensionAnalysis,
    pub shocks: ShockAnalysis,
    pub tires: LapTireAnalysis,
}

/// Distance-Normalized Lap Trace for 1-to-1 X-Axis Alignment
#[derive(Clone, Debug)]
pub struct NormalizedLapTrace {
    pub lap_number: i32,
    pub duration_secs: f32,
    pub grid_dist_pct: Vec<f32>, // Uniform 0.0 ..= 1.0
    pub grid_dist_m: Vec<f32>,
    pub elapsed_time_s: Vec<f32>,
    pub speed_mph: Option<Vec<f32>>,
    pub speed_kph: Option<Vec<f32>>,
    pub throttle_pct: Option<Vec<f32>>,
    pub brake_pct: Option<Vec<f32>>,
    pub steering_deg: Option<Vec<f32>>,
    pub rpm: Option<Vec<f32>>,
    pub gear: Option<Vec<f32>>,
    pub lf_rh_mm: Option<Vec<f32>>,
    pub rf_rh_mm: Option<Vec<f32>>,
    pub lr_rh_mm: Option<Vec<f32>>,
    pub rr_rh_mm: Option<Vec<f32>>,
    pub cfsr_rh_mm: Option<Vec<f32>>,
    pub rake_mm: Option<Vec<f32>>,
    pub side_rake_mm: Option<Vec<f32>>,
    pub front_split_mm: Option<Vec<f32>>,
    pub rear_split_mm: Option<Vec<f32>>,
}

/// Side-by-Side Lap Comparison (Lap A Baseline vs. Lap B Target)
#[derive(Clone, Debug)]
pub struct LapComparisonReport {
    pub lap_a: NormalizedLapTrace,
    pub lap_b: NormalizedLapTrace,
    pub grid_dist_pct: Vec<f32>,
    pub grid_dist_m: Vec<f32>,
    /// Elapsed time delta (Lap B - Lap A) at each distance point:
    /// Negative = Lap B is faster/ahead of Lap A; Positive = Lap B is slower/behind Lap A
    pub time_delta_s: Vec<f32>,
    pub speed_delta_mph: Option<Vec<f32>>,
    pub rake_delta_mm: Option<Vec<f32>>,
    pub side_rake_delta_mm: Option<Vec<f32>>,
}

/// Full Parsed .IBT Session Container
#[derive(Clone, Debug)]
pub struct IbtSession {
    pub metadata: IbtSessionMetadata,
    pub channels: Vec<IbtChannelInfo>,
    pub laps: Vec<LapDescriptor>,
    pub analyzed_laps: Vec<LapAnalysis>,
    pub fastest_lap_idx: Option<usize>,
    pub garage_tires: Option<crate::StintTireSet>,
}

// ============================================================================
// 4. CORE BINARY .IBT INGESTION & PARSING ENGINE
// ============================================================================

fn read_i32_le(bytes: &[u8], offset: usize) -> Option<i32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_i64_le(bytes: &[u8], offset: usize) -> Option<i64> {
    let slice = bytes.get(offset..offset + 8)?;
    Some(i64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Option<f32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_f64_le(bytes: &[u8], offset: usize) -> Option<f64> {
    let slice = bytes.get(offset..offset + 8)?;
    Some(f64::from_le_bytes([
        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6], slice[7],
    ]))
}

fn decode_cstr_latin1(bytes: &[u8]) -> String {
    let valid_len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    if valid_len == 0 {
        return String::new();
    }
    ISO_8859_1
        .decode(&bytes[..valid_len], DecoderTrap::Replace)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Parses an `.ibt` file from disk into a complete `IbtSession`.
pub fn parse_ibt_file(path: &Path) -> Result<IbtSession, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read .ibt file '{}': {}", path.display(), e))?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("telemetry.ibt")
        .to_string();
    parse_ibt_bytes(&bytes, &file_name)
}

/// Parses raw `.ibt` binary bytes into a complete `IbtSession`.
pub fn parse_ibt_bytes(bytes: &[u8], file_name: &str) -> Result<IbtSession, String> {
    if bytes.len() < IRSDK_HEADER_SIZE + IRSDK_DISK_SUB_HEADER_SIZE {
        return Err(format!(
            "File '{}' is too small ({} bytes) to be a valid iRacing .ibt telemetry file.",
            file_name,
            bytes.len()
        ));
    }

    let ver = read_i32_le(bytes, 0).unwrap_or(0);
    let _status = read_i32_le(bytes, 4).unwrap_or(0);
    let tick_rate = read_i32_le(bytes, 8).unwrap_or(60).max(1);
    let _session_info_update = read_i32_le(bytes, 12).unwrap_or(0);
    let session_info_len = read_i32_le(bytes, 16).unwrap_or(0).max(0) as usize;
    let session_info_offset = read_i32_le(bytes, 20).unwrap_or(0).max(0) as usize;
    let num_vars = read_i32_le(bytes, 24).unwrap_or(0).max(0) as usize;
    let var_header_offset = read_i32_le(bytes, 28).unwrap_or(0).max(0) as usize;
    let _num_buf = read_i32_le(bytes, 32).unwrap_or(1);
    let buf_len = read_i32_le(bytes, 36).unwrap_or(0).max(0) as usize;

    // First varBuf is at offset 48 (tick_count at 48, buf_offset at 52)
    let buf_offset = read_i32_le(bytes, 52).unwrap_or(0).max(0) as usize;

    if ver <= 0 || ver > 10 {
        return Err(format!("Unsupported iRacing .ibt header version: {}", ver));
    }
    if buf_len == 0 || buf_offset == 0 || buf_offset > bytes.len() {
        return Err("Invalid .ibt telemetry buffer offset or stride length.".to_string());
    }

    // Disk sub-header at offset 112 (0x70)
    let session_start_date = read_i64_le(bytes, 112).unwrap_or(0);
    let session_start_time = read_f64_le(bytes, 120).unwrap_or(0.0);
    let session_end_time = read_f64_le(bytes, 128).unwrap_or(0.0);
    let session_lap_count = read_i32_le(bytes, 136).unwrap_or(0);
    let session_record_count = read_i32_le(bytes, 140).unwrap_or(0).max(0) as usize;

    let max_possible_ticks = (bytes.len().saturating_sub(buf_offset)) / buf_len;
    let total_ticks = if session_record_count > 0 && session_record_count <= max_possible_ticks {
        session_record_count
    } else {
        max_possible_ticks
    };

    // Parse YAML Session Info string
    let yaml_str = if session_info_len > 0
        && session_info_offset > 0
        && session_info_offset < bytes.len()
    {
        let end = (session_info_offset + session_info_len).min(bytes.len());
        decode_cstr_latin1(&bytes[session_info_offset..end])
    } else {
        String::new()
    };

    // Parse variable headers (144 bytes each)
    let mut channels = Vec::with_capacity(num_vars);
    let mut channel_map: HashMap<String, IbtChannelInfo> = HashMap::with_capacity(num_vars);

    for i in 0..num_vars {
        let base = var_header_offset + i * IRSDK_VAR_HEADER_SIZE;
        if base + IRSDK_VAR_HEADER_SIZE > bytes.len() {
            break;
        }
        let raw_type = read_i32_le(bytes, base).unwrap_or(4);
        let var_type = match IbtVarType::from_i32(raw_type) {
            Some(vt) => vt,
            None => continue,
        };
        let offset = read_i32_le(bytes, base + 4).unwrap_or(0).max(0) as usize;
        let count = read_i32_le(bytes, base + 8).unwrap_or(1).max(1) as usize;
        let name = decode_cstr_latin1(&bytes[base + 16..base + 48]);
        let desc = decode_cstr_latin1(&bytes[base + 48..base + 112]);
        let unit = decode_cstr_latin1(&bytes[base + 112..base + 144]);

        if !name.is_empty() && offset + var_type.byte_size() <= buf_len {
            let info = IbtChannelInfo {
                name: name.clone(),
                desc,
                unit,
                var_type,
                offset,
                count,
            };
            channel_map.insert(name, info.clone());
            channels.push(info);
        }
    }

    // Parse metadata from YAML + Disk Sub-Header
    let mut metadata = parse_yaml_metadata(&yaml_str);
    metadata.file_name = file_name.to_string();
    metadata.file_size_bytes = bytes.len();
    metadata.tick_rate = tick_rate;
    metadata.total_ticks = total_ticks;
    metadata.total_laps_header = session_lap_count;
    metadata.channel_count = channels.len();
    if metadata.session_date.is_empty() || metadata.session_date == "—" {
        if session_start_date > 0 {
            metadata.session_date = format_unix_timestamp(session_start_date);
        } else {
            metadata.session_date = "—".to_string();
        }
    }
    if session_end_time > session_start_time {
        metadata.session_duration_secs = session_end_time - session_start_time;
    } else {
        metadata.session_duration_secs = total_ticks as f64 / tick_rate as f64;
    }

    let tick_buffer = &bytes[buf_offset..buf_offset + total_ticks * buf_len];

    // Extract core indexing channels across all ticks
    let lap_channel = extract_channel_i32(tick_buffer, buf_len, total_ticks, channel_map.get("Lap"))
        .unwrap_or_else(|| vec![1; total_ticks]);

    let session_time_channel = extract_channel_f64(
        tick_buffer,
        buf_len,
        total_ticks,
        channel_map.get("SessionTime"),
    )
    .unwrap_or_else(|| {
        (0..total_ticks)
            .map(|i| i as f64 / tick_rate as f64)
            .collect()
    });

    let lap_dist_pct_channel = extract_channel_f32(
        tick_buffer,
        buf_len,
        total_ticks,
        channel_map.get("LapDistPct"),
    )
    .unwrap_or_else(|| {
        if total_ticks <= 1 {
            vec![0.0; total_ticks]
        } else {
            (0..total_ticks)
                .map(|i| (i as f32) / ((total_ticks - 1) as f32))
                .collect()
        }
    });

    let lap_dist_m_channel = extract_channel_f32(
        tick_buffer,
        buf_len,
        total_ticks,
        channel_map.get("LapDist"),
    )
    .unwrap_or_else(|| {
        let track_len_m = parse_track_length_meters(&metadata.track_length).unwrap_or(4000.0);
        lap_dist_pct_channel
            .iter()
            .map(|&pct| pct * track_len_m)
            .collect()
    });

    let on_pit_road_channel = extract_channel_i32(
        tick_buffer,
        buf_len,
        total_ticks,
        channel_map.get("OnPitRoad"),
    );

    let player_surface_channel = extract_channel_i32(
        tick_buffer,
        buf_len,
        total_ticks,
        channel_map.get("PlayerTrackSurface"),
    );

    // Slice laps
    let mut laps = slice_laps(
        &lap_channel,
        &session_time_channel,
        &lap_dist_pct_channel,
        on_pit_road_channel.as_deref(),
        player_surface_channel.as_deref(),
        tick_rate,
    );

    // Determine fastest valid flying lap (or fallback to fastest complete lap)
    let fastest_lap_idx = laps
        .iter()
        .filter(|l| l.is_valid_flying && l.duration_secs > 1.0)
        .min_by(|a, b| {
            a.duration_secs
                .partial_cmp(&b.duration_secs)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|l| l.lap_index)
        .or_else(|| {
            laps.iter()
                .filter(|l| l.is_complete && !l.is_out_lap && !l.is_in_lap && l.duration_secs > 1.0)
                .min_by(|a, b| {
                    a.duration_secs
                        .partial_cmp(&b.duration_secs)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|l| l.lap_index)
        })
        .or_else(|| {
            laps.iter()
                .filter(|l| l.duration_secs > 1.0)
                .min_by(|a, b| {
                    a.duration_secs
                        .partial_cmp(&b.duration_secs)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|l| l.lap_index)
        });

    if let Some(pb_idx) = fastest_lap_idx {
        if let Some(pb_lap) = laps.get_mut(pb_idx) {
            pb_lap.is_personal_best = true;
        }
    }

    // Extract full-session channels once (contiguous columnar vectors) and slice per lap
    let full_speed = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("Speed"));
    let full_throttle = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("Throttle"));
    let full_brake = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("Brake"));
    let full_clutch = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("Clutch"));
    let full_steer = extract_channel_f32(
        tick_buffer,
        buf_len,
        total_ticks,
        channel_map.get("SteeringWheelAngle"),
    );
    let full_gear = extract_channel_i32(tick_buffer, buf_len, total_ticks, channel_map.get("Gear"));
    let full_rpm = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RPM"));
    let full_lat_accel = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LatAccel"));
    let full_long_accel = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LongAccel"));

    // Ride height channels
    let full_lf_rh = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFrideHeight"));
    let full_rf_rh = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFrideHeight"));
    let full_lr_rh = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRrideHeight"));
    let full_rr_rh = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRrideHeight"));
    let full_cfsr_rh = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("CFSRrideHeight"));

    // Shock deflection & native velocity channels
    let full_lf_defl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFshockDefl"))
        .or_else(|| extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LSHshockDefl")));
    let full_rf_defl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFshockDefl"))
        .or_else(|| extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RSHshockDefl")));
    let full_lr_defl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRshockDefl"));
    let full_rr_defl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRshockDefl"));

    let full_lf_vel = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFshockVel"))
        .or_else(|| extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LSHshockVel")));
    let full_rf_vel = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFshockVel"))
        .or_else(|| extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RSHshockVel")));
    let full_lr_vel = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRshockVel"));
    let full_rr_vel = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRshockVel"));

    // Tire temperature channels (Prioritize Outer Surface *tempL/M/R, then *tempCL/CM/CR)
    let (lf_temp_src, full_lf_cl, full_lf_cm, full_lf_cr) =
        extract_corner_temps(tick_buffer, buf_len, total_ticks, &channel_map, "LF");
    let (rf_temp_src, full_rf_cl, full_rf_cm, full_rf_cr) =
        extract_corner_temps(tick_buffer, buf_len, total_ticks, &channel_map, "RF");
    let (lr_temp_src, full_lr_cl, full_lr_cm, full_lr_cr) =
        extract_corner_temps(tick_buffer, buf_len, total_ticks, &channel_map, "LR");
    let (rr_temp_src, full_rr_cl, full_rr_cm, full_rr_cr) =
        extract_corner_temps(tick_buffer, buf_len, total_ticks, &channel_map, "RR");

    // Tire pressure & cold pressure channels
    let full_lf_press = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFpressure"));
    let full_rf_press = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFpressure"));
    let full_lr_press = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRpressure"));
    let full_rr_press = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRpressure"));

    let full_lf_cold_p = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFcoldPressure"));
    let full_rf_cold_p = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFcoldPressure"));
    let full_lr_cold_p = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRcoldPressure"));
    let full_rr_cold_p = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRcoldPressure"));

    // Tire tread wear channels (*wearL / *wearM / *wearR)
    let full_lf_wl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFwearL"));
    let full_lf_wm = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFwearM"));
    let full_lf_wr = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LFwearR"));
    let full_rf_wl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFwearL"));
    let full_rf_wm = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFwearM"));
    let full_rf_wr = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RFwearR"));
    let full_lr_wl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRwearL"));
    let full_lr_wm = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRwearM"));
    let full_lr_wr = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("LRwearR"));
    let full_rr_wl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRwearL"));
    let full_rr_wm = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRwearM"));
    let full_rr_wr = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get("RRwearR"));

    // Parse CarSetup YAML hot tire readings & cold pressures + merge with any pit-stall latch in the .ibt file
    let (yaml_garage_tires, yaml_cold_press_kpa) = parse_yaml_garage_tires_and_cold(&yaml_str);
    let pit_latch_tires = extract_session_pit_latch_tires(
        total_ticks,
        (&full_lf_cl, &full_lf_cm, &full_lf_cr, &full_lf_wl, &full_lf_wm, &full_lf_wr, &full_lf_press),
        (&full_rf_cl, &full_rf_cm, &full_rf_cr, &full_rf_wl, &full_rf_wm, &full_rf_wr, &full_rf_press),
        (&full_lr_cl, &full_lr_cm, &full_lr_cr, &full_lr_wl, &full_lr_wm, &full_lr_wr, &full_lr_press),
        (&full_rr_cl, &full_rr_cm, &full_rr_cr, &full_rr_wl, &full_rr_wm, &full_rr_wr, &full_rr_press),
    );
    let garage_tires = merge_garage_and_pit_tires(yaml_garage_tires, pit_latch_tires);

    let cold_press_kpa = [
        full_lf_cold_p
            .as_ref()
            .and_then(|v| v.first().copied())
            .filter(|&p| p > 1.0)
            .or_else(|| yaml_cold_press_kpa.map(|a| a[0]))
            .unwrap_or(0.0),
        full_rf_cold_p
            .as_ref()
            .and_then(|v| v.first().copied())
            .filter(|&p| p > 1.0)
            .or_else(|| yaml_cold_press_kpa.map(|a| a[1]))
            .unwrap_or(0.0),
        full_lr_cold_p
            .as_ref()
            .and_then(|v| v.first().copied())
            .filter(|&p| p > 1.0)
            .or_else(|| yaml_cold_press_kpa.map(|a| a[2]))
            .unwrap_or(0.0),
        full_rr_cold_p
            .as_ref()
            .and_then(|v| v.first().copied())
            .filter(|&p| p > 1.0)
            .or_else(|| yaml_cold_press_kpa.map(|a| a[3]))
            .unwrap_or(0.0),
    ];

    // Build analyzed dataset for each sliced lap
    let mut analyzed_laps = Vec::with_capacity(laps.len());
    for desc in &laps {
        let s = desc.start_tick;
        let e = desc.end_tick;
        let t0 = session_time_channel.get(s).copied().unwrap_or(0.0);
        let elapsed_time_s: Vec<f32> = session_time_channel[s..e]
            .iter()
            .map(|&t| (t - t0).max(0.0) as f32)
            .collect();
        let lap_dist_pct = lap_dist_pct_channel[s..e].to_vec();
        let lap_dist_m = lap_dist_m_channel[s..e].to_vec();

        let inputs = compute_driver_inputs(
            slice_opt(&full_speed, s, e),
            slice_opt(&full_throttle, s, e),
            slice_opt(&full_brake, s, e),
            slice_opt(&full_clutch, s, e),
            slice_opt(&full_steer, s, e),
            slice_opt(&full_gear, s, e),
            slice_opt(&full_rpm, s, e),
            slice_opt(&full_lat_accel, s, e),
            slice_opt(&full_long_accel, s, e),
        );

        let suspension = compute_suspension_dynamics(
            slice_opt(&full_lf_rh, s, e),
            slice_opt(&full_rf_rh, s, e),
            slice_opt(&full_lr_rh, s, e),
            slice_opt(&full_rr_rh, s, e),
            slice_opt(&full_cfsr_rh, s, e),
        );

        let shocks = compute_shock_analysis(
            slice_opt(&full_lf_defl, s, e),
            slice_opt(&full_rf_defl, s, e),
            slice_opt(&full_lr_defl, s, e),
            slice_opt(&full_rr_defl, s, e),
            slice_opt(&full_lf_vel, s, e),
            slice_opt(&full_rf_vel, s, e),
            slice_opt(&full_lr_vel, s, e),
            slice_opt(&full_rr_vel, s, e),
            tick_rate as f32,
        );

        let mut tires = compute_tire_analysis(
            (
                lf_temp_src,
                slice_opt(&full_lf_cl, s, e),
                slice_opt(&full_lf_cm, s, e),
                slice_opt(&full_lf_cr, s, e),
                slice_opt(&full_lf_press, s, e),
            ),
            (
                rf_temp_src,
                slice_opt(&full_rf_cl, s, e),
                slice_opt(&full_rf_cm, s, e),
                slice_opt(&full_rf_cr, s, e),
                slice_opt(&full_rf_press, s, e),
            ),
            (
                lr_temp_src,
                slice_opt(&full_lr_cl, s, e),
                slice_opt(&full_lr_cm, s, e),
                slice_opt(&full_lr_cr, s, e),
                slice_opt(&full_lr_press, s, e),
            ),
            (
                rr_temp_src,
                slice_opt(&full_rr_cl, s, e),
                slice_opt(&full_rr_cm, s, e),
                slice_opt(&full_rr_cr, s, e),
                slice_opt(&full_rr_press, s, e),
            ),
        );

        apply_hot_tires_and_wear_to_lap(
            &mut tires,
            garage_tires.as_ref(),
            &cold_press_kpa,
            [
                sample_end_wear(&full_lf_wl, &full_lf_wm, &full_lf_wr, s, e),
                sample_end_wear(&full_rf_wl, &full_rf_wm, &full_rf_wr, s, e),
                sample_end_wear(&full_lr_wl, &full_lr_wm, &full_lr_wr, s, e),
                sample_end_wear(&full_rr_wl, &full_rr_wm, &full_rr_wr, s, e),
            ],
        );

        analyzed_laps.push(LapAnalysis {
            descriptor: desc.clone(),
            tick_rate,
            elapsed_time_s,
            lap_dist_pct,
            lap_dist_m,
            inputs,
            suspension,
            shocks,
            tires,
        });
    }

    Ok(IbtSession {
        metadata,
        channels,
        laps,
        analyzed_laps,
        fastest_lap_idx,
        garage_tires,
    })
}

fn slice_opt<T: Clone>(opt_vec: &Option<Vec<T>>, start: usize, end: usize) -> Option<Vec<T>> {
    opt_vec.as_ref().and_then(|v| {
        if start <= end && end <= v.len() {
            Some(v[start..end].to_vec())
        } else {
            None
        }
    })
}

fn extract_corner_temps(
    tick_buffer: &[u8],
    buf_len: usize,
    total_ticks: usize,
    channel_map: &HashMap<String, IbtChannelInfo>,
    prefix: &str,
) -> (TireTempSource, Option<Vec<f32>>, Option<Vec<f32>>, Option<Vec<f32>>) {
    // 1. Prioritize live outer surface temperature channels (*tempL / *tempM / *tempR) whenever logged
    let l_name = format!("{}tempL", prefix);
    let m_name = format!("{}tempM", prefix);
    let r_name = format!("{}tempR", prefix);

    let l = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get(&l_name));
    let m = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get(&m_name));
    let r = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get(&r_name));

    if let (Some(_lv), Some(mv), Some(_rv)) = (&l, &m, &r) {
        if mv.iter().any(|&v| v > 5.0) {
            return (TireTempSource::Surface, l, m, r);
        }
    }

    // 2. Otherwise extract telemetry *tempCL / *tempCM / *tempCR channels
    let cl_name = format!("{}tempCL", prefix);
    let cm_name = format!("{}tempCM", prefix);
    let cr_name = format!("{}tempCR", prefix);

    let cl = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get(&cl_name));
    let cm = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get(&cm_name));
    let cr = extract_channel_f32(tick_buffer, buf_len, total_ticks, channel_map.get(&cr_name));

    if cl.is_some() && cm.is_some() && cr.is_some() {
        return (TireTempSource::Carcass, cl, cm, cr);
    }

    if l.is_some() && m.is_some() && r.is_some() {
        (TireTempSource::Surface, l, m, r)
    } else {
        (TireTempSource::Unavailable, None, None, None)
    }
}

pub fn extract_channel_f32(
    tick_buffer: &[u8],
    buf_len: usize,
    total_ticks: usize,
    var: Option<&IbtChannelInfo>,
) -> Option<Vec<f32>> {
    let info = var?;
    let offset = info.offset;
    let elem_size = info.var_type.byte_size();
    if offset + elem_size > buf_len {
        return None;
    }

    let mut out = Vec::with_capacity(total_ticks);
    for tick in 0..total_ticks {
        let pos = tick * buf_len + offset;
        let val = match info.var_type {
            IbtVarType::Float => read_f32_le(tick_buffer, pos).unwrap_or(0.0),
            IbtVarType::Double => read_f64_le(tick_buffer, pos).unwrap_or(0.0) as f32,
            IbtVarType::Int => read_i32_le(tick_buffer, pos).unwrap_or(0) as f32,
            IbtVarType::BitField => read_u32_le(tick_buffer, pos).unwrap_or(0) as f32,
            IbtVarType::Bool | IbtVarType::Char => {
                if tick_buffer.get(pos).copied().unwrap_or(0) != 0 {
                    1.0
                } else {
                    0.0
                }
            }
        };
        out.push(if val.is_finite() { val } else { 0.0 });
    }
    Some(out)
}

pub fn extract_channel_i32(
    tick_buffer: &[u8],
    buf_len: usize,
    total_ticks: usize,
    var: Option<&IbtChannelInfo>,
) -> Option<Vec<i32>> {
    let info = var?;
    let offset = info.offset;
    let elem_size = info.var_type.byte_size();
    if offset + elem_size > buf_len {
        return None;
    }

    let mut out = Vec::with_capacity(total_ticks);
    for tick in 0..total_ticks {
        let pos = tick * buf_len + offset;
        let val = match info.var_type {
            IbtVarType::Int => read_i32_le(tick_buffer, pos).unwrap_or(0),
            IbtVarType::BitField => read_u32_le(tick_buffer, pos).unwrap_or(0) as i32,
            IbtVarType::Bool | IbtVarType::Char => {
                if tick_buffer.get(pos).copied().unwrap_or(0) != 0 {
                    1
                } else {
                    0
                }
            }
            IbtVarType::Float => read_f32_le(tick_buffer, pos).unwrap_or(0.0).round() as i32,
            IbtVarType::Double => read_f64_le(tick_buffer, pos).unwrap_or(0.0).round() as i32,
        };
        out.push(val);
    }
    Some(out)
}

pub fn extract_channel_f64(
    tick_buffer: &[u8],
    buf_len: usize,
    total_ticks: usize,
    var: Option<&IbtChannelInfo>,
) -> Option<Vec<f64>> {
    let info = var?;
    let offset = info.offset;
    let elem_size = info.var_type.byte_size();
    if offset + elem_size > buf_len {
        return None;
    }

    let mut out = Vec::with_capacity(total_ticks);
    for tick in 0..total_ticks {
        let pos = tick * buf_len + offset;
        let val = match info.var_type {
            IbtVarType::Double => read_f64_le(tick_buffer, pos).unwrap_or(0.0),
            IbtVarType::Float => read_f32_le(tick_buffer, pos).unwrap_or(0.0) as f64,
            IbtVarType::Int => read_i32_le(tick_buffer, pos).unwrap_or(0) as f64,
            IbtVarType::BitField => read_u32_le(tick_buffer, pos).unwrap_or(0) as f64,
            IbtVarType::Bool | IbtVarType::Char => {
                if tick_buffer.get(pos).copied().unwrap_or(0) != 0 {
                    1.0
                } else {
                    0.0
                }
            }
        };
        out.push(if val.is_finite() { val } else { 0.0 });
    }
    Some(out)
}

// ============================================================================
// 5. YAML METADATA EXTRACTION
// ============================================================================

fn parse_yaml_metadata(yaml_str: &str) -> IbtSessionMetadata {
    let mut meta = IbtSessionMetadata {
        track_display_name: "Unknown Track".to_string(),
        track_config_name: String::new(),
        track_length: "—".to_string(),
        track_type: String::new(),
        car_name: "Unknown Car".to_string(),
        car_number: "—".to_string(),
        driver_name: "Driver".to_string(),
        irating: "—".to_string(),
        setup_name: "Baseline".to_string(),
        session_date: "—".to_string(),
        session_type: "OFFLINE TELEMETRY".to_string(),
        weather_type: "—".to_string(),
        skies: "—".to_string(),
        air_temp: "—".to_string(),
        track_temp: "—".to_string(),
        wind_speed: "—".to_string(),
        wind_dir: "—".to_string(),
        relative_humidity: "—".to_string(),
        ..Default::default()
    };

    if yaml_str.trim().is_empty() {
        return meta;
    }

    if let Ok(root) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
        let weekend = &root["WeekendInfo"];
        if let Some(t) = weekend["TrackDisplayName"]
            .as_str()
            .or_else(|| weekend["TrackName"].as_str())
        {
            if !t.trim().is_empty() {
                meta.track_display_name = t.trim().to_string();
            }
        }
        if let Some(cfg) = weekend["TrackConfigName"].as_str() {
            meta.track_config_name = cfg.trim().to_string();
        }
        if let Some(len) = weekend["TrackLength"].as_str() {
            if !len.trim().is_empty() {
                meta.track_length = len.trim().to_string();
            }
        }
        if let Some(tt) = weekend["TrackType"].as_str() {
            meta.track_type = tt.trim().to_string();
        }
        if let Some(wt) = weekend["TrackWeatherType"].as_str() {
            if !wt.trim().is_empty() {
                meta.weather_type = wt.trim().to_string();
            }
        }
        if let Some(sk) = weekend["TrackSkies"].as_str() {
            if !sk.trim().is_empty() {
                meta.skies = sk.trim().to_string();
            }
        }
        if let Some(at) = weekend["TrackAirTemp"].as_str() {
            if !at.trim().is_empty() {
                meta.air_temp = at.trim().to_string();
            }
        }
        if let Some(st) = weekend["TrackSurfaceTemp"].as_str() {
            if !st.trim().is_empty() {
                meta.track_temp = st.trim().to_string();
            }
        }
        if let Some(wv) = weekend["TrackWindVel"].as_str() {
            if !wv.trim().is_empty() {
                meta.wind_speed = wv.trim().to_string();
            }
        }
        if let Some(wd) = weekend["WeekendOptions"]["WindDirection"]
            .as_str()
            .or_else(|| weekend["TrackWindDir"].as_str())
        {
            if !wd.trim().is_empty() {
                meta.wind_dir = wd.trim().to_string();
            }
        }
        if let Some(rh) = weekend["WeekendOptions"]["RelativeHumidity"]
            .as_str()
            .or_else(|| weekend["TrackRelativeHumidity"].as_str())
        {
            if !rh.trim().is_empty() {
                meta.relative_humidity = rh.trim().to_string();
            }
        }
        if let Some(dt) = weekend["WeekendOptions"]["Date"].as_str() {
            if !dt.trim().is_empty() {
                meta.session_date = dt.trim().to_string();
            }
        }

        if let Some(sessions) = root["SessionInfo"]["Sessions"].as_sequence() {
            if let Some(last_sess) = sessions.last() {
                if let Some(stype) = last_sess["SessionType"].as_str() {
                    if !stype.trim().is_empty() {
                        meta.session_type = stype.trim().to_uppercase();
                    }
                }
            }
        }

        let driver_info = &root["DriverInfo"];
        let player_idx = driver_info["DriverCarIdx"].as_u64().unwrap_or(0) as usize;
        if let Some(raw_setup) = driver_info["DriverSetupName"].as_str() {
            let cleaned = crate::clean_setup_name(raw_setup);
            if !cleaned.is_empty() {
                meta.setup_name = cleaned;
            }
        }

        if let Some(drivers) = driver_info["Drivers"].as_sequence() {
            if let Some(d) = drivers
                .iter()
                .find(|drv| drv["CarIdx"].as_u64().unwrap_or(999) as usize == player_idx)
                .or_else(|| drivers.first())
            {
                if let Some(uname) = d["UserName"].as_str() {
                    if !uname.trim().is_empty() {
                        meta.driver_name = uname.trim().to_string();
                    }
                }
                meta.car_number = match &d["CarNumber"] {
                    serde_yaml::Value::String(s) => s.clone(),
                    serde_yaml::Value::Number(n) => n.to_string(),
                    _ => "—".to_string(),
                };
                meta.irating = match &d["IRating"] {
                    serde_yaml::Value::Number(n) => n.to_string(),
                    serde_yaml::Value::String(s) => s.clone(),
                    _ => "—".to_string(),
                };
                if let Some(cname) = d["CarScreenName"]
                    .as_str()
                    .or_else(|| d["CarScreenNameShort"].as_str())
                    .or_else(|| d["CarPath"].as_str())
                {
                    if !cname.trim().is_empty() {
                        meta.car_name = cname.trim().to_string();
                    }
                }
            }
        }
    } else {
        // Fallback line-by-line scan if YAML has non-standard setup strings
        for line in yaml_str.lines() {
            let trimmed = line.trim();
            if let Some(val) = trimmed.strip_prefix("TrackDisplayName:") {
                meta.track_display_name = val.trim().trim_matches('"').to_string();
            } else if let Some(val) = trimmed.strip_prefix("TrackLength:") {
                meta.track_length = val.trim().trim_matches('"').to_string();
            } else if let Some(val) = trimmed.strip_prefix("DriverSetupName:") {
                let c = crate::clean_setup_name(val.trim().trim_matches('"'));
                if !c.is_empty() {
                    meta.setup_name = c;
                }
            } else if let Some(val) = trimmed.strip_prefix("TrackAirTemp:") {
                meta.air_temp = val.trim().trim_matches('"').to_string();
            } else if let Some(val) = trimmed.strip_prefix("TrackSurfaceTemp:") {
                meta.track_temp = val.trim().trim_matches('"').to_string();
            }
        }
    }

    meta
}

fn parse_track_length_meters(raw: &str) -> Option<f32> {
    let clean = raw.trim().to_lowercase();
    let mut num_end = 0;
    for (i, c) in clean.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + c.len_utf8();
        } else if num_end > 0 {
            break;
        }
    }
    if num_end == 0 {
        return None;
    }
    let val = clean[..num_end].parse::<f32>().ok()?;
    let unit = clean[num_end..].trim();
    if unit.starts_with("km") {
        Some(val * 1000.0)
    } else if unit.starts_with("mi") {
        Some(val * 1609.344)
    } else {
        Some(val)
    }
}

// ============================================================================
// 6. LAP SLICING, BOUNDARY DETECTION & X-AXIS NORMALIZATION
// ============================================================================

pub fn slice_laps(
    lap_channel: &[i32],
    session_time: &[f64],
    lap_dist_pct: &[f32],
    on_pit_road: Option<&[i32]>,
    player_surface: Option<&[i32]>,
    tick_rate: i32,
) -> Vec<LapDescriptor> {
    let n = lap_channel.len();
    if n == 0 {
        return Vec::new();
    }

    // Identify start..end index boundaries whenever `Lap` changes
    let mut raw_ranges: Vec<(i32, usize, usize)> = Vec::new();
    let mut seg_start = 0usize;
    let mut current_lap = lap_channel[0];

    for i in 1..n {
        if lap_channel[i] != current_lap {
            raw_ranges.push((current_lap, seg_start, i));
            seg_start = i;
            current_lap = lap_channel[i];
        }
    }
    raw_ranges.push((current_lap, seg_start, n));

    let dt = 1.0 / (tick_rate.max(1) as f32);
    let mut descriptors = Vec::with_capacity(raw_ranges.len());

    for (idx, (lap_num, start, end)) in raw_ranges.into_iter().enumerate() {
        let tick_count = end.saturating_sub(start);
        if tick_count == 0 {
            continue;
        }

        let time_delta = if end > start + 1 {
            let t_start = session_time.get(start).copied().unwrap_or(0.0);
            let t_end = session_time.get(end - 1).copied().unwrap_or(t_start);
            let diff = (t_end - t_start) as f32 + dt;
            if diff > 0.0 && diff.is_finite() {
                diff
            } else {
                tick_count as f32 * dt
            }
        } else {
            tick_count as f32 * dt
        };

        // Distance coverage on this lap segment
        // Skip first/last 1-2 transition ticks when checking min/max LapDistPct to avoid S/F wrap artifacts
        let dist_slice = &lap_dist_pct[start..end];
        let inner_slice = if dist_slice.len() > 6 {
            &dist_slice[2..dist_slice.len() - 2]
        } else {
            dist_slice
        };
        let min_dist_pct = inner_slice
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min)
            .clamp(0.0, 1.0);
        let max_dist_pct = inner_slice
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max)
            .clamp(0.0, 1.0);

        // Pit road detection:
        // Out-lap: starts on pit road or exits pit road during the first half of the lap
        // In-lap: ends on pit road or enters pit road during the second half of the lap
        let mut is_out_lap = false;
        let mut is_in_lap = false;
        if let Some(pit_slice) = on_pit_road.and_then(|p| p.get(start..end)) {
            if !pit_slice.is_empty() {
                let first_window = 30.min(pit_slice.len());
                let first_ticks_pit = pit_slice[..first_window].iter().filter(|&&v| v != 0).count() >= 2;
                let last_ticks_pit = pit_slice[pit_slice.len() - first_window..]
                    .iter()
                    .filter(|&&v| v != 0)
                    .count()
                    >= 2;
                let exited_pits = pit_slice.windows(2).any(|w| w[0] != 0 && w[1] == 0);
                let entered_pits = pit_slice.windows(2).any(|w| w[0] == 0 && w[1] != 0);

                is_out_lap = first_ticks_pit || (exited_pits && !entered_pits);
                is_in_lap = last_ticks_pit || (entered_pits && !exited_pits);
            }
        }

        // iRacing raw binary `irsdk_TrkLoc` enum (`PlayerTrackSurface`):
        // -1 = NotInWorld, 0 = OffTrack, 1 = InPitStall, 2 = ApproachingPits, 3 = OnTrack
        let mut has_off_track = false;
        let mut has_not_in_world = false;
        if let Some(surf_slice) = player_surface.and_then(|s| s.get(start..end)) {
            if !surf_slice.is_empty() {
                // Only flag InPitStall (1) at start/end of lap as out-lap / in-lap
                if surf_slice[0] == 1 {
                    is_out_lap = true;
                }
                if *surf_slice.last().unwrap_or(&3) == 1 {
                    is_in_lap = true;
                }
                // Flag NotInWorld (-1) if vehicle left the world for > 15 ticks
                let not_in_world_ticks = surf_slice.iter().filter(|&&s| s < 0).count();
                has_not_in_world = not_in_world_ticks > 15;

                // Flag OffTrack (0) only if sustained for > 30 consecutive ticks (0.5s at 60Hz)
                // while not on pit road, so brief apron touches on ovals don't invalidate flying laps
                let mut consecutive_off = 0usize;
                for (local_i, &s_val) in surf_slice.iter().enumerate() {
                    let on_pit_here = on_pit_road
                        .and_then(|p| p.get(start + local_i))
                        .copied()
                        .unwrap_or(0)
                        != 0;
                    if s_val == 0 && !on_pit_here {
                        consecutive_off += 1;
                        if consecutive_off > 30 {
                            has_off_track = true;
                            break;
                        }
                    } else {
                        consecutive_off = 0;
                    }
                }
            }
        }

        let span = max_dist_pct - min_dist_pct;
        let is_complete = !has_not_in_world
            && tick_count >= 10
            && min_dist_pct <= 0.15
            && max_dist_pct >= 0.85
            && span >= 0.75;

        let is_valid_flying = is_complete && !is_out_lap && !is_in_lap && !has_off_track && lap_num > 0;

        descriptors.push(LapDescriptor {
            lap_index: descriptors.len(),
            lap_number: lap_num,
            start_tick: start,
            end_tick: end,
            tick_count,
            duration_secs: time_delta,
            is_out_lap,
            is_in_lap,
            has_off_track,
            is_complete,
            is_valid_flying,
            is_personal_best: false,
            min_dist_pct,
            max_dist_pct,
        });
        let _ = idx;
    }

    descriptors
}

/// Resamples a 1D telemetry channel from `src_dist` onto a uniform `target_grid`,
/// safely handling stagnant distance counters (`Δdist == 0`) with zero division-by-zero risk.
pub fn interpolate_onto_grid(src_dist: &[f32], src_vals: &[f32], target_grid: &[f32]) -> Vec<f32> {
    let n = src_dist.len().min(src_vals.len());
    if n == 0 || target_grid.is_empty() {
        return vec![0.0; target_grid.len()];
    }
    if n == 1 {
        let v = if src_vals[0].is_finite() { src_vals[0] } else { 0.0 };
        return vec![v; target_grid.len()];
    }

    // Build monotonically non-decreasing distance array to prevent S/F wrap glitches
    // or stationary jitter from inverting the interpolation bracket
    let mut clean_dist = Vec::with_capacity(n);
    let mut running_max = src_dist[0].clamp(0.0, 1.0);
    // If the very first sample is from the end of the previous lap (e.g. 0.998 right at S/F), clamp it to 0.0
    if running_max > 0.90 && n > 4 && src_dist[n / 2] < 0.75 {
        running_max = 0.0;
    }
    clean_dist.push(running_max);

    for i in 1..n {
        let mut d = if src_dist[i].is_finite() {
            src_dist[i].clamp(0.0, 1.0)
        } else {
            running_max
        };
        // Guard against S/F wrap at the very start or end of a lap slice
        if i < 5 && d > 0.90 && src_dist[n / 2] < 0.75 {
            d = 0.0;
        }
        if d < running_max {
            d = running_max;
        } else {
            running_max = d;
        }
        clean_dist.push(d);
    }

    let mut out = Vec::with_capacity(target_grid.len());
    let mut cursor = 0usize;

    for &target_x in target_grid {
        // Advance cursor while next point is <= target_x
        while cursor + 1 < n && clean_dist[cursor + 1] <= target_x {
            cursor += 1;
        }

        if cursor + 1 >= n {
            let v = src_vals[n - 1];
            out.push(if v.is_finite() { v } else { 0.0 });
            continue;
        }

        let x0 = clean_dist[cursor];
        let x1 = clean_dist[cursor + 1];
        let y0 = if src_vals[cursor].is_finite() { src_vals[cursor] } else { 0.0 };
        let y1 = if src_vals[cursor + 1].is_finite() { src_vals[cursor + 1] } else { y0 };

        let denom = x1 - x0;
        let interpolated = if denom.abs() > 1e-7 {
            let alpha = ((target_x - x0) / denom).clamp(0.0, 1.0);
            y0 + alpha * (y1 - y0)
        } else {
            // Stagnant distance counter: no division by zero!
            y0
        };

        out.push(if interpolated.is_finite() { interpolated } else { 0.0 });
    }

    out
}

/// Normalizes all channels of a `LapAnalysis` onto a uniform `LapDistPct` grid (`0.0..=1.0`).
pub fn normalize_lap(lap: &LapAnalysis, num_points: usize) -> NormalizedLapTrace {
    let pts = num_points.max(2);
    let grid_dist_pct: Vec<f32> = (0..pts)
        .map(|i| (i as f32) / ((pts - 1) as f32))
        .collect();

    let src_x = &lap.lap_dist_pct;
    let grid_dist_m = interpolate_onto_grid(src_x, &lap.lap_dist_m, &grid_dist_pct);
    let elapsed_time_s = interpolate_onto_grid(src_x, &lap.elapsed_time_s, &grid_dist_pct);

    let norm_opt = |opt: &Option<Vec<f32>>| -> Option<Vec<f32>> {
        opt.as_ref()
            .map(|vals| interpolate_onto_grid(src_x, vals, &grid_dist_pct))
    };

    let gear = lap.inputs.gear.as_ref().map(|g| {
        let g_f32: Vec<f32> = g.iter().map(|&v| v as f32).collect();
        interpolate_onto_grid(src_x, &g_f32, &grid_dist_pct)
    });

    let speed_mph = norm_opt(&lap.inputs.speed_mph);
    let speed_kph = norm_opt(&lap.inputs.speed_kph);
    let throttle_pct = norm_opt(&lap.inputs.throttle_pct);
    let brake_pct = norm_opt(&lap.inputs.brake_pct);
    let steering_deg = norm_opt(&lap.inputs.steering_deg);
    let rpm = norm_opt(&lap.inputs.rpm);
    let lf_rh_mm = norm_opt(&lap.suspension.lf_rh_mm);
    let rf_rh_mm = norm_opt(&lap.suspension.rf_rh_mm);
    let lr_rh_mm = norm_opt(&lap.suspension.lr_rh_mm);
    let rr_rh_mm = norm_opt(&lap.suspension.rr_rh_mm);
    let cfsr_rh_mm = norm_opt(&lap.suspension.cfsr_rh_mm);
    let rake_mm = norm_opt(&lap.suspension.rake_mm);
    let side_rake_mm = norm_opt(&lap.suspension.side_rake_mm);
    let front_split_mm = norm_opt(&lap.suspension.front_split_mm);
    let rear_split_mm = norm_opt(&lap.suspension.rear_split_mm);

    NormalizedLapTrace {
        lap_number: lap.descriptor.lap_number,
        duration_secs: lap.descriptor.duration_secs,
        grid_dist_pct,
        grid_dist_m,
        elapsed_time_s,
        speed_mph,
        speed_kph,
        throttle_pct,
        brake_pct,
        steering_deg,
        rpm,
        gear,
        lf_rh_mm,
        rf_rh_mm,
        lr_rh_mm,
        rr_rh_mm,
        cfsr_rh_mm,
        rake_mm,
        side_rake_mm,
        front_split_mm,
        rear_split_mm,
    }
}

/// Compares two laps (Lap A = Baseline, Lap B = Comparison) across a uniform distance grid.
pub fn compare_laps(lap_a: &LapAnalysis, lap_b: &LapAnalysis, num_points: usize) -> LapComparisonReport {
    let norm_a = normalize_lap(lap_a, num_points);
    let norm_b = normalize_lap(lap_b, num_points);

    let time_delta_s: Vec<f32> = norm_b
        .elapsed_time_s
        .iter()
        .zip(norm_a.elapsed_time_s.iter())
        .map(|(&tb, &ta)| tb - ta)
        .collect();

    let diff_opt = |opt_b: &Option<Vec<f32>>, opt_a: &Option<Vec<f32>>| -> Option<Vec<f32>> {
        match (opt_b, opt_a) {
            (Some(vb), Some(va)) => Some(
                vb.iter()
                    .zip(va.iter())
                    .map(|(&b, &a)| b - a)
                    .collect(),
            ),
            _ => None,
        }
    };

    let speed_delta_mph = diff_opt(&norm_b.speed_mph, &norm_a.speed_mph);
    let rake_delta_mm = diff_opt(&norm_b.rake_mm, &norm_a.rake_mm);
    let side_rake_delta_mm = diff_opt(&norm_b.side_rake_mm, &norm_a.side_rake_mm);

    LapComparisonReport {
        grid_dist_pct: norm_a.grid_dist_pct.clone(),
        grid_dist_m: norm_a.grid_dist_m.clone(),
        lap_a: norm_a,
        lap_b: norm_b,
        time_delta_s,
        speed_delta_mph,
        rake_delta_mm,
        side_rake_delta_mm,
    }
}

// ============================================================================
// 7. TELEMETRY CHANNEL CALCULATIONS (SECTIONS 3.A, 3.B, 3.C, 3.D)
// ============================================================================

/// Section 3.A: Driver Inputs & Vehicle Dynamics
pub fn compute_driver_inputs(
    speed_mps: Option<Vec<f32>>,
    throttle_raw: Option<Vec<f32>>,
    brake_raw: Option<Vec<f32>>,
    clutch_raw: Option<Vec<f32>>,
    steering_rad: Option<Vec<f32>>,
    gear: Option<Vec<i32>>,
    rpm: Option<Vec<f32>>,
    lat_accel_mps2: Option<Vec<f32>>,
    long_accel_mps2: Option<Vec<f32>>,
) -> DriverInputsAnalysis {
    let speed_mph = speed_mps
        .as_ref()
        .map(|v| v.iter().map(|&s| s * MPS_TO_MPH).collect::<Vec<f32>>());
    let speed_kph = speed_mps
        .as_ref()
        .map(|v| v.iter().map(|&s| s * MPS_TO_KPH).collect::<Vec<f32>>());

    let throttle_pct = throttle_raw
        .as_ref()
        .map(|v| v.iter().map(|&t| (t * 100.0).clamp(0.0, 100.0)).collect::<Vec<f32>>());
    let brake_pct = brake_raw
        .as_ref()
        .map(|v| v.iter().map(|&b| (b * 100.0).clamp(0.0, 100.0)).collect::<Vec<f32>>());
    let clutch_pct = clutch_raw
        .as_ref()
        .map(|v| v.iter().map(|&c| (c * 100.0).clamp(0.0, 100.0)).collect::<Vec<f32>>());

    let steering_deg = steering_rad
        .as_ref()
        .map(|v| v.iter().map(|&r| r * RAD_TO_DEG).collect::<Vec<f32>>());

    let lat_accel_g = lat_accel_mps2
        .as_ref()
        .map(|v| v.iter().map(|&a| a / 9.80665).collect::<Vec<f32>>());
    let long_accel_g = long_accel_mps2
        .as_ref()
        .map(|v| v.iter().map(|&a| a / 9.80665).collect::<Vec<f32>>());

    let speed_mph_stats = speed_mph
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();
    let speed_kph_stats = speed_kph
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();
    let throttle_stats = throttle_pct
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();
    let brake_stats = brake_pct
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();
    let steering_deg_stats = steering_deg
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();
    let rpm_stats = rpm
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();

    let full_throttle_pct_of_lap = throttle_pct
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| {
            let count = s.iter().filter(|&&v| v >= 98.0).count();
            (count as f32 / s.len() as f32) * 100.0
        })
        .unwrap_or(0.0);

    let braking_pct_of_lap = brake_pct
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| {
            let count = s.iter().filter(|&&v| v >= 2.0).count();
            (count as f32 / s.len() as f32) * 100.0
        })
        .unwrap_or(0.0);

    let coasting_pct_of_lap = match (&throttle_pct, &brake_pct) {
        (Some(thr), Some(brk)) if !thr.is_empty() && thr.len() == brk.len() => {
            let coast_count = thr
                .iter()
                .zip(brk.iter())
                .filter(|(&t, &b)| t < 5.0 && b < 2.0)
                .count();
            (coast_count as f32 / thr.len() as f32) * 100.0
        }
        _ => 0.0,
    };

    DriverInputsAnalysis {
        speed_mps,
        speed_mph,
        speed_kph,
        throttle_pct,
        brake_pct,
        clutch_pct,
        steering_rad,
        steering_deg,
        gear,
        rpm,
        lat_accel_g,
        long_accel_g,
        speed_mph_stats,
        speed_kph_stats,
        throttle_stats,
        brake_stats,
        steering_deg_stats,
        rpm_stats,
        full_throttle_pct_of_lap,
        braking_pct_of_lap,
        coasting_pct_of_lap,
    }
}

/// Section 3.B: Suspension Dynamics, Rake & Side Rake Formulas
///
/// Formulas:
/// - Front RH = (LF + RF) / 2
/// - Rear RH = (LR + RR) / 2
/// - Rake = Rear RH - Front RH  (Positive = tail-up / nose-down rake)
/// - Left RH = (LF + LR) / 2
/// - Right RH = (RF + RR) / 2
/// - Side Rake = Left RH - Right RH
/// - Front Split = LF - RF
/// - Rear Split = LR - RR
pub fn compute_suspension_dynamics(
    lf_m: Option<Vec<f32>>,
    rf_m: Option<Vec<f32>>,
    lr_m: Option<Vec<f32>>,
    rr_m: Option<Vec<f32>>,
    cfsr_m: Option<Vec<f32>>,
) -> SuspensionAnalysis {
    let to_mm = |opt: Option<Vec<f32>>| -> Option<Vec<f32>> {
        opt.map(|v| v.into_iter().map(|m| m * METERS_TO_MM).collect())
    };

    let lf_rh_mm = to_mm(lf_m);
    let rf_rh_mm = to_mm(rf_m);
    let lr_rh_mm = to_mm(lr_m);
    let rr_rh_mm = to_mm(rr_m);
    let cfsr_rh_mm = to_mm(cfsr_m);

    let front_rh_mm = match (&lf_rh_mm, &rf_rh_mm) {
        (Some(lf), Some(rf)) => Some(
            lf.iter()
                .zip(rf.iter())
                .map(|(&l, &r)| 0.5 * (l + r))
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let rear_rh_mm = match (&lr_rh_mm, &rr_rh_mm) {
        (Some(lr), Some(rr)) => Some(
            lr.iter()
                .zip(rr.iter())
                .map(|(&l, &r)| 0.5 * (l + r))
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let rake_mm = match (&rear_rh_mm, &front_rh_mm) {
        (Some(rear), Some(front)) => Some(
            rear.iter()
                .zip(front.iter())
                .map(|(&r, &f)| r - f)
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let left_rh_mm = match (&lf_rh_mm, &lr_rh_mm) {
        (Some(lf), Some(lr)) => Some(
            lf.iter()
                .zip(lr.iter())
                .map(|(&f, &r)| 0.5 * (f + r))
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let right_rh_mm = match (&rf_rh_mm, &rr_rh_mm) {
        (Some(rf), Some(rr)) => Some(
            rf.iter()
                .zip(rr.iter())
                .map(|(&f, &r)| 0.5 * (f + r))
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let side_rake_mm = match (&left_rh_mm, &right_rh_mm) {
        (Some(left), Some(right)) => Some(
            left.iter()
                .zip(right.iter())
                .map(|(&l, &r)| l - r)
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let front_split_mm = match (&lf_rh_mm, &rf_rh_mm) {
        (Some(lf), Some(rf)) => Some(
            lf.iter()
                .zip(rf.iter())
                .map(|(&l, &r)| l - r)
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let rear_split_mm = match (&lr_rh_mm, &rr_rh_mm) {
        (Some(lr), Some(rr)) => Some(
            lr.iter()
                .zip(rr.iter())
                .map(|(&l, &r)| l - r)
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let lf_stats_mm = lf_rh_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let rf_stats_mm = rf_rh_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let lr_stats_mm = lr_rh_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let rr_stats_mm = rr_rh_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let cfsr_stats_mm = cfsr_rh_mm.as_deref().map(ChannelStats::from_slice);
    let front_rh_stats_mm = front_rh_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let rear_rh_stats_mm = rear_rh_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let rake_stats_mm = rake_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let side_rake_stats_mm = side_rake_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let front_split_stats_mm = front_split_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let rear_split_stats_mm = rear_split_mm.as_deref().map(ChannelStats::from_slice).unwrap_or_default();

    SuspensionAnalysis {
        lf_rh_mm,
        rf_rh_mm,
        lr_rh_mm,
        rr_rh_mm,
        cfsr_rh_mm,
        front_rh_mm,
        rear_rh_mm,
        rake_mm,
        left_rh_mm,
        right_rh_mm,
        side_rake_mm,
        front_split_mm,
        rear_split_mm,
        lf_stats_mm,
        rf_stats_mm,
        lr_stats_mm,
        rr_stats_mm,
        cfsr_stats_mm,
        front_rh_stats_mm,
        rear_rh_stats_mm,
        rake_stats_mm,
        side_rake_stats_mm,
        front_split_stats_mm,
        rear_split_stats_mm,
    }
}

/// Vectorized numerical differentiation `ΔDefl / Δt` (with `Δt = 1 / tick_rate`)
/// followed by a zero-phase 3-tap Gaussian low-pass filter (`[0.25, 0.50, 0.25]`) to reject sensor noise.
pub fn differentiate_and_filter_defl_mm(defl_mm: &[f32], tick_rate_hz: f32) -> Vec<f32> {
    let n = defl_mm.len();
    if n == 0 {
        return Vec::new();
    }
    let fs = tick_rate_hz.max(1.0);
    if n == 1 {
        return vec![0.0];
    }

    let mut raw_vel = Vec::with_capacity(n);
    // Forward difference for index 0
    raw_vel.push((defl_mm[1] - defl_mm[0]) * fs);
    for i in 1..n {
        raw_vel.push((defl_mm[i] - defl_mm[i - 1]) * fs);
    }

    if n < 3 {
        return raw_vel;
    }

    // Apply symmetric [0.25, 0.50, 0.25] low-pass filter (zero phase shift, attenuates Nyquist sensor noise)
    let mut filtered = Vec::with_capacity(n);
    filtered.push(0.75 * raw_vel[0] + 0.25 * raw_vel[1]);
    for i in 1..n - 1 {
        filtered.push(0.25 * raw_vel[i - 1] + 0.50 * raw_vel[i] + 0.25 * raw_vel[i + 1]);
    }
    filtered.push(0.25 * raw_vel[n - 2] + 0.75 * raw_vel[n - 1]);
    filtered
}

/// Builds a shock velocity distribution histogram across `[-200.0, +200.0] mm/s` in `10.0 mm/s` increments
/// (40 discrete bins, with extreme outliers clamped into the outermost high-speed bins so 100.0% of samples are binned).
pub fn compute_corner_shock_data(
    corner_name: &'static str,
    defl_m: Option<Vec<f32>>,
    native_vel_mps: Option<Vec<f32>>,
    tick_rate_hz: f32,
) -> ShockCornerData {
    let deflection_mm = defl_m.map(|v| v.into_iter().map(|d| d * METERS_TO_MM).collect::<Vec<f32>>());

    let (source, velocity_mms) = if let Some(vel_mps) = native_vel_mps {
        let v_mms: Vec<f32> = vel_mps.into_iter().map(|v| v * METERS_TO_MM).collect();
        (ShockVelocitySource::NativeChannel, Some(v_mms))
    } else if let Some(ref d_mm) = deflection_mm {
        let v_mms = differentiate_and_filter_defl_mm(d_mm, tick_rate_hz);
        (ShockVelocitySource::DerivedFromDeflection, Some(v_mms))
    } else {
        (ShockVelocitySource::Unavailable, None)
    };

    let vel_slice = match velocity_mms.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => {
            return ShockCornerData {
                corner_name,
                source: ShockVelocitySource::Unavailable,
                deflection_mm,
                velocity_mms: None,
                ..Default::default()
            };
        }
    };

    // 20 bins from -200 mm/s to +200 mm/s (step = 20 mm/s), cleanly aligning at -50 / 0 / +50 boundaries
    // Wait: with step = 10 mm/s across -200..+200 (40 bins) or step = 10 mm/s across -150..+150 (30 bins),
    // -50 mm/s and +50 mm/s are exact bin boundaries! Let's use 10 mm/s increments from -150 to +150 mm/s (30 bins).
    const MIN_EDGE: f32 = -150.0;
    const BIN_STEP: f32 = 10.0;
    const NUM_BINS: usize = 30;

    let mut counts = [0usize; NUM_BINS];
    let mut valid_total = 0usize;

    let mut comp_sum = 0.0_f64;
    let mut comp_count = 0usize;
    let mut reb_sum = 0.0_f64;
    let mut reb_count = 0usize;
    let mut peak_compression_mms = 0.0_f32;
    let mut peak_rebound_mms = 0.0_f32;

    for &v in vel_slice {
        if !v.is_finite() {
            continue;
        }
        valid_total += 1;

        if v >= 0.0 {
            comp_sum += v as f64;
            comp_count += 1;
            if v > peak_compression_mms {
                peak_compression_mms = v;
            }
        } else {
            reb_sum += v.abs() as f64;
            reb_count += 1;
            if v < peak_rebound_mms {
                peak_rebound_mms = v;
            }
        }

        let raw_idx = ((v - MIN_EDGE) / BIN_STEP).floor() as isize;
        let idx = raw_idx.clamp(0, (NUM_BINS as isize) - 1) as usize;
        counts[idx] += 1;
    }

    let denom = valid_total.max(1) as f32;
    let mut bins = Vec::with_capacity(NUM_BINS);
    let mut total_binned_pct = 0.0_f32;
    let mut low_speed_compression_pct = 0.0_f32;
    let mut low_speed_rebound_pct = 0.0_f32;
    let mut high_speed_compression_pct = 0.0_f32;
    let mut high_speed_rebound_pct = 0.0_f32;

    for i in 0..NUM_BINS {
        let min_mms = MIN_EDGE + (i as f32) * BIN_STEP;
        let max_mms = min_mms + BIN_STEP;
        let center_mms = min_mms + 0.5 * BIN_STEP;
        let count = counts[i];
        let time_pct = if valid_total > 0 {
            (count as f32 / denom) * 100.0
        } else {
            0.0
        };
        total_binned_pct += time_pct;

        let regime = if max_mms <= -50.0 {
            high_speed_rebound_pct += time_pct;
            DamperRegime::HighSpeedRebound
        } else if max_mms <= 0.0 {
            low_speed_rebound_pct += time_pct;
            DamperRegime::LowSpeedRebound
        } else if max_mms <= 50.0 {
            low_speed_compression_pct += time_pct;
            DamperRegime::LowSpeedCompression
        } else {
            high_speed_compression_pct += time_pct;
            DamperRegime::HighSpeedCompression
        };

        bins.push(ShockHistogramBin {
            min_mms,
            max_mms,
            center_mms,
            count,
            time_pct,
            regime,
        });
    }

    ShockCornerData {
        corner_name,
        source,
        deflection_mm,
        velocity_mms,
        bins,
        total_binned_pct,
        low_speed_compression_pct,
        low_speed_rebound_pct,
        high_speed_compression_pct,
        high_speed_rebound_pct,
        avg_compression_mms: if comp_count > 0 {
            (comp_sum / comp_count as f64) as f32
        } else {
            0.0
        },
        avg_rebound_mms: if reb_count > 0 {
            (reb_sum / reb_count as f64) as f32
        } else {
            0.0
        },
        peak_compression_mms,
        peak_rebound_mms,
    }
}

pub fn compute_shock_analysis(
    lf_defl: Option<Vec<f32>>,
    rf_defl: Option<Vec<f32>>,
    lr_defl: Option<Vec<f32>>,
    rr_defl: Option<Vec<f32>>,
    lf_vel: Option<Vec<f32>>,
    rf_vel: Option<Vec<f32>>,
    lr_vel: Option<Vec<f32>>,
    rr_vel: Option<Vec<f32>>,
    tick_rate_hz: f32,
) -> ShockAnalysis {
    let lf = compute_corner_shock_data("LF", lf_defl, lf_vel, tick_rate_hz);
    let rf = compute_corner_shock_data("RF", rf_defl, rf_vel, tick_rate_hz);
    let lr = compute_corner_shock_data("LR", lr_defl, lr_vel, tick_rate_hz);
    let rr = compute_corner_shock_data("RR", rr_defl, rr_vel, tick_rate_hz);

    let has_any_shock_data = lf.source != ShockVelocitySource::Unavailable
        || rf.source != ShockVelocitySource::Unavailable
        || lr.source != ShockVelocitySource::Unavailable
        || rr.source != ShockVelocitySource::Unavailable;

    ShockAnalysis {
        lf,
        rf,
        lr,
        rr,
        has_any_shock_data,
    }
}

/// Section 3.D: Tire Surface / Carcass Temperatures & Cold/Hot Pressures
///
/// Accounts for iRacing Left vs. Right tire orientation:
/// - Left tires (LF, LR): CL = Outside, CM = Middle, CR = Inside (O·M·I)
/// - Right tires (RF, RR): CL = Inside, CM = Middle, CR = Outside (I·M·O)
/// - Inside-to-Outside Spread: ΔT = T_inside - T_outside
/// - Center Wear Bias: T_center - (T_inside + T_outside) / 2
pub fn compute_corner_tire_analysis(
    corner_name: &'static str,
    is_left_side: bool,
    temp_source: TireTempSource,
    cl_c: Option<Vec<f32>>,
    cm_c: Option<Vec<f32>>,
    cr_c: Option<Vec<f32>>,
    pressure_kpa: Option<Vec<f32>>,
) -> TireCornerAnalysis {
    let c_to_f = |opt: Option<Vec<f32>>| -> Option<Vec<f32>> {
        opt.map(|v| v.into_iter().map(|c| (c * 9.0 / 5.0) + 32.0).collect())
    };

    let temp_cl_f = c_to_f(cl_c);
    let temp_cm_f = c_to_f(cm_c);
    let temp_cr_f = c_to_f(cr_c);

    // Resolve Inside vs Outside based on Left (LF/LR) vs Right (RF/RR) corner
    let (inside_temp_f, outside_temp_f) = if is_left_side {
        // Left side: CR is Inside, CL is Outside
        (temp_cr_f.clone(), temp_cl_f.clone())
    } else {
        // Right side: CL is Inside, CR is Outside
        (temp_cl_f.clone(), temp_cr_f.clone())
    };
    let center_temp_f = temp_cm_f.clone();

    let inside_outside_spread_f = match (&inside_temp_f, &outside_temp_f) {
        (Some(ins), Some(out)) => Some(
            ins.iter()
                .zip(out.iter())
                .map(|(&i, &o)| i - o)
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let center_wear_bias_f = match (&center_temp_f, &inside_temp_f, &outside_temp_f) {
        (Some(cen), Some(ins), Some(out)) => Some(
            cen.iter()
                .zip(ins.iter().zip(out.iter()))
                .map(|(&c, (&i, &o))| c - 0.5 * (i + o))
                .collect::<Vec<f32>>(),
        ),
        _ => None,
    };

    let pressure_psi = pressure_kpa.map(|v| v.into_iter().map(|kpa| kpa * KPA_TO_PSI).collect::<Vec<f32>>());

    let inside_stats_f = inside_temp_f.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let center_stats_f = center_temp_f.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let outside_stats_f = outside_temp_f.as_deref().map(ChannelStats::from_slice).unwrap_or_default();
    let spread_stats_f = inside_outside_spread_f
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();
    let center_bias_stats_f = center_wear_bias_f
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();
    let pressure_stats_psi = pressure_psi
        .as_deref()
        .map(ChannelStats::from_slice)
        .unwrap_or_default();

    let (start_pressure_psi, end_pressure_psi, peak_pressure_psi, pressure_build_psi) =
        if let Some(ref p) = pressure_psi {
            if !p.is_empty() {
                let start_p = p[0];
                let end_p = p[p.len() - 1];
                let peak_p = pressure_stats_psi.max;
                (start_p, end_p, peak_p, end_p - start_p)
            } else {
                (0.0, 0.0, 0.0, 0.0)
            }
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

    TireCornerAnalysis {
        corner_name,
        is_left_side,
        temp_source,
        temp_cl_f,
        temp_cm_f,
        temp_cr_f,
        inside_temp_f,
        center_temp_f,
        outside_temp_f,
        inside_outside_spread_f,
        center_wear_bias_f,
        pressure_psi,
        inside_stats_f,
        center_stats_f,
        outside_stats_f,
        spread_stats_f,
        center_bias_stats_f,
        pressure_stats_psi,
        cold_pressure_psi: start_pressure_psi,
        start_pressure_psi,
        end_pressure_psi,
        peak_pressure_psi,
        pressure_build_psi,
        wear_outside_pct: 100.0,
        wear_center_pct: 100.0,
        wear_inside_pct: 100.0,
    }
}

type RawCornerTireTuple = (
    TireTempSource,
    Option<Vec<f32>>,
    Option<Vec<f32>>,
    Option<Vec<f32>>,
    Option<Vec<f32>>,
);

pub fn compute_tire_analysis(
    lf: RawCornerTireTuple,
    rf: RawCornerTireTuple,
    lr: RawCornerTireTuple,
    rr: RawCornerTireTuple,
) -> LapTireAnalysis {
    let lf_data = compute_corner_tire_analysis("LF", true, lf.0, lf.1, lf.2, lf.3, lf.4);
    let rf_data = compute_corner_tire_analysis("RF", false, rf.0, rf.1, rf.2, rf.3, rf.4);
    let lr_data = compute_corner_tire_analysis("LR", true, lr.0, lr.1, lr.2, lr.3, lr.4);
    let rr_data = compute_corner_tire_analysis("RR", false, rr.0, rr.1, rr.2, rr.3, rr.4);

    let has_temp_data = lf_data.temp_source != TireTempSource::Unavailable
        || rf_data.temp_source != TireTempSource::Unavailable
        || lr_data.temp_source != TireTempSource::Unavailable
        || rr_data.temp_source != TireTempSource::Unavailable;

    let has_pressure_data = lf_data.pressure_psi.is_some()
        || rf_data.pressure_psi.is_some()
        || lr_data.pressure_psi.is_some()
        || rr_data.pressure_psi.is_some();

    LapTireAnalysis {
        lf: lf_data,
        rf: rf_data,
        lr: lr_data,
        rr: rr_data,
        has_temp_data,
        has_pressure_data,
    }
}

/// Parses `CarSetup` tire measurements (`LastTempsOMI`, `LastTempsIMO`, `LastHotPressure`, `TreadRemaining`)
/// and cold/starting pressures (`StartingPressure` / `ColdPressure`) from the `.ibt` YAML header.
fn parse_yaml_garage_tires_and_cold(yaml_str: &str) -> (Option<crate::StintTireSet>, Option<[f32; 4]>) {
    if yaml_str.trim().is_empty() {
        return (None, None);
    }
    if let Ok(root) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
        if let Some(cs) = root.get("CarSetup") {
            let tires = crate::parse_car_setup_tires(cs);
            let cold = crate::parse_car_setup_cold_pressures_kpa(cs);
            if tires.is_some() || cold.is_some() {
                return (tires, cold);
            }
        }
    }
    parse_yaml_tires_line_fallback(yaml_str)
}

/// Line-by-line fallback scanner for `CarSetup` tire readings in case `serde_yaml` fails on custom setup strings.
fn parse_yaml_tires_line_fallback(yaml_str: &str) -> (Option<crate::StintTireSet>, Option<[f32; 4]>) {
    let mut in_car_setup = false;
    let mut active_corner: Option<usize> = None; // 0=LF, 1=RF, 2=LR, 3=RR
    let mut corners = [
        crate::TireCornerData::default(),
        crate::TireCornerData::default(),
        crate::TireCornerData::default(),
        crate::TireCornerData::default(),
    ];
    let mut cold_kpa = [0.0_f32; 4];

    for line in yaml_str.lines() {
        let trimmed = line.trim();
        if trimmed == "CarSetup:" {
            in_car_setup = true;
            continue;
        }
        if !in_car_setup {
            continue;
        }
        if !line.starts_with(' ') && !line.starts_with('\t') && trimmed.ends_with(':') {
            break;
        }
        match trimmed {
            "LeftFrontTire:" | "LeftFront:" | "Left Front:" => active_corner = Some(0),
            "RightFrontTire:" | "RightFront:" | "Right Front:" => active_corner = Some(1),
            "LeftRearTire:" | "LeftRear:" | "Left Rear:" => active_corner = Some(2),
            "RightRearTire:" | "RightRear:" | "Right Rear:" => active_corner = Some(3),
            _ => {
                if let Some(c_idx) = active_corner {
                    if let Some(val) = trimmed
                        .strip_prefix("LastHotPressure:")
                        .or_else(|| trimmed.strip_prefix("LastPressure:"))
                        .or_else(|| trimmed.strip_prefix("HotPressure:"))
                    {
                        let p = crate::parse_pressure_str_to_kpa(val.trim().trim_matches('"'));
                        if p > 1.0 {
                            corners[c_idx].pressure = p;
                        }
                    } else if let Some(val) = trimmed
                        .strip_prefix("StartingPressure:")
                        .or_else(|| trimmed.strip_prefix("ColdPressure:"))
                    {
                        let p = crate::parse_pressure_str_to_kpa(val.trim().trim_matches('"'));
                        if p > 1.0 {
                            cold_kpa[c_idx] = p;
                        }
                    } else if let Some(val) = trimmed
                        .strip_prefix("LastTempsOMI:")
                        .or_else(|| trimmed.strip_prefix("LastTempsIMO:"))
                        .or_else(|| trimmed.strip_prefix("TempsOMI:"))
                        .or_else(|| trimmed.strip_prefix("TempsIMO:"))
                    {
                        let clean = val.trim().trim_matches('"');
                        let is_f = clean.contains('F') || clean.contains('f');
                        if let Some(mut t) = crate::parse_three_floats(clean) {
                            if is_f || t[0] > 120.0 || t[1] > 120.0 || t[2] > 120.0 {
                                t[0] = (t[0] - 32.0) / 1.8;
                                t[1] = (t[1] - 32.0) / 1.8;
                                t[2] = (t[2] - 32.0) / 1.8;
                            }
                            corners[c_idx].temp_l = t[0];
                            corners[c_idx].temp_m = t[1];
                            corners[c_idx].temp_r = t[2];
                        }
                    } else if let Some(val) = trimmed
                        .strip_prefix("TreadRemaining:")
                        .or_else(|| trimmed.strip_prefix("Wear:"))
                    {
                        if let Some(w) = crate::parse_three_floats(val.trim().trim_matches('"')) {
                            corners[c_idx].wear_l = w[0];
                            corners[c_idx].wear_m = w[1];
                            corners[c_idx].wear_r = w[2];
                        }
                    }
                }
            }
        }
    }

    let has_tires = corners.iter().any(|c| c.temp_m > 5.0 || c.pressure > 1.0 || c.wear_m > 0.1);
    let tire_set = if has_tires {
        Some(crate::StintTireSet {
            lf: corners[0].clone(),
            rf: corners[1].clone(),
            lr: corners[2].clone(),
            rr: corners[3].clone(),
            has_data: true,
        })
    } else {
        None
    };
    let cold_opt = if cold_kpa.iter().any(|&p| p > 1.0) {
        Some(cold_kpa)
    } else {
        None
    };
    (tire_set, cold_opt)
}

type FullCornerChannels<'a> = (
    &'a Option<Vec<f32>>,
    &'a Option<Vec<f32>>,
    &'a Option<Vec<f32>>,
    &'a Option<Vec<f32>>,
    &'a Option<Vec<f32>>,
    &'a Option<Vec<f32>>,
    &'a Option<Vec<f32>>,
);

/// Scans the `.ibt` session ticks for the hottest pit-stall / post-run tire reading
/// (matching `read_player_tires` in `src/main.rs` when the car stops in the pit stall).
fn extract_session_pit_latch_tires(
    total_ticks: usize,
    lf: FullCornerChannels<'_>,
    rf: FullCornerChannels<'_>,
    lr: FullCornerChannels<'_>,
    rr: FullCornerChannels<'_>,
) -> Option<crate::StintTireSet> {
    if total_ticks == 0 {
        return None;
    }

    let extract_best_corner = |c: FullCornerChannels<'_>| -> crate::TireCornerData {
        let (cl_opt, cm_opt, cr_opt, wl_opt, wm_opt, wr_opt, p_opt) = c;
        let mut best_idx = total_ticks.saturating_sub(1);
        let mut best_temp = -1.0_f32;

        if let Some(cm) = cm_opt {
            for (idx, &v) in cm.iter().enumerate() {
                if v.is_finite() && v > best_temp {
                    best_temp = v;
                    best_idx = idx;
                }
            }
        }

        let to_wear = |w: f32| -> f32 {
            if w > 0.0 && w <= 1.0 {
                w * 100.0
            } else if w > 1.0 {
                w
            } else {
                100.0
            }
        };

        let temp_l = cl_opt.as_ref().and_then(|v| v.get(best_idx).copied()).unwrap_or(0.0);
        let temp_m = cm_opt.as_ref().and_then(|v| v.get(best_idx).copied()).unwrap_or(0.0);
        let temp_r = cr_opt.as_ref().and_then(|v| v.get(best_idx).copied()).unwrap_or(0.0);
        let wear_l = wl_opt
            .as_ref()
            .and_then(|v| v.last().copied())
            .map(to_wear)
            .unwrap_or(100.0);
        let wear_m = wm_opt
            .as_ref()
            .and_then(|v| v.last().copied())
            .map(to_wear)
            .unwrap_or(100.0);
        let wear_r = wr_opt
            .as_ref()
            .and_then(|v| v.last().copied())
            .map(to_wear)
            .unwrap_or(100.0);
        let pressure = p_opt
            .as_ref()
            .and_then(|v| {
                v.iter()
                    .copied()
                    .filter(|x| x.is_finite() && *x > 1.0)
                    .fold(None, |acc: Option<f32>, x| Some(acc.map_or(x, |a| a.max(x))))
            })
            .unwrap_or(0.0);

        crate::TireCornerData {
            temp_l,
            temp_m,
            temp_r,
            wear_l,
            wear_m,
            wear_r,
            pressure,
        }
    };

    let lf_c = extract_best_corner(lf);
    let rf_c = extract_best_corner(rf);
    let lr_c = extract_best_corner(lr);
    let rr_c = extract_best_corner(rr);

    let has_data = lf_c.temp_m > 5.0 || rf_c.temp_m > 5.0 || lf_c.pressure > 1.0 || rf_c.pressure > 1.0;
    if has_data {
        Some(crate::StintTireSet {
            lf: lf_c,
            rf: rf_c,
            lr: lr_c,
            rr: rr_c,
            has_data: true,
        })
    } else {
        None
    }
}

fn merge_garage_and_pit_tires(
    yaml_tires: Option<crate::StintTireSet>,
    pit_tires: Option<crate::StintTireSet>,
) -> Option<crate::StintTireSet> {
    match (yaml_tires, pit_tires) {
        (Some(y), Some(p)) => {
            let merge_corner = |yc: &crate::TireCornerData, pc: &crate::TireCornerData| -> crate::TireCornerData {
                let (temp_l, temp_m, temp_r) = if yc.temp_m >= pc.temp_m && yc.temp_m > 5.0 {
                    (yc.temp_l, yc.temp_m, yc.temp_r)
                } else {
                    (pc.temp_l, pc.temp_m, pc.temp_r)
                };
                // Pick the lower tread remaining % (more worn post-run reading) if valid
                let (wear_l, wear_m, wear_r) = if yc.wear_m > 0.1 && (pc.wear_m <= 0.1 || yc.wear_m <= pc.wear_m) {
                    (yc.wear_l, yc.wear_m, yc.wear_r)
                } else if pc.wear_m > 0.1 {
                    (pc.wear_l, pc.wear_m, pc.wear_r)
                } else {
                    (100.0, 100.0, 100.0)
                };
                let pressure = if yc.pressure >= pc.pressure && yc.pressure > 1.0 {
                    yc.pressure
                } else {
                    pc.pressure
                };
                crate::TireCornerData {
                    temp_l,
                    temp_m,
                    temp_r,
                    wear_l,
                    wear_m,
                    wear_r,
                    pressure,
                }
            };
            Some(crate::StintTireSet {
                lf: merge_corner(&y.lf, &p.lf),
                rf: merge_corner(&y.rf, &p.rf),
                lr: merge_corner(&y.lr, &p.lr),
                rr: merge_corner(&y.rr, &p.rr),
                has_data: y.has_data || p.has_data,
            })
        }
        (Some(y), None) => Some(y),
        (None, Some(p)) => Some(p),
        (None, None) => None,
    }
}

fn sample_end_wear(
    wl: &Option<Vec<f32>>,
    wm: &Option<Vec<f32>>,
    wr: &Option<Vec<f32>>,
    start: usize,
    end: usize,
) -> Option<[f32; 3]> {
    let idx = end.saturating_sub(1).max(start);
    let to_pct = |w: f32| -> f32 {
        if w > 0.0 && w <= 1.0 {
            w * 100.0
        } else if w > 1.0 {
            w
        } else {
            100.0
        }
    };
    let l = wl.as_ref().and_then(|v| v.get(idx).copied()).map(to_pct)?;
    let m = wm.as_ref().and_then(|v| v.get(idx).copied()).map(to_pct)?;
    let r = wr.as_ref().and_then(|v| v.get(idx).copied()).map(to_pct)?;
    Some([l, m, r])
}

/// Enriches a lap's `LapTireAnalysis` with Pit-Stop / Garage Hot Tire Readings (`CarSetup` YAML / Pit-Stall Latch),
/// Cold/Starting Pressures, and 3-Zone Tread Remaining `%` when on-track telemetry channels are frozen at cold values.
fn apply_hot_tires_and_wear_to_lap(
    tires: &mut LapTireAnalysis,
    garage: Option<&crate::StintTireSet>,
    cold_press_kpa: &[f32; 4],
    lap_wear: [Option<[f32; 3]>; 4],
) {
    let apply_corner = |corner: &mut TireCornerAnalysis,
                        g_opt: Option<&crate::TireCornerData>,
                        cold_kpa: f32,
                        w_opt: Option<[f32; 3]>| {
        if cold_kpa > 1.0 {
            corner.cold_pressure_psi = cold_kpa * KPA_TO_PSI;
        } else if corner.start_pressure_psi > 0.1 {
            corner.cold_pressure_psi = corner.start_pressure_psi;
        }

        // Resolve tread wear (O·M·I on Left, I·M·O on Right)
        let resolved_w = match (w_opt, g_opt) {
            (Some(w), Some(g)) => {
                if g.wear_m > 0.1 && g.wear_m < w[1] {
                    [g.wear_l, g.wear_m, g.wear_r]
                } else {
                    w
                }
            }
            (Some(w), None) => w,
            (None, Some(g)) if g.wear_m > 0.1 => [g.wear_l, g.wear_m, g.wear_r],
            _ => [100.0, 100.0, 100.0],
        };
        if corner.is_left_side {
            corner.wear_outside_pct = resolved_w[0];
            corner.wear_center_pct = resolved_w[1];
            corner.wear_inside_pct = resolved_w[2];
        } else {
            corner.wear_inside_pct = resolved_w[0];
            corner.wear_center_pct = resolved_w[1];
            corner.wear_outside_pct = resolved_w[2];
        }

        // If on-track *tempCL/CM/CR is static/frozen (or unavailable) while Garage/Pit-Stop hot reading has hotter temps,
        // promote the corner to Pit-Stop / Garage Hot Reading (exact match to pit stop & garage screen!)
        if let Some(g) = g_opt {
            if corner.temp_source != TireTempSource::Surface && g.temp_m > 5.0 {
                let g_cl_f = g.temp_l * 1.8 + 32.0;
                let g_cm_f = g.temp_m * 1.8 + 32.0;
                let g_cr_f = g.temp_r * 1.8 + 32.0;
                let lap_is_static =
                    (corner.center_stats_f.max - corner.center_stats_f.min).abs() < 0.08;
                let garage_is_hotter = g_cm_f > corner.center_stats_f.max + 0.5;

                if corner.temp_source == TireTempSource::Unavailable
                    || garage_is_hotter
                    || (lap_is_static && g.temp_m > 5.0)
                {
                    let (in_f, out_f) = if corner.is_left_side {
                        (g_cr_f, g_cl_f)
                    } else {
                        (g_cl_f, g_cr_f)
                    };
                    let spread_f = in_f - out_f;
                    let bias_f = g_cm_f - 0.5 * (in_f + out_f);

                    corner.temp_source = TireTempSource::GarageOrPitHot;
                    corner.inside_stats_f = ChannelStats {
                        min: in_f,
                        max: in_f,
                        avg: in_f,
                    };
                    corner.center_stats_f = ChannelStats {
                        min: g_cm_f,
                        max: g_cm_f,
                        avg: g_cm_f,
                    };
                    corner.outside_stats_f = ChannelStats {
                        min: out_f,
                        max: out_f,
                        avg: out_f,
                    };
                    corner.spread_stats_f = ChannelStats {
                        min: spread_f,
                        max: spread_f,
                        avg: spread_f,
                    };
                    corner.center_bias_stats_f = ChannelStats {
                        min: bias_f,
                        max: bias_f,
                        avg: bias_f,
                    };
                }
            }

            if g.pressure > 1.0 {
                let g_psi = g.pressure * KPA_TO_PSI;
                let press_is_static =
                    (corner.end_pressure_psi - corner.start_pressure_psi).abs() < 0.02;
                if corner.end_pressure_psi <= 0.1
                    || (press_is_static && g_psi > corner.end_pressure_psi + 0.05)
                {
                    corner.end_pressure_psi = g_psi;
                    corner.peak_pressure_psi = corner.peak_pressure_psi.max(g_psi);
                }
            }
        }

        if corner.cold_pressure_psi > 0.1 && corner.end_pressure_psi > 0.1 {
            corner.pressure_build_psi = corner.end_pressure_psi - corner.cold_pressure_psi;
        }
    };

    apply_corner(&mut tires.lf, garage.map(|g| &g.lf), cold_press_kpa[0], lap_wear[0]);
    apply_corner(&mut tires.rf, garage.map(|g| &g.rf), cold_press_kpa[1], lap_wear[1]);
    apply_corner(&mut tires.lr, garage.map(|g| &g.lr), cold_press_kpa[2], lap_wear[2]);
    apply_corner(&mut tires.rr, garage.map(|g| &g.rr), cold_press_kpa[3], lap_wear[3]);

    tires.has_temp_data = tires.lf.temp_source != TireTempSource::Unavailable
        || tires.rf.temp_source != TireTempSource::Unavailable
        || tires.lr.temp_source != TireTempSource::Unavailable
        || tires.rr.temp_source != TireTempSource::Unavailable;
    tires.has_pressure_data = tires.lf.end_pressure_psi > 0.1
        || tires.rf.end_pressure_psi > 0.1
        || tires.lr.end_pressure_psi > 0.1
        || tires.rr.end_pressure_psi > 0.1;
}

// ============================================================================
// 8. SYNTHETIC / DEMO .IBT GENERATOR (FOR UI PREVIEW & AUTOMATED VERIFICATION)
// ============================================================================

/// Generates a realistic binary `.ibt` file in memory containing 3 laps (Out-Lap, Fast Lap 1, Fast Lap 2 PB)
/// at 60 Hz with full YAML header, driver inputs, corner & center ride heights, shock deflections/velocities,
/// and 4-corner tire temperatures and pressures.
pub fn build_demo_ibt_session() -> Result<IbtSession, String> {
    let yaml = r#"---
WeekendInfo:
  TrackName: watkinsglen 2021 boot
  TrackDisplayName: Watkins Glen International
  TrackConfigName: Boot
  TrackLength: 5.47 km
  TrackType: road course
  TrackWeatherType: Realistic
  TrackSkies: Partly Cloudy
  TrackAirTemp: 23.5 C
  TrackSurfaceTemp: 34.2 C
  TrackWindVel: 3.13 m/s
  TrackWindDir: 0.785 rad
  WeekendOptions:
    Date: 2026-10-22
    WindDirection: NE
    RelativeHumidity: 52 %
SessionInfo:
  Sessions:
    - SessionNum: 0
      SessionType: Practice
      SessionTrackRubberState: high usage
DriverInfo:
  DriverCarIdx: 0
  DriverSetupName: S4S_WGI_Qual_v4.sto
  DriverCarFuelMaxLtr: 110.0
  Drivers:
    - CarIdx: 0
      UserName: Search For Speed Driver
      CarNumber: "42"
      IRating: 4850
      CarScreenName: Porsche 963 GTP
      CarScreenNameShort: Porsche 963
      CarPath: porsche963gtp
CarSetup:
  UpdateCount: 2
  TiresAero:
    LeftFrontTire:
      StartingPressure: 152.0 kPa
      LastHotPressure: 171.5 kPa
      LastTempsOMI: 85C, 91C, 96C
      TreadRemaining: 97%, 96%, 95%
    RightFrontTire:
      StartingPressure: 154.0 kPa
      LastHotPressure: 173.5 kPa
      LastTempsIMO: 98C, 93C, 87C
      TreadRemaining: 94%, 95%, 97%
    LeftRearTire:
      StartingPressure: 148.0 kPa
      LastHotPressure: 167.5 kPa
      LastTempsOMI: 83C, 89C, 93C
      TreadRemaining: 97%, 96%, 95%
    RightRearTire:
      StartingPressure: 150.0 kPa
      LastHotPressure: 169.5 kPa
      LastTempsIMO: 95C, 90C, 84C
      TreadRemaining: 95%, 96%, 97%
...
"#;

    // Channels to include in our binary .ibt packet
    let channel_defs: Vec<(&str, &str, &str, IbtVarType)> = vec![
        ("SessionTime", "Seconds since session start", "s", IbtVarType::Double),
        ("Lap", "Current lap number", "", IbtVarType::Int),
        ("LapDist", "Meters traveled from S/F", "m", IbtVarType::Float),
        ("LapDistPct", "Percentage of lap completed", "%", IbtVarType::Float),
        ("OnPitRoad", "Is player car on pit road", "", IbtVarType::Bool),
        ("PlayerTrackSurface", "Player track surface", "", IbtVarType::Int),
        ("Speed", "GPS vehicle speed", "m/s", IbtVarType::Float),
        ("Throttle", "0=off throttle to 1=full throttle", "%", IbtVarType::Float),
        ("Brake", "0=brake released to 1=max pedal force", "%", IbtVarType::Float),
        ("SteeringWheelAngle", "Steering wheel angle", "rad", IbtVarType::Float),
        ("Gear", "Transmission gear", "", IbtVarType::Int),
        ("RPM", "Engine rpm", "revs/min", IbtVarType::Float),
        ("LFrideHeight", "LF ride height", "m", IbtVarType::Float),
        ("RFrideHeight", "RF ride height", "m", IbtVarType::Float),
        ("LRrideHeight", "LR ride height", "m", IbtVarType::Float),
        ("RRrideHeight", "RR ride height", "m", IbtVarType::Float),
        ("CFSRrideHeight", "Center front splitter ride height", "m", IbtVarType::Float),
        ("LFshockDefl", "LF shock deflection", "m", IbtVarType::Float),
        ("RFshockDefl", "RF shock deflection", "m", IbtVarType::Float),
        ("LRshockDefl", "LR shock deflection", "m", IbtVarType::Float),
        ("RRshockDefl", "RR shock deflection", "m", IbtVarType::Float),
        ("LFshockVel", "LF shock velocity", "m/s", IbtVarType::Float),
        ("RFshockVel", "RF shock velocity", "m/s", IbtVarType::Float),
        ("LRshockVel", "LR shock velocity", "m/s", IbtVarType::Float),
        ("RRshockVel", "RR shock velocity", "m/s", IbtVarType::Float),
        ("LFtempCL", "LF tire left carcass temperature", "C", IbtVarType::Float),
        ("LFtempCM", "LF tire middle carcass temperature", "C", IbtVarType::Float),
        ("LFtempCR", "LF tire right carcass temperature", "C", IbtVarType::Float),
        ("RFtempCL", "RF tire left carcass temperature", "C", IbtVarType::Float),
        ("RFtempCM", "RF tire middle carcass temperature", "C", IbtVarType::Float),
        ("RFtempCR", "RF tire right carcass temperature", "C", IbtVarType::Float),
        ("LRtempCL", "LR tire left carcass temperature", "C", IbtVarType::Float),
        ("LRtempCM", "LR tire middle carcass temperature", "C", IbtVarType::Float),
        ("LRtempCR", "LR tire right carcass temperature", "C", IbtVarType::Float),
        ("RRtempCL", "RR tire left carcass temperature", "C", IbtVarType::Float),
        ("RRtempCM", "RR tire middle carcass temperature", "C", IbtVarType::Float),
        ("RRtempCR", "RR tire right carcass temperature", "C", IbtVarType::Float),
        ("LFpressure", "LF tire hot pressure", "kPa", IbtVarType::Float),
        ("RFpressure", "RF tire hot pressure", "kPa", IbtVarType::Float),
        ("LRpressure", "LR tire hot pressure", "kPa", IbtVarType::Float),
        ("RRpressure", "RR tire hot pressure", "kPa", IbtVarType::Float),
    ];

    let tick_rate = 60i32;
    // 3 laps:
    // Lap 1 (Out-lap): 240 ticks (4.0s compressed demo resolution or 600 ticks = 10.0s)
    // Lap 2 (Flying Lap A): 900 ticks (92.40s simulated via SessionTime step, or 900 ticks at 60Hz = 15.0s)
    //Wait: let's make SessionTime step realistic or 60Hz ticks!
    // At 60 Hz, 1800 ticks per lap = 30.0s lap (3 laps = 5400 ticks, ~1 MB in memory, parses in <3ms!).
    let lap1_ticks = 900usize;  // Out-lap (15.0s)
    let lap2_ticks = 1830usize; // Flying Lap 2 (30.500s)
    let lap3_ticks = 1800usize; // Flying Lap 3 PB (30.000s)
    let total_ticks = lap1_ticks + lap2_ticks + lap3_ticks;
    let track_len_m = 5470.0_f32;

    let mut writer = SyntheticIbtBuilder::new(tick_rate, yaml, &channel_defs);

    let mut session_time = 100.0_f64;
    let dt = 1.0 / (tick_rate as f64);

    let laps_spec = [
        (1i32, lap1_ticks, true, 1.0_f32),  // Lap 1: Out-lap
        (2i32, lap2_ticks, false, 0.98_f32), // Lap 2: Flying Lap
        (3i32, lap3_ticks, false, 1.02_f32), // Lap 3: Faster Flying PB Lap
    ];

    for (lap_num, n_ticks, is_out, pace_mult) in laps_spec {
        for t in 0..n_ticks {
            let pct = (t as f32) / ((n_ticks - 1).max(1) as f32);
            let dist_m = pct * track_len_m;
            let phase = pct * 2.0 * std::f32::consts::PI;

            // Simulate 5 corners around the lap using harmonics
            let corner_wave = (phase * 4.0).sin() * 0.5 + (phase * 8.0).cos() * 0.5;
            let braking_zone = (-corner_wave - 0.25).max(0.0) * 1.4;
            let brake = braking_zone.clamp(0.0, 1.0);
            let throttle = if brake > 0.05 {
                0.0
            } else {
                ((corner_wave + 0.45) * 1.3 * pace_mult).clamp(0.15, 1.0)
            };

            let base_speed = if is_out && pct < 0.18 {
                22.0 // Pit lane speed limit
            } else {
                (62.0 + 26.0 * corner_wave - 24.0 * brake) * pace_mult
            };
            let speed_mps = base_speed.max(18.0);
            let gear = ((speed_mps / 13.5).floor() as i32).clamp(2, 7);
            let rpm = 5200.0 + (speed_mps * 42.0) + (throttle * 650.0);

            // Steering angle in radians (left & right corners)
            let steer_rad = (phase * 5.0).sin() * 0.32 * (1.0 - 0.3 * throttle);

            // Ride heights in meters:
            // Under braking, front compresses (nose down -> Front RH drops, Rear RH lifts -> positive Rake increases)
            // Aero compression at high speed lowers both front and rear
            let aero_drop = (speed_mps / 85.0).powi(2) * 0.014;
            let pitch_shift = brake * 0.012 - throttle * 0.003; // + under braking
            let roll_shift = steer_rad * 0.022; // lateral roll

            let lf_rh = (0.042 - aero_drop - pitch_shift + roll_shift).max(0.008);
            let rf_rh = (0.042 - aero_drop - pitch_shift - roll_shift).max(0.008);
            let lr_rh = (0.064 - aero_drop * 0.85 + pitch_shift * 0.8 + roll_shift * 0.9).max(0.015);
            let rr_rh = (0.064 - aero_drop * 0.85 + pitch_shift * 0.8 - roll_shift * 0.9).max(0.015);
            let cfsr_rh = (0.5 * (lf_rh + rf_rh) - 0.004).max(0.004);

            // Shock deflections (m) & velocities (m/s)
            let bump_ripple = (phase * 36.0).sin() * 0.0025 + (phase * 95.0).cos() * 0.0012;
            let lf_defl = 0.050 - lf_rh + bump_ripple;
            let rf_defl = 0.050 - rf_rh - bump_ripple;
            let lr_defl = 0.075 - lr_rh + bump_ripple * 0.8;
            let rr_defl = 0.075 - rr_rh - bump_ripple * 0.8;

            let lf_vel = (phase * 36.0).cos() * 0.032 + (phase * 95.0).sin() * 0.024;
            let rf_vel = -(phase * 36.0).cos() * 0.030 + (phase * 95.0).cos() * 0.022;
            let lr_vel = (phase * 36.0).sin() * 0.028 - (phase * 95.0).sin() * 0.019;
            let rr_vel = -(phase * 36.0).sin() * 0.027 - (phase * 95.0).cos() * 0.021;

            // Tire Temperatures (°C):
            // Remember: LF/LR (Left): CL=Outside, CM=Center, CR=Inside (Inside hotter -> CR > CM > CL)
            // RF/RR (Right): CL=Inside, CM=Center, CR=Outside (Inside hotter -> CL > CM > CR)
            let heat_prog = (lap_num as f32 - 1.0) * 3.5 + pct * 3.5;
            let lf_cl = 78.0 + heat_prog + roll_shift.abs() * 15.0; // LF Outside
            let lf_cm = 84.0 + heat_prog + brake * 4.0;             // LF Center
            let lf_cr = 89.0 + heat_prog + brake * 6.0;             // LF Inside

            let rf_cl = 91.0 + heat_prog + brake * 6.0;             // RF Inside
            let rf_cm = 86.0 + heat_prog + brake * 4.0;             // RF Center
            let rf_cr = 80.0 + heat_prog + roll_shift.abs() * 15.0; // RF Outside

            let lr_cl = 76.0 + heat_prog + throttle * 3.0;          // LR Outside
            let lr_cm = 82.0 + heat_prog + throttle * 4.5;          // LR Center
            let lr_cr = 86.0 + heat_prog + throttle * 5.5;          // LR Inside

            let rr_cl = 88.0 + heat_prog + throttle * 5.5;          // RR Inside
            let rr_cm = 83.5 + heat_prog + throttle * 4.5;          // RR Center
            let rr_cr = 77.5 + heat_prog + throttle * 3.0;          // RR Outside

            // Tire Pressures (kPa): ~152 kPa (22.0 psi) building to ~172 kPa (25.0 psi)
            let press_prog = (lap_num as f32 - 1.0) * 6.5 + pct * 6.5;
            let lf_p = 152.0 + press_prog;
            let rf_p = 154.0 + press_prog;
            let lr_p = 148.0 + press_prog;
            let rr_p = 150.0 + press_prog;

            let on_pit = is_out && pct < 0.15;
            let surface = if on_pit { 2 } else { 3 };

            let mut row = writer.new_row();
            writer.set_f64(&mut row, "SessionTime", session_time);
            writer.set_i32(&mut row, "Lap", lap_num);
            writer.set_f32(&mut row, "LapDist", dist_m);
            writer.set_f32(&mut row, "LapDistPct", pct);
            writer.set_bool(&mut row, "OnPitRoad", on_pit);
            writer.set_i32(&mut row, "PlayerTrackSurface", surface);
            writer.set_f32(&mut row, "Speed", speed_mps);
            writer.set_f32(&mut row, "Throttle", throttle);
            writer.set_f32(&mut row, "Brake", brake);
            writer.set_f32(&mut row, "SteeringWheelAngle", steer_rad);
            writer.set_i32(&mut row, "Gear", gear);
            writer.set_f32(&mut row, "RPM", rpm);
            writer.set_f32(&mut row, "LFrideHeight", lf_rh);
            writer.set_f32(&mut row, "RFrideHeight", rf_rh);
            writer.set_f32(&mut row, "LRrideHeight", lr_rh);
            writer.set_f32(&mut row, "RRrideHeight", rr_rh);
            writer.set_f32(&mut row, "CFSRrideHeight", cfsr_rh);
            writer.set_f32(&mut row, "LFshockDefl", lf_defl);
            writer.set_f32(&mut row, "RFshockDefl", rf_defl);
            writer.set_f32(&mut row, "LRshockDefl", lr_defl);
            writer.set_f32(&mut row, "RRshockDefl", rr_defl);
            writer.set_f32(&mut row, "LFshockVel", lf_vel);
            writer.set_f32(&mut row, "RFshockVel", rf_vel);
            writer.set_f32(&mut row, "LRshockVel", lr_vel);
            writer.set_f32(&mut row, "RRshockVel", rr_vel);
            writer.set_f32(&mut row, "LFtempCL", lf_cl);
            writer.set_f32(&mut row, "LFtempCM", lf_cm);
            writer.set_f32(&mut row, "LFtempCR", lf_cr);
            writer.set_f32(&mut row, "RFtempCL", rf_cl);
            writer.set_f32(&mut row, "RFtempCM", rf_cm);
            writer.set_f32(&mut row, "RFtempCR", rf_cr);
            writer.set_f32(&mut row, "LRtempCL", lr_cl);
            writer.set_f32(&mut row, "LRtempCM", lr_cm);
            writer.set_f32(&mut row, "LRtempCR", lr_cr);
            writer.set_f32(&mut row, "RRtempCL", rr_cl);
            writer.set_f32(&mut row, "RRtempCM", rr_cm);
            writer.set_f32(&mut row, "RRtempCR", rr_cr);
            writer.set_f32(&mut row, "LFpressure", lf_p);
            writer.set_f32(&mut row, "RFpressure", rf_p);
            writer.set_f32(&mut row, "LRpressure", lr_p);
            writer.set_f32(&mut row, "RRpressure", rr_p);

            writer.push_row(&row);
            session_time += dt;
        }
    }

    let _ = total_ticks;
    let bytes = writer.finish(1792680000, 100.0, session_time, 3);
    parse_ibt_bytes(&bytes, "porsche963gtp_watkinsglen_demo.ibt")
}

/// Helper builder for creating valid binary `.ibt` byte buffers in tests & demo mode.
pub struct SyntheticIbtBuilder {
    tick_rate: i32,
    yaml_bytes: Vec<u8>,
    channels: Vec<IbtChannelInfo>,
    channel_lookup: HashMap<String, IbtChannelInfo>,
    buf_len: usize,
    tick_data: Vec<u8>,
    record_count: usize,
}

impl SyntheticIbtBuilder {
    pub fn new(
        tick_rate: i32,
        yaml: &str,
        defs: &[(&str, &str, &str, IbtVarType)],
    ) -> Self {
        let mut yaml_bytes = yaml.as_bytes().to_vec();
        yaml_bytes.push(0); // Null terminator

        let mut offset = 0usize;
        let mut channels = Vec::with_capacity(defs.len());
        let mut channel_lookup = HashMap::with_capacity(defs.len());

        for &(name, desc, unit, var_type) in defs {
            let sz = var_type.byte_size();
            // Align to 4-byte or 8-byte boundaries
            let align = sz.min(8);
            if align > 1 && offset % align != 0 {
                offset += align - (offset % align);
            }
            let info = IbtChannelInfo {
                name: name.to_string(),
                desc: desc.to_string(),
                unit: unit.to_string(),
                var_type,
                offset,
                count: 1,
            };
            offset += sz;
            channel_lookup.insert(name.to_string(), info.clone());
            channels.push(info);
        }

        let buf_len = offset.max(4);
        Self {
            tick_rate,
            yaml_bytes,
            channels,
            channel_lookup,
            buf_len,
            tick_data: Vec::new(),
            record_count: 0,
        }
    }

    pub fn new_row(&self) -> Vec<u8> {
        vec![0u8; self.buf_len]
    }

    pub fn set_f32(&self, row: &mut [u8], name: &str, val: f32) {
        if let Some(ch) = self.channel_lookup.get(name) {
            row[ch.offset..ch.offset + 4].copy_from_slice(&val.to_le_bytes());
        }
    }

    pub fn set_f64(&self, row: &mut [u8], name: &str, val: f64) {
        if let Some(ch) = self.channel_lookup.get(name) {
            row[ch.offset..ch.offset + 8].copy_from_slice(&val.to_le_bytes());
        }
    }

    pub fn set_i32(&self, row: &mut [u8], name: &str, val: i32) {
        if let Some(ch) = self.channel_lookup.get(name) {
            row[ch.offset..ch.offset + 4].copy_from_slice(&val.to_le_bytes());
        }
    }

    pub fn set_bool(&self, row: &mut [u8], name: &str, val: bool) {
        if let Some(ch) = self.channel_lookup.get(name) {
            row[ch.offset] = if val { 1 } else { 0 };
        }
    }

    pub fn push_row(&mut self, row: &[u8]) {
        self.tick_data.extend_from_slice(row);
        self.record_count += 1;
    }

    pub fn finish(
        self,
        start_date_unix: i64,
        start_time: f64,
        end_time: f64,
        lap_count: i32,
    ) -> Vec<u8> {
        let var_header_offset = IRSDK_HEADER_SIZE + IRSDK_DISK_SUB_HEADER_SIZE;
        let var_headers_len = self.channels.len() * IRSDK_VAR_HEADER_SIZE;
        let session_info_offset = var_header_offset + var_headers_len;
        let session_info_len = self.yaml_bytes.len();
        let buf_offset = session_info_offset + session_info_len;

        let total_size = buf_offset + self.tick_data.len();
        let mut out = vec![0u8; total_size];

        // 1. Write irsdk_header (112 bytes)
        out[0..4].copy_from_slice(&2i32.to_le_bytes()); // ver = 2
        out[4..8].copy_from_slice(&1i32.to_le_bytes()); // status = 1
        out[8..12].copy_from_slice(&self.tick_rate.to_le_bytes());
        out[12..16].copy_from_slice(&1i32.to_le_bytes()); // session_info_update = 1
        out[16..20].copy_from_slice(&(session_info_len as i32).to_le_bytes());
        out[20..24].copy_from_slice(&(session_info_offset as i32).to_le_bytes());
        out[24..28].copy_from_slice(&(self.channels.len() as i32).to_le_bytes());
        out[28..32].copy_from_slice(&(var_header_offset as i32).to_le_bytes());
        out[32..36].copy_from_slice(&1i32.to_le_bytes()); // num_buf = 1
        out[36..40].copy_from_slice(&(self.buf_len as i32).to_le_bytes());
        // varBuf[0] at offset 48
        out[48..52].copy_from_slice(&(self.record_count as i32).to_le_bytes());
        out[52..56].copy_from_slice(&(buf_offset as i32).to_le_bytes());

        // 2. Write irsdk_diskSubHeader (32 bytes at offset 112)
        out[112..120].copy_from_slice(&start_date_unix.to_le_bytes());
        out[120..128].copy_from_slice(&start_time.to_le_bytes());
        out[128..136].copy_from_slice(&end_time.to_le_bytes());
        out[136..140].copy_from_slice(&lap_count.to_le_bytes());
        out[140..144].copy_from_slice(&(self.record_count as i32).to_le_bytes());

        // 3. Write irsdk_varHeader array (144 bytes each)
        for (i, ch) in self.channels.iter().enumerate() {
            let base = var_header_offset + i * IRSDK_VAR_HEADER_SIZE;
            let vt_int = match ch.var_type {
                IbtVarType::Char => 0i32,
                IbtVarType::Bool => 1i32,
                IbtVarType::Int => 2i32,
                IbtVarType::BitField => 3i32,
                IbtVarType::Float => 4i32,
                IbtVarType::Double => 5i32,
            };
            out[base..base + 4].copy_from_slice(&vt_int.to_le_bytes());
            out[base + 4..base + 8].copy_from_slice(&(ch.offset as i32).to_le_bytes());
            out[base + 8..base + 12].copy_from_slice(&(ch.count as i32).to_le_bytes());
            write_fixed_cstr(&mut out[base + 16..base + 48], &ch.name);
            write_fixed_cstr(&mut out[base + 48..base + 112], &ch.desc);
            write_fixed_cstr(&mut out[base + 112..base + 144], &ch.unit);
        }

        // 4. Write YAML session info
        out[session_info_offset..session_info_offset + session_info_len]
            .copy_from_slice(&self.yaml_bytes);

        // 5. Write tick data
        out[buf_offset..buf_offset + self.tick_data.len()].copy_from_slice(&self.tick_data);

        out
    }
}

fn write_fixed_cstr(dst: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let max_copy = dst.len().saturating_sub(1).min(bytes.len());
    dst[..max_copy].copy_from_slice(&bytes[..max_copy]);
    dst[max_copy] = 0;
}

// ============================================================================
// 9. SELF-CONTAINED AUTOMATED VERIFICATION SUITE (SECTION 4.4)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 1. Validates zero-division handling on stagnant distance counters during X-axis normalization
    #[test]
    fn test_zero_division_handling_on_stagnant_distance_counters() {
        // Stagnant distance values (e.g. car stopped on track or identical consecutive LapDistPct ticks)
        let stagnant_dist = vec![0.0, 0.0, 0.0, 0.25, 0.25, 0.25, 0.25, 0.50, 0.50, 1.0, 1.0];
        let src_values = vec![10.0, 20.0, 30.0, 45.0, 50.0, 55.0, 60.0, 80.0, 85.0, 100.0, 105.0];
        let target_grid: Vec<f32> = (0..=100).map(|i| i as f32 / 100.0).collect();

        let resampled = interpolate_onto_grid(&stagnant_dist, &src_values, &target_grid);
        assert_eq!(resampled.len(), 101);
        for (idx, &val) in resampled.iter().enumerate() {
            assert!(
                val.is_finite(),
                "Sample at index {} must be finite (no NaN or Inf on stagnant distance), got {}",
                idx,
                val
            );
        }

        // Also test completely flat 0.0 distance across the entire array
        let all_zero_dist = vec![0.25; 20];
        let some_vals = vec![55.5; 20];
        let flat_res = interpolate_onto_grid(&all_zero_dist, &some_vals, &target_grid);
        assert_eq!(flat_res.len(), 101);
        for &val in &flat_res {
            assert!(val.is_finite());
            assert!((val - 55.5).abs() < 1e-4);
        }
    }

    /// 2. Confirms sign conventions on longitudinal rake (nose-down compression vs. rebound) & side rake
    #[test]
    fn test_rake_and_side_rake_sign_conventions() {
        // Tick 0: Nose-down compression under braking (LF=25mm, RF=25mm -> Front=25mm; LR=60mm, RR=60mm -> Rear=60mm)
        // Expected Longitudinal Rake = Rear(60) - Front(25) = +35.0 mm (Positive = tail-up / nose-down)
        // Tick 1: Nose-up under hard acceleration / crest rebound (LF=55mm, RF=55mm -> Front=55mm; LR=40mm, RR=40mm -> Rear=40mm)
        // Expected Longitudinal Rake = Rear(40) - Front(55) = -15.0 mm (Negative = nose-up / squat)
        // Tick 2: Left-turn lateral roll (right side compressed: LF=45mm, LR=55mm -> Left=50mm; RF=25mm, RR=35mm -> Right=30mm)
        // Expected Side Rake = Left(50) - Right(30) = +20.0 mm; Front Split = 45 - 25 = +20.0 mm; Rear Split = 55 - 35 = +20.0 mm
        let lf_m = Some(vec![0.025, 0.055, 0.045]);
        let rf_m = Some(vec![0.025, 0.055, 0.025]);
        let lr_m = Some(vec![0.060, 0.040, 0.055]);
        let rr_m = Some(vec![0.060, 0.040, 0.035]);
        let cfsr_m = Some(vec![0.020, 0.050, 0.030]);

        let susp = compute_suspension_dynamics(lf_m, rf_m, lr_m, rr_m, cfsr_m);

        let rake = susp.rake_mm.as_ref().expect("Rake must be computed");
        assert!(
            (rake[0] - 35.0).abs() < 1e-3,
            "Nose-down compression must yield positive rake (+35mm), got {}",
            rake[0]
        );
        assert!(
            (rake[1] - (-15.0)).abs() < 1e-3,
            "Nose-up rebound/squat must yield negative rake (-15mm), got {}",
            rake[1]
        );

        let side_rake = susp.side_rake_mm.as_ref().expect("Side rake must be computed");
        let front_split = susp.front_split_mm.as_ref().expect("Front split must be computed");
        let rear_split = susp.rear_split_mm.as_ref().expect("Rear split must be computed");

        assert!((side_rake[2] - 20.0).abs() < 1e-3);
        assert!((front_split[2] - 20.0).abs() < 1e-3);
        assert!((rear_split[2] - 20.0).abs() < 1e-3);
        assert!(susp.cfsr_rh_mm.is_some());
    }

    /// 3. Confirms total shock histogram percentages sum to 100% (± 0.01%) per corner
    ///    both for native shock velocity channels and numerically differentiated shock deflections.
    #[test]
    fn test_shock_histogram_percentages_sum_to_100_percent() {
        let session = build_demo_ibt_session().expect("Demo .ibt session must parse cleanly");
        assert!(!session.analyzed_laps.is_empty());

        for lap in &session.analyzed_laps {
            for corner in [&lap.shocks.lf, &lap.shocks.rf, &lap.shocks.lr, &lap.shocks.rr] {
                let sum_bins: f32 = corner.bins.iter().map(|b| b.time_pct).sum();
                assert!(
                    (sum_bins - 100.0).abs() <= 0.01,
                    "Corner {} shock histogram bins must sum to 100.0% (±0.01%), got {:.5}%",
                    corner.corner_name,
                    sum_bins
                );

                let sum_regimes = corner.low_speed_compression_pct
                    + corner.low_speed_rebound_pct
                    + corner.high_speed_compression_pct
                    + corner.high_speed_rebound_pct;
                assert!(
                    (sum_regimes - 100.0).abs() <= 0.01,
                    "Corner {} regime percentages must sum to 100.0% (±0.01%), got {:.5}%",
                    corner.corner_name,
                    sum_regimes
                );
            }
        }

        // Also verify numerical differentiation fallback when native *shockVel is absent
        let defl_only_m: Vec<f32> = (0..600)
            .map(|i| 0.040 + 0.008 * (i as f32 * 0.15).sin())
            .collect();
        let derived_corner = compute_corner_shock_data("LF", Some(defl_only_m), None, 60.0);
        assert_eq!(derived_corner.source, ShockVelocitySource::DerivedFromDeflection);
        let derived_sum: f32 = derived_corner.bins.iter().map(|b| b.time_pct).sum();
        assert!(
            (derived_sum - 100.0).abs() <= 0.01,
            "Derived deflection shock histogram must sum to 100.0% (±0.01%), got {:.5}%",
            derived_sum
        );
    }

    /// 4. Confirms Left vs. Right tire orientation for Inside-to-Outside temperature spread & Center bias
    #[test]
    fn test_tire_inside_outside_orientation_and_conversions() {
        // Left Front (LF): CL = Outside (80°C = 176°F), CM = Center (90°C = 194°F), CR = Inside (100°C = 212°F)
        // Inside - Outside Spread on LF = CR(212°F) - CL(176°F) = +36.0°F
        // Center Bias on LF = CM(194°F) - (212 + 176)/2 (194°F) = 0.0°F
        let lf = compute_corner_tire_analysis(
            "LF",
            true,
            TireTempSource::Carcass,
            Some(vec![80.0]),
            Some(vec![90.0]),
            Some(vec![100.0]),
            Some(vec![100.0]), // 100 kPa = 14.5038 PSI
        );
        assert!((lf.inside_stats_f.avg - 212.0).abs() < 0.01);
        assert!((lf.outside_stats_f.avg - 176.0).abs() < 0.01);
        assert!((lf.spread_stats_f.avg - 36.0).abs() < 0.01);
        assert!(lf.center_bias_stats_f.avg.abs() < 0.01);
        assert!((lf.pressure_stats_psi.avg - 14.5038).abs() < 0.01);

        // Right Front (RF): CL = Inside (100°C = 212°F), CM = Center (95°C = 203°F), CR = Outside (80°C = 176°F)
        // Inside - Outside Spread on RF = CL(212°F) - CR(176°F) = +36.0°F
        // Center Bias on RF = CM(203°F) - (212 + 176)/2 (194°F) = +9.0°F
        let rf = compute_corner_tire_analysis(
            "RF",
            false,
            TireTempSource::Carcass,
            Some(vec![100.0]),
            Some(vec![95.0]),
            Some(vec![80.0]),
            Some(vec![200.0]), // 200 kPa = 29.0076 PSI
        );
        assert!((rf.inside_stats_f.avg - 212.0).abs() < 0.01);
        assert!((rf.outside_stats_f.avg - 176.0).abs() < 0.01);
        assert!((rf.spread_stats_f.avg - 36.0).abs() < 0.01);
        assert!((rf.center_bias_stats_f.avg - 9.0).abs() < 0.01);
        assert!((rf.pressure_stats_psi.avg - 29.0076).abs() < 0.01);
    }

    /// 5. Confirms dynamic channel schema resilience when vehicle-specific channels are absent
    #[test]
    fn test_dynamic_missing_channel_schema_resilience() {
        let minimal_defs = [
            ("SessionTime", "Time", "s", IbtVarType::Double),
            ("Lap", "Lap", "", IbtVarType::Int),
            ("LapDistPct", "Pct", "%", IbtVarType::Float),
            ("Speed", "Speed", "m/s", IbtVarType::Float),
        ];
        let mut builder = SyntheticIbtBuilder::new(60, "---\nWeekendInfo:\n  TrackDisplayName: Test Oval\n...\n", &minimal_defs);
        for i in 0..120 {
            let mut row = builder.new_row();
            builder.set_f64(&mut row, "SessionTime", i as f64 / 60.0);
            builder.set_i32(&mut row, "Lap", 1);
            builder.set_f32(&mut row, "LapDistPct", i as f32 / 119.0);
            builder.set_f32(&mut row, "Speed", 50.0);
            builder.push_row(&row);
        }
        let bytes = builder.finish(1700000000, 0.0, 2.0, 1);
        let session = parse_ibt_bytes(&bytes, "minimal.ibt").expect("Minimal schema must parse without panic");

        assert_eq!(session.analyzed_laps.len(), 1);
        let lap = &session.analyzed_laps[0];
        assert!(lap.inputs.speed_mph.is_some());
        assert!(lap.inputs.throttle_pct.is_none());
        assert!(lap.suspension.rake_mm.is_none());
        assert!(lap.suspension.cfsr_rh_mm.is_none());
        assert!(!lap.shocks.has_any_shock_data);
        assert!(!lap.tires.has_temp_data);
        assert!(!lap.tires.has_pressure_data);
    }

    /// 6. Confirms flying laps with raw `PlayerTrackSurface == 3` (`OnTrack`) are marked VALID / PB ★ (not OUT LAP)
    ///    and that `CarSetup -> TiresAero -> *Tire` hot readings override frozen cold on-track carcass channels.
    #[test]
    fn test_flying_laps_surface_3_and_carsetup_hot_tire_override() {
        let yaml = r#"---
WeekendInfo:
  TrackDisplayName: Martinsville Speedway
  TrackAirTemp: 22.22 C
  TrackSurfaceTemp: 28.22 C
  TrackWindVel: 2.68 m/s
CarSetup:
  UpdateCount: 2
  TiresAero:
    LeftFrontTire:
      StartingPressure: 137.9 kPa
      LastHotPressure: 206.8 kPa
      LastTempsOMI: 95C, 102C, 110C
      TreadRemaining: 98%, 97%, 96%
    RightFrontTire:
      StartingPressure: 241.3 kPa
      LastHotPressure: 330.9 kPa
      LastTempsIMO: 120C, 112C, 104C
      TreadRemaining: 92%, 94%, 96%
    LeftRearTire:
      StartingPressure: 131.0 kPa
      LastHotPressure: 193.1 kPa
      LastTempsOMI: 90C, 96C, 101C
      TreadRemaining: 98%, 97%, 97%
    RightRearTire:
      StartingPressure: 227.5 kPa
      LastHotPressure: 310.3 kPa
      LastTempsIMO: 115C, 108C, 100C
      TreadRemaining: 93%, 95%, 97%
  Chassis:
    LeftFront:
      CornerWeight: 4200 N
...
"#;
        let defs = [
            ("SessionTime", "Time", "s", IbtVarType::Double),
            ("Lap", "Lap", "", IbtVarType::Int),
            ("LapDistPct", "Pct", "%", IbtVarType::Float),
            ("OnPitRoad", "Pit", "", IbtVarType::Bool),
            ("PlayerTrackSurface", "Surf", "", IbtVarType::Int),
            ("Speed", "Speed", "m/s", IbtVarType::Float),
            ("LFtempCL", "LF CL", "C", IbtVarType::Float),
            ("LFtempCM", "LF CM", "C", IbtVarType::Float),
            ("LFtempCR", "LF CR", "C", IbtVarType::Float),
            ("RFtempCL", "RF CL", "C", IbtVarType::Float),
            ("RFtempCM", "RF CM", "C", IbtVarType::Float),
            ("RFtempCR", "RF CR", "C", IbtVarType::Float),
            ("LRtempCL", "LR CL", "C", IbtVarType::Float),
            ("LRtempCM", "LR CM", "C", IbtVarType::Float),
            ("LRtempCR", "LR CR", "C", IbtVarType::Float),
            ("RRtempCL", "RR CL", "C", IbtVarType::Float),
            ("RRtempCM", "RR CM", "C", IbtVarType::Float),
            ("RRtempCR", "RR CR", "C", IbtVarType::Float),
            ("LFpressure", "LF P", "kPa", IbtVarType::Float),
            ("RFpressure", "RF P", "kPa", IbtVarType::Float),
            ("LRpressure", "LR P", "kPa", IbtVarType::Float),
            ("RRpressure", "RR P", "kPa", IbtVarType::Float),
        ];

        let mut builder = SyntheticIbtBuilder::new(60, yaml, &defs);
        let mut t = 10.0_f64;
        // Lap 6 (20.0s PB) and Lap 7 (21.0s VALID), both with PlayerTrackSurface == 3 (OnTrack) and OnPitRoad == false
        for (lap_no, ticks) in [(6i32, 120usize), (7i32, 126usize)] {
            for i in 0..ticks {
                let mut row = builder.new_row();
                builder.set_f64(&mut row, "SessionTime", t);
                builder.set_i32(&mut row, "Lap", lap_no);
                builder.set_f32(&mut row, "LapDistPct", i as f32 / ((ticks - 1) as f32));
                builder.set_bool(&mut row, "OnPitRoad", false);
                builder.set_i32(&mut row, "PlayerTrackSurface", 3); // 3 = irsdk_OnTrack!
                builder.set_f32(&mut row, "Speed", 45.0);
                // On-track telemetry temp channels frozen at cold starting temp (28.22 C)
                for ch in [
                    "LFtempCL", "LFtempCM", "LFtempCR", "RFtempCL", "RFtempCM", "RFtempCR",
                    "LRtempCL", "LRtempCM", "LRtempCR", "RRtempCL", "RRtempCM", "RRtempCR",
                ] {
                    builder.set_f32(&mut row, ch, 28.22);
                }
                builder.set_f32(&mut row, "LFpressure", 137.9);
                builder.set_f32(&mut row, "RFpressure", 241.3);
                builder.set_f32(&mut row, "LRpressure", 131.0);
                builder.set_f32(&mut row, "RRpressure", 227.5);
                builder.push_row(&row);
                t += 1.0 / 60.0;
            }
        }

        let bytes = builder.finish(1700000000, 10.0, t, 2);
        let sess = parse_ibt_bytes(&bytes, "martinsville_test.ibt").expect("Must parse");
        assert_eq!(sess.laps.len(), 2);
        assert!(!sess.laps[0].is_out_lap, "Flying lap with surface==3 must NOT be marked out-lap");
        assert!(sess.laps[0].is_valid_flying);
        assert!(sess.laps[0].is_personal_best);
        assert_eq!(sess.laps[0].status_badge(), "PB ★");
        assert!(!sess.laps[1].is_out_lap);
        assert!(sess.laps[1].is_valid_flying);
        assert_eq!(sess.laps[1].status_badge(), "VALID");

        // Verify CarSetup hot tire readings overrode the frozen cold on-track carcass temps
        let lap6_rf = &sess.analyzed_laps[0].tires.rf;
        assert_eq!(lap6_rf.temp_source, TireTempSource::GarageOrPitHot);
        // RF Inside = 120°C = 248.0°F, Center = 112°C = 233.6°F, Outside = 104°C = 219.2°F
        assert!((lap6_rf.inside_stats_f.avg - 248.0).abs() < 0.2);
        assert!((lap6_rf.center_stats_f.avg - 233.6).abs() < 0.2);
        assert!((lap6_rf.outside_stats_f.avg - 219.2).abs() < 0.2);
        assert!((lap6_rf.wear_inside_pct - 92.0).abs() < 0.1);
    }
}
