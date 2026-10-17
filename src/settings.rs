use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActiveViewMode {
    CockpitEngineer, // 3-Column Boxy High-End Stint & Fuel Analyzer
    GridTable,       // Full Multi-Driver Stint Grid
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
fn default_margin_laps() -> f32 { 1.0 }
fn default_opacity() -> f32 { 0.88 }

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
    pub show_weather: bool,
    pub show_fastest: bool,
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
    S4SGoldBlue,     // Pitch black + Goldish Bronze + Racing Blue
    MatteSageGreen,  // Dark mode + Matte Sage Green accents
    StealthDark,     // Pure monochrome dark + icy white/silver
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
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let file_path = get_settings_file();
        if Path::new(file_path).exists() {
            if let Ok(data) = fs::read_to_string(file_path) {
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
            let _ = fs::write(get_settings_file(), json);
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

    pub fn primary_accent(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(255, 212, 64), // High-Luminance Vibrant Gold
            ThemePreset::MatteSageGreen => Color32::from_rgb(145, 215, 165), // Crisp Matte Sage Mint
            ThemePreset::StealthDark => Color32::from_rgb(255, 255, 255), // Pure Crisp White
        }
    }

    pub fn secondary_accent(&self) -> Color32 {
        match self.theme {
            ThemePreset::S4SGoldBlue => Color32::from_rgb(45, 115, 220), // Electric Racing Blue
            ThemePreset::MatteSageGreen => Color32::from_rgb(55, 135, 95), // Deep Forest Accent
            ThemePreset::StealthDark => Color32::from_rgb(90, 100, 115), // Steel Slate
        }
    }

    pub fn bg_color(&self) -> Color32 {
        Color32::from_rgb(11, 13, 18) // Deep Obsidian Canvas
    }

    pub fn panel_color(&self) -> Color32 {
        Color32::from_rgb(19, 22, 29) // Sleek Elevated Card Surface
    }

    pub fn card_elevated_color(&self) -> Color32 {
        Color32::from_rgb(26, 30, 40) // Nested Tile / Pill Surface
    }

    pub fn text_color(&self) -> Color32 {
        Color32::from_rgb(236, 240, 246) // Soft High-Contrast White
    }

    pub fn text_muted(&self) -> Color32 {
        Color32::from_rgb(139, 147, 163) // Muted Label Slate
    }

    pub fn divider_color(&self) -> Color32 {
        Color32::from_rgb(32, 38, 50) // Subtle Hairline Border
    }

    pub fn status_green(&self) -> Color32 {
        Color32::from_rgb(75, 225, 110)
    }

    pub fn status_amber(&self) -> Color32 {
        Color32::from_rgb(255, 165, 50)
    }

    pub fn status_red(&self) -> Color32 {
        Color32::from_rgb(255, 75, 75)
    }

    pub fn status_purple(&self) -> Color32 {
        Color32::from_rgb(185, 120, 255)
    }
}

