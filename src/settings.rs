use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use eframe::egui::Color32;

pub fn is_beta_mode() -> bool {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(name) = exe_path.file_name().and_then(|s| s.to_str()) {
            let lower = name.to_lowercase();
            if lower.contains("beta") || lower.contains("staging") {
                return true;
            }
        }
    }
    std::env::args().any(|arg| arg == "--beta" || arg == "-beta")
}

pub fn get_settings_file() -> &'static str {
    if is_beta_mode() {
        "settings-beta.json"
    } else {
        "settings.json"
    }
}

/// Per-user settings folder: `%APPDATA%\SearchForSpeedAnalyzer`.
/// Always writable without admin, and independent of where the exe lives or is launched from.
/// Falls back to the exe's folder if `%APPDATA%` is unavailable.
pub fn get_settings_dir() -> PathBuf {
    if let Some(appdata) = std::env::var_os("APPDATA").filter(|v| !v.is_empty()) {
        return PathBuf::from(appdata).join("SearchForSpeedAnalyzer");
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn get_settings_path() -> PathBuf {
    get_settings_dir().join(get_settings_file())
}

/// Older versions saved settings relative to the working directory (usually next to the exe).
/// Copy the first one found into the new per-user location so users keep their setup.
fn migrate_legacy_settings(target: &Path) {
    if target.exists() {
        return;
    }
    let file = get_settings_file();
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        candidates.push(exe_dir.join(file));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(file));
    }
    if let Some(legacy) = candidates.into_iter().find(|c| c.is_file() && c != target) {
        if let Some(dir) = target.parent() {
            let _ = fs::create_dir_all(dir);
        }
        let _ = fs::copy(&legacy, target);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveViewMode {
    CockpitEngineer, // 3-Column Boxy High-End Stint & Fuel Analyzer
    GridTable,       // Full Multi-Driver Stint Grid
    IbtTelemetry,    // Offline .ibt Telemetry Ingestion & Engineering Lab
}

impl Default for ActiveViewMode {
    fn default() -> Self {
        Self::CockpitEngineer
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayShape {
    Square,     // Default HUD widget (compact box)
    Horizontal, // Streamlined banner for top/bottom dash
    Vertical,   // Tower strip for screen edge / ultrawide
}

impl Default for OverlayShape {
    fn default() -> Self {
        Self::Square
    }
}

fn default_true() -> bool { true }
fn default_false() -> bool { false }
fn default_margin_laps() -> f32 { 1.0 }
fn default_opacity() -> f32 { 0.88 }
fn default_hud_scale() -> f32 { 1.0 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FuelHeroMetric {
    FuelAmount,  // Fuel level in tank (gallons / liters) is primary
    LapsToEmpty, // Laps remaining is primary
}

impl Default for FuelHeroMetric {
    fn default() -> Self {
        Self::FuelAmount
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSettings {
    #[serde(default = "default_true")]
    pub check_on_startup: bool,
    #[serde(default = "default_false")]
    pub include_prereleases: bool,
    #[serde(default)]
    pub ignored_version: Option<String>,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            check_on_startup: true,
            include_prereleases: is_beta_mode(),
            ignored_version: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlaySettings {
    pub enabled: bool,
    pub shape: OverlayShape,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_true")]
    pub always_on_top: bool,
    #[serde(default = "default_margin_laps")]
    pub safety_margin_laps: f32,
    #[serde(default = "default_true")]
    pub show_pace_card: bool,
    #[serde(default = "default_true")]
    pub show_fuel_card: bool,
    #[serde(default = "default_true")]
    pub show_race_control_card: bool,
    #[serde(default)]
    pub fuel_hero_metric: FuelHeroMetric,
    #[serde(default = "default_hud_scale")]
    pub hud_scale: f32,
    #[serde(default = "default_true")]
    pub show_weather: bool,
    #[serde(default = "default_true")]
    pub show_fastest: bool,
    #[serde(default = "default_true")]
    pub show_averages: bool,
    #[serde(default)]
    pub position: Option<[f32; 2]>,
}

impl Default for OverlaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            shape: OverlayShape::Square,
            opacity: 0.88,
            always_on_top: true,
            safety_margin_laps: 1.0,
            show_pace_card: true,
            show_fuel_card: true,
            show_race_control_card: true,
            fuel_hero_metric: FuelHeroMetric::FuelAmount,
            hud_scale: 1.0,
            show_weather: true,
            show_fastest: true,
            show_averages: true,
            position: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnVisibility {
    pub pos: bool,
    pub car_num: bool,
    pub driver: bool,
    pub irating: bool,
    pub stint: bool,
    pub laps: bool,
    pub last_lap: bool,
    pub fastest: bool,
    pub cur_avg: bool,
    pub five_lap_avg: bool,
    pub ten_lap_avg: bool,
    pub setup_name: bool,
}

impl Default for ColumnVisibility {
    fn default() -> Self {
        Self {
            pos: true,
            car_num: true,
            driver: true,
            irating: true,
            stint: true,
            laps: true,
            last_lap: true,
            fastest: true,
            cur_avg: true,
            five_lap_avg: true,
            ten_lap_avg: true,
            setup_name: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreset {
    #[serde(alias = "MatteSageGreen")]
    S4SGoldBlue,      // Pitch black + Vibrant Gold + Electric Racing Blue
    StealthDark,      // Pure monochrome dark + icy white/silver
    S4SGoldBlueLight, // Inverted: Daylight Crisp White/Silver + Royal Racing Blue + Warm Gold
}

impl ThemePreset {
    pub fn is_dark(&self) -> bool {
        match self {
            ThemePreset::S4SGoldBlue | ThemePreset::StealthDark => true,
            ThemePreset::S4SGoldBlueLight => false,
        }
    }
}

impl Default for ThemePreset {
    fn default() -> Self {
        Self::S4SGoldBlue
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayoutPreset {
    FullStint,
    EnduranceEngineer,
    SprintQualifying,
    Minimalist,
    Custom,
}

impl Default for LayoutPreset {
    fn default() -> Self {
        Self::FullStint
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub view_mode: ActiveViewMode,
    pub theme: ThemePreset,
    pub use_metric: bool,
    pub layout_preset: LayoutPreset,
    pub columns: ColumnVisibility,
    pub overlay: OverlaySettings,
    pub font_size_offset: f32,
    #[serde(default = "default_custom_avg_buckets")]
    pub custom_avg_buckets: Vec<usize>,
    #[serde(default = "default_active_avg_buckets")]
    pub active_avg_buckets: Vec<usize>,
    #[serde(default)]
    pub updates: UpdateSettings,
}

fn default_custom_avg_buckets() -> Vec<usize> {
    vec![5, 10, 15, 20, 25, 30, 35, 40, 50, 60]
}

fn default_active_avg_buckets() -> Vec<usize> {
    vec![5, 10, 15, 20, 25, 30]
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            view_mode: ActiveViewMode::CockpitEngineer,
            theme: ThemePreset::S4SGoldBlue,
            use_metric: false,
            layout_preset: LayoutPreset::FullStint,
            columns: ColumnVisibility::default(),
            overlay: OverlaySettings::default(),
            font_size_offset: 0.0,
            custom_avg_buckets: default_custom_avg_buckets(),
            active_avg_buckets: default_active_avg_buckets(),
            updates: UpdateSettings::default(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let file_path = get_settings_path();
        migrate_legacy_settings(&file_path);
        if file_path.exists() {
            if let Ok(data) = fs::read_to_string(&file_path) {
                if let Ok(mut settings) = serde_json::from_str::<AppSettings>(&data) {
                    if settings.custom_avg_buckets.is_empty() {
                        settings.custom_avg_buckets = default_custom_avg_buckets();
                    }
                    if settings.active_avg_buckets.is_empty() {
                        settings.active_avg_buckets = default_active_avg_buckets();
                    }
                    return settings;
                }
            }
        }
        let default_settings = Self::default();
        default_settings.save();
        default_settings
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let path = get_settings_path();
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(path, json);
        }
    }

    pub fn apply_preset(&mut self, preset: LayoutPreset) {
        self.layout_preset = preset;
        match preset {
            LayoutPreset::FullStint => {
                self.columns = ColumnVisibility {
                    pos: true,
                    car_num: true,
                    driver: true,
                    irating: true,
                    stint: true,
                    laps: true,
                    last_lap: true,
                    fastest: true,
                    cur_avg: true,
                    five_lap_avg: true,
                    ten_lap_avg: false,
                    setup_name: false,
                };
            }
            LayoutPreset::EnduranceEngineer => {
                self.columns = ColumnVisibility {
                    pos: true,
                    car_num: true,
                    driver: true,
                    irating: false,
                    stint: true,
                    laps: true,
                    last_lap: true,
                    fastest: true,
                    cur_avg: true,
                    five_lap_avg: true,
                    ten_lap_avg: true,
                    setup_name: true,
                };
            }
            LayoutPreset::SprintQualifying => {
                self.columns = ColumnVisibility {
                    pos: true,
                    car_num: true,
                    driver: true,
                    irating: true,
                    stint: false,
                    laps: true,
                    last_lap: true,
                    fastest: true,
                    cur_avg: false,
                    five_lap_avg: false,
                    ten_lap_avg: false,
                    setup_name: false,
                };
            }
            LayoutPreset::Minimalist => {
                self.columns = ColumnVisibility {
                    pos: true,
                    car_num: true,
                    driver: true,
                    irating: false,
                    stint: true,
                    laps: true,
                    last_lap: true,
                    fastest: false,
                    cur_avg: false,
                    five_lap_avg: false,
                    ten_lap_avg: false,
                    setup_name: false,
                };
            }
            LayoutPreset::Custom => {}
        }
        self.save();
    }

    pub fn is_dark_theme(&self) -> bool {
        self.theme.is_dark()
    }

    pub fn primary_accent(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(255, 212, 64), // Vibrant Gold
            ThemePreset::StealthDark => Color32::from_rgb(255, 255, 255), // Pure Crisp White
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(212, 142, 12), // Deep Warm Amber Gold (high contrast on daylight)
        }
    }

    pub fn secondary_accent(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(45, 115, 220), // Electric Racing Blue
            ThemePreset::StealthDark => Color32::from_rgb(110, 120, 138), // Steel Slate
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(25, 95, 215), // Vibrant Royal Racing Blue
        }
    }

    pub fn bg_color(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(11, 13, 18), // Deep Obsidian Canvas
            ThemePreset::StealthDark => Color32::from_rgb(10, 11, 14), // Pitch Black Canvas
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(238, 241, 246), // Crisp Light Canvas (Daylight Matte)
        }
    }

    pub fn panel_color(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(19, 22, 29), // Sleek Elevated Card Surface
            ThemePreset::StealthDark => Color32::from_rgb(18, 20, 25), // Dark Carbon Surface
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(255, 255, 255), // Pure Crisp White Card Surface
        }
    }

    pub fn card_elevated_color(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(26, 30, 40), // Nested Tile / Pill Surface
            ThemePreset::StealthDark => Color32::from_rgb(25, 28, 36), // Gunmetal Tile Surface
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(228, 233, 242), // Soft Cool Tinted Pill Surface
        }
    }

    pub fn text_color(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(236, 240, 246), // Soft High-Contrast White
            ThemePreset::StealthDark => Color32::from_rgb(245, 247, 250), // High-Visibility White
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(16, 22, 34), // Deep Navy Charcoal Text
        }
    }

    pub fn text_muted(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(139, 147, 163), // Muted Label Slate
            ThemePreset::StealthDark => Color32::from_rgb(130, 138, 150), // Cool Silver Muted
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(95, 108, 128), // Refined Slate Muted
        }
    }

    pub fn divider_color(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(32, 38, 50), // Subtle Hairline Border
            ThemePreset::StealthDark => Color32::from_rgb(32, 35, 45), // Monochrome Border
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(212, 220, 232), // Clean Subtle Light Border
        }
    }

    pub fn status_green(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(18, 150, 60),
            _ => Color32::from_rgb(75, 225, 110),
        }
    }

    pub fn status_amber(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(215, 125, 15),
            _ => Color32::from_rgb(255, 165, 50),
        }
    }

    pub fn status_red(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(215, 45, 45),
            _ => Color32::from_rgb(255, 75, 75),
        }
    }

    pub fn status_purple(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlueLight => Color32::from_rgb(130, 50, 205),
            _ => Color32::from_rgb(185, 120, 255),
        }
    }
}

