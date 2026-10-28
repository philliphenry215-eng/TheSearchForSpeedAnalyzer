use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::settings::get_settings_dir;
use crate::{StintData, WarRoomState};

pub const STINT_SHARE_PREFIX: &str = "S4S-STINT-V1:";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SavedStintRecord {
    pub id: String,
    pub saved_at_unix: u64,
    pub saved_at_display: String,
    pub track_name: String,
    pub track_length: String,
    pub car_name: String,
    pub session_type: String,
    pub driver_name: String,
    pub car_num: String,
    pub setup_name: String,
    pub track_state: String,
    pub air_temp: String,
    pub track_temp: String,
    pub stint: StintData,
}

impl Default for SavedStintRecord {
    fn default() -> Self {
        Self {
            id: String::new(),
            saved_at_unix: current_unix_secs(),
            saved_at_display: current_local_timestamp_display(),
            track_name: "Unknown Track".to_string(),
            track_length: "—".to_string(),
            car_name: "—".to_string(),
            session_type: "PRACTICE".to_string(),
            driver_name: "Driver".to_string(),
            car_num: "0".to_string(),
            setup_name: "Baseline".to_string(),
            track_state: "Clean Track".to_string(),
            air_temp: "—".to_string(),
            track_temp: "—".to_string(),
            stint: StintData::default(),
        }
    }
}

impl SavedStintRecord {
    pub fn short_badge_label(&self) -> String {
        let date_part = self
            .saved_at_display
            .split_whitespace()
            .next()
            .unwrap_or(&self.saved_at_display);
        let setup_clean = if !self.setup_name.is_empty()
            && self.setup_name != "—"
            && self.setup_name != "Baseline"
        {
            self.setup_name.as_str()
        } else {
            self.session_type.as_str()
        };
        if !date_part.is_empty() {
            format!("{} · {}", date_part, setup_clean)
        } else {
            setup_clean.to_string()
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct StintDatabaseFile {
    pub version: u32,
    pub records: Vec<SavedStintRecord>,
}

impl Default for StintDatabaseFile {
    fn default() -> Self {
        Self {
            version: 1,
            records: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StintDatabase {
    pub records: Vec<SavedStintRecord>,
    custom_dir: Option<PathBuf>,
}

pub fn current_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(target_os = "windows")]
pub fn current_local_timestamp_display() -> String {
    #[repr(C)]
    struct SystemTimeWin32 {
        w_year: u16,
        w_month: u16,
        w_day_of_week: u16,
        w_day: u16,
        w_hour: u16,
        w_minute: u16,
        w_second: u16,
        w_milliseconds: u16,
    }

    extern "system" {
        fn GetLocalTime(lp_system_time: *mut SystemTimeWin32);
    }

    let mut st = SystemTimeWin32 {
        w_year: 2026,
        w_month: 1,
        w_day_of_week: 0,
        w_day: 1,
        w_hour: 0,
        w_minute: 0,
        w_second: 0,
        w_milliseconds: 0,
    };
    unsafe {
        GetLocalTime(&mut st);
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        st.w_year, st.w_month, st.w_day, st.w_hour, st.w_minute
    )
}

#[cfg(not(target_os = "windows"))]
pub fn current_local_timestamp_display() -> String {
    let secs = current_unix_secs();
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    // Civil date from days since 1970-01-01
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}", year, m, d, hour, min)
}

pub fn is_placeholder_track(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.is_empty()
        || matches!(
            trimmed,
            "Waiting for iRacing..."
                | "Waiting for connection..."
                | "Connecting to Session..."
                | "iRacing Live Session"
                | "Unknown Track"
                | "—"
                | "..."
        )
}

pub fn normalize_track_key(track: &str) -> String {
    track
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn tracks_match(a: &str, b: &str) -> bool {
    if is_placeholder_track(a) || is_placeholder_track(b) {
        return false;
    }
    let ka = normalize_track_key(a);
    let kb = normalize_track_key(b);
    !ka.is_empty() && ka == kb
}

pub fn cars_match(a: &str, b: &str) -> bool {
    let ca = normalize_track_key(a);
    let cb = normalize_track_key(b);
    !ca.is_empty() && ca != "car" && ca == cb
}

fn sanitize_filename_part(raw: &str, max_len: usize) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            out.push(ch);
            prev_underscore = false;
        } else if (ch.is_whitespace() || ch == '_' || ch == '.') && !out.is_empty() && !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
        if out.len() >= max_len {
            break;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "Stint".to_string()
    } else {
        trimmed
    }
}

pub fn deterministic_live_stint_id(
    session_run_token: u64,
    session_id: i32,
    subsession_id: i32,
    session_num: i32,
    car_idx: usize,
    stint_number: i32,
) -> String {
    format!(
        "live_{}_{}_{}_{}_c{}_s{}",
        session_run_token,
        session_id.max(0),
        subsession_id.max(0),
        session_num.max(0),
        car_idx,
        stint_number.max(1)
    )
}

pub fn build_record_from_state(
    s: &WarRoomState,
    car_idx: usize,
    stint: &StintData,
) -> SavedStintRecord {
    let drv = s.drivers.get(&car_idx);
    let is_player = car_idx == s.player_car_idx;

    let driver_name = drv
        .map(|d| d.driver_name.as_str())
        .filter(|n| !n.is_empty())
        .unwrap_or(if is_player { "Player" } else { "Driver" })
        .to_string();

    let car_num = drv
        .map(|d| d.car_num.as_str())
        .filter(|n| !n.is_empty())
        .unwrap_or("0")
        .to_string();

    let setup_name = if !stint.setup_name.is_empty()
        && stint.setup_name != "—"
        && stint.setup_name != "Baseline"
    {
        stint.setup_name.clone()
    } else if let Some(d) = drv {
        if !d.setup_name.is_empty() && d.setup_name != "—" && d.setup_name != "Baseline" {
            d.setup_name.clone()
        } else if is_player
            && !s.player_setup_name.is_empty()
            && s.player_setup_name != "Baseline"
        {
            s.player_setup_name.clone()
        } else {
            "Baseline".to_string()
        }
    } else {
        "Baseline".to_string()
    };

    let track_temp = if !stint.track_temp_at_start.is_empty() && stint.track_temp_at_start != "—" {
        stint.track_temp_at_start.clone()
    } else {
        s.track_temp_raw.clone()
    };

    let air_temp = if !stint.air_temp_at_start.is_empty() && stint.air_temp_at_start != "—" {
        stint.air_temp_at_start.clone()
    } else {
        s.air_temp_raw.clone()
    };

    let record_id = if !stint.saved_id.is_empty() {
        stint.saved_id.clone()
    } else {
        deterministic_live_stint_id(
            s.session_run_token,
            s.session_id,
            s.subsession_id,
            s.session_num,
            car_idx,
            stint.stint_number,
        )
    };

    let mut clean_stint = stint.clone();
    clean_stint.is_active = false;
    clean_stint.is_imported = false;
    clean_stint.saved_id = record_id.clone();
    clean_stint.setup_name = setup_name.clone();
    if clean_stint.track_temp_at_start.is_empty() || clean_stint.track_temp_at_start == "—" {
        clean_stint.track_temp_at_start = track_temp.clone();
    }
    if clean_stint.air_temp_at_start.is_empty() || clean_stint.air_temp_at_start == "—" {
        clean_stint.air_temp_at_start = air_temp.clone();
    }

    let track_name = if is_placeholder_track(&s.track_name) {
        "Unknown Track".to_string()
    } else {
        s.track_name.clone()
    };

    let session_type = if s.session_type.is_empty() || s.session_type == "—" || s.session_type == "..." {
        "PRACTICE".to_string()
    } else {
        s.session_type.clone()
    };

    SavedStintRecord {
        id: record_id,
        saved_at_unix: current_unix_secs(),
        saved_at_display: current_local_timestamp_display(),
        track_name,
        track_length: s.track_length.clone(),
        car_name: s.car_name.clone(),
        session_type,
        driver_name,
        car_num,
        setup_name,
        track_state: s.track_state.clone(),
        air_temp,
        track_temp,
        stint: clean_stint,
    }
}

impl StintDatabase {
    pub fn base_dir(&self) -> PathBuf {
        self.custom_dir.clone().unwrap_or_else(get_settings_dir)
    }

    pub fn db_file_path(&self) -> PathBuf {
        self.base_dir().join("stint_database.json")
    }

    pub fn stints_folder_path(&self) -> PathBuf {
        self.base_dir().join("stints")
    }

    pub fn load() -> Self {
        let base = get_settings_dir();
        let path = base.join("stint_database.json");
        if !path.exists() {
            migrate_legacy_stint_db(&base);
        }
        Self::load_from_dir(base)
    }

    pub fn load_from_dir(dir: PathBuf) -> Self {
        let mut db = Self {
            records: Vec::new(),
            custom_dir: Some(dir),
        };

        let path = db.db_file_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<StintDatabaseFile>(&content) {
                db.records = parsed.records;
            } else if let Ok(records) = serde_json::from_str::<Vec<SavedStintRecord>>(&content) {
                db.records = records;
            }
        }

        // Also scan individual .s4s.json files in the stints/ folder so dropped files are picked up
        let _ = db.scan_stints_folder();
        db.sort_records();
        db
    }

    pub fn sort_records(&mut self) {
        self.records
            .sort_by(|a, b| b.saved_at_unix.cmp(&a.saved_at_unix).then_with(|| b.id.cmp(&a.id)));
    }

    pub fn save(&self) {
        let base = self.base_dir();
        let _ = fs::create_dir_all(&base);
        let path = self.db_file_path();
        let tmp_path = base.join("stint_database.json.tmp");

        let file_obj = StintDatabaseFile {
            version: 1,
            records: self.records.clone(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&file_obj) {
            if fs::write(&tmp_path, &json).is_ok() {
                if fs::rename(&tmp_path, &path).is_err() {
                    let _ = fs::write(&path, &json);
                    let _ = fs::remove_file(&tmp_path);
                }
            } else {
                let _ = fs::write(&path, &json);
            }
        }
    }

    pub fn record_file_path(&self, record: &SavedStintRecord) -> PathBuf {
        let track_part = sanitize_filename_part(&record.track_name, 26);
        let drv_part = sanitize_filename_part(&record.driver_name, 18);
        let id_part = sanitize_filename_part(&record.id, 28);
        let filename = format!(
            "{}_{}_S{}_{}.s4s.json",
            track_part,
            drv_part,
            record.stint.stint_number.max(1),
            id_part
        );
        self.stints_folder_path().join(filename)
    }

    pub fn write_individual_stint_file(&self, record: &SavedStintRecord) -> Result<PathBuf, String> {
        let folder = self.stints_folder_path();
        fs::create_dir_all(&folder)
            .map_err(|e| format!("Failed to create stints folder: {}", e))?;
        let file_path = self.record_file_path(record);
        let json = serde_json::to_string_pretty(record)
            .map_err(|e| format!("Failed to serialize stint file: {}", e))?;
        fs::write(&file_path, json)
            .map_err(|e| format!("Failed to write stint file: {}", e))?;
        Ok(file_path)
    }

    pub fn upsert_stint(&mut self, mut record: SavedStintRecord) {
        if record.id.trim().is_empty() {
            record.id = format!("stint_{}_{}", current_unix_millis(), record.stint.stint_number.max(1));
        }
        record.stint.saved_id = record.id.clone();
        record.stint.is_active = false;
        record.stint.is_imported = false;

        if let Some(existing) = self.records.iter_mut().find(|r| r.id == record.id) {
            // Preserve original timestamp when updating an existing stint in-place (e.g. tire latch or note edit)
            record.saved_at_unix = existing.saved_at_unix;
            record.saved_at_display = existing.saved_at_display.clone();
            *existing = record.clone();
        } else {
            self.records.push(record.clone());
        }

        self.sort_records();
        let _ = self.write_individual_stint_file(&record);
        self.save();
    }

    pub fn delete_stint(&mut self, id: &str) -> bool {
        let mut removed_record: Option<SavedStintRecord> = None;
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            removed_record = Some(self.records.remove(pos));
        }
        if let Some(rec) = removed_record {
            let individual_path = self.record_file_path(&rec);
            let _ = fs::remove_file(individual_path);
            // Also remove any file in stints/ whose JSON matches this id
            if let Ok(entries) = fs::read_dir(self.stints_folder_path()) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Ok(raw) = fs::read_to_string(&p) {
                            if let Ok(parsed) = serde_json::from_str::<SavedStintRecord>(&raw) {
                                if parsed.id == id {
                                    let _ = fs::remove_file(&p);
                                }
                            }
                        }
                    }
                }
            }
            self.save();
            true
        } else {
            false
        }
    }

    pub fn scan_stints_folder(&mut self) -> usize {
        let folder = self.stints_folder_path();
        let _ = fs::create_dir_all(&folder);
        let mut newly_added = 0;

        let entries = match fs::read_dir(&folder) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext_ok = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".s4s.json") || n.ends_with(".json") || n.ends_with(".s4s"))
                .unwrap_or(false);
            if !ext_ok {
                continue;
            }
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut rec) = import_from_str(&content) {
                    if rec.id.trim().is_empty() {
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("imported");
                        rec.id = format!("file_{}", sanitize_filename_part(stem, 32));
                    }
                    if !self.records.iter().any(|r| r.id == rec.id) {
                        rec.stint.saved_id = rec.id.clone();
                        rec.stint.is_active = false;
                        rec.stint.is_imported = false;
                        self.records.push(rec);
                        newly_added += 1;
                    }
                }
            }
        }

        if newly_added > 0 {
            self.sort_records();
            self.save();
        }
        newly_added
    }

    pub fn count_for_track(&self, track_name: &str) -> usize {
        if is_placeholder_track(track_name) {
            return self.records.len();
        }
        self.records
            .iter()
            .filter(|r| tracks_match(&r.track_name, track_name))
            .count()
    }

    pub fn latest_fuel_burn_for_track_and_car(&self, track_name: &str, car_name: &str) -> Option<f32> {
        if is_placeholder_track(track_name) {
            return None;
        }
        let has_car = !car_name.trim().is_empty() && car_name != "—" && car_name != "...";

        // 1. Exact Track + Car match (records are already sorted newest first)
        if has_car {
            for r in &self.records {
                if tracks_match(&r.track_name, track_name) && cars_match(&r.car_name, car_name) {
                    let rate = r.stint.fuel_per_lap();
                    if rate > 0.005 && rate < 25.0 {
                        return Some(rate);
                    }
                }
            }
        }

        // 2. Fallback to Track match if Car wasn't specified yet
        if !has_car {
            for r in &self.records {
                if tracks_match(&r.track_name, track_name) {
                    let rate = r.stint.fuel_per_lap();
                    if rate > 0.005 && rate < 25.0 {
                        return Some(rate);
                    }
                }
            }
        }

        None
    }

    pub fn sync_player_stints_from_state(&mut self, s: &WarRoomState) -> usize {
        if is_placeholder_track(&s.track_name) {
            return 0;
        }
        let player = match s.drivers.get(&s.player_car_idx) {
            Some(d) => d,
            None => return 0,
        };

        let mut updated_count = 0;
        for stint in &player.stints {
            if stint.is_imported || stint.laps_completed < 1 {
                continue;
            }
            let rec = build_record_from_state(s, s.player_car_idx, stint);
            let needs_write = match self.records.iter().find(|r| r.id == rec.id) {
                None => true,
                Some(existing) => {
                    existing.stint.laps_completed != rec.stint.laps_completed
                        || existing.stint.tires.has_data != rec.stint.tires.has_data
                        || (existing.stint.tires.lf.wear_m - rec.stint.tires.lf.wear_m).abs() > 0.05
                        || (existing.stint.tires.rf.wear_m - rec.stint.tires.rf.wear_m).abs() > 0.05
                        || (existing.stint.tires.lf.temp_m - rec.stint.tires.lf.temp_m).abs() > 0.1
                        || existing.stint.notes != rec.stint.notes
                        || existing.setup_name != rec.setup_name
                        || (existing.stint.fuel_used - rec.stint.fuel_used).abs() > 0.01
                }
            };
            if needs_write {
                self.upsert_stint(rec);
                updated_count += 1;
            }
        }

        updated_count
    }
}

pub fn export_share_string(record: &SavedStintRecord) -> Result<String, String> {
    let mut shared = record.clone();
    shared.stint.is_active = false;
    shared.stint.is_imported = false;
    let json = serde_json::to_string(&shared).map_err(|e| format!("Serialize error: {}", e))?;
    Ok(format!("{}{}", STINT_SHARE_PREFIX, json))
}

pub fn import_from_str(raw: &str) -> Result<SavedStintRecord, String> {
    let mut trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Clipboard or input is empty.".to_string());
    }

    // Strip markdown code block fences if someone pasted ```...``` from Discord
    if trimmed.starts_with("```") {
        trimmed = trimmed.trim_start_matches('`');
        if let Some(newline_pos) = trimmed.find('\n') {
            let first_line = &trimmed[..newline_pos].trim();
            if !first_line.starts_with('{') && !first_line.starts_with("S4S-") {
                trimmed = &trimmed[newline_pos + 1..];
            }
        }
        trimmed = trimmed.trim_end_matches('`').trim();
    }

    let json_payload = if let Some(idx) = trimmed.find(STINT_SHARE_PREFIX) {
        trimmed[idx + STINT_SHARE_PREFIX.len()..].trim()
    } else {
        trimmed
    };

    if let Ok(mut record) = serde_json::from_str::<SavedStintRecord>(json_payload) {
        if record.stint.laps_completed <= 0 && !record.stint.lap_times.is_empty() {
            record.stint.laps_completed = record.stint.lap_times.len() as i32;
        }
        if record.stint.laps_completed <= 0 && record.stint.lap_times.is_empty() {
            return Err("Stint data contains 0 completed laps.".to_string());
        }
        if record.id.trim().is_empty() {
            record.id = format!(
                "imp_{}_{}",
                current_unix_millis(),
                record.stint.stint_number.max(1)
            );
        }
        record.stint.saved_id = record.id.clone();
        record.stint.is_active = false;
        record.stint.is_imported = false;
        return Ok(record);
    }

    if let Ok(mut stint) = serde_json::from_str::<StintData>(json_payload) {
        if stint.laps_completed <= 0 && !stint.lap_times.is_empty() {
            stint.laps_completed = stint.lap_times.len() as i32;
        }
        if stint.laps_completed <= 0 {
            return Err("Stint data contains 0 completed laps.".to_string());
        }
        let id = format!("imp_{}_{}", current_unix_millis(), stint.stint_number.max(1));
        stint.saved_id = id.clone();
        stint.is_active = false;
        stint.is_imported = false;
        return Ok(SavedStintRecord {
            id,
            saved_at_unix: current_unix_secs(),
            saved_at_display: current_local_timestamp_display(),
            track_name: "Imported Stint".to_string(),
            setup_name: stint.setup_name.clone(),
            air_temp: stint.air_temp_at_start.clone(),
            track_temp: stint.track_temp_at_start.clone(),
            stint,
            ..Default::default()
        });
    }

    Err("Could not parse stint data. Copy a Stint Share Code (S4S-STINT-V1:...) or .s4s.json file.".to_string())
}

pub fn read_clipboard_text() -> Result<String, String> {
    clipboard_win::get_clipboard_string()
        .map_err(|e| format!("Could not read text from clipboard: {}", e))
}

/// Copies the physical .s4s.json stint file to the Windows Clipboard (CF_HDROP).
/// When pasted (Ctrl+V) into Discord, Slack, Teams, or Windows Explorer, it attaches/copies
/// the actual file rather than raw JSON text.
pub fn copy_stint_file_to_clipboard(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Stint file does not exist: {}", path.display()));
    }
    use clipboard_win::Setter;
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = abs_path.to_string_lossy();
    // Windows Explorer does not handle \\?\ extended UNC path prefixes when pasting from clipboard
    let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str).to_string();

    let _clip = clipboard_win::Clipboard::new_attempts(10)
        .map_err(|e| format!("Could not open Windows clipboard: {}", e))?;
    let _ = clipboard_win::raw::empty();

    let paths = [clean_path.as_str()];
    clipboard_win::formats::FileList
        .write_clipboard(&paths)
        .map_err(|e| format!("Failed to copy file to clipboard: {}", e))?;

    Ok(())
}

/// Reads file paths from the clipboard if the user copied files in Windows Explorer or another app.
pub fn read_clipboard_file_paths() -> Vec<PathBuf> {
    use clipboard_win::Getter;
    let mut files = Vec::<String>::new();
    if let Ok(_clip) = clipboard_win::Clipboard::new_attempts(5) {
        let _ = clipboard_win::formats::FileList.read_clipboard(&mut files);
    }
    files
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".s4s.json") || n.ends_with(".json") || n.ends_with(".s4s"))
                    .unwrap_or(false)
        })
        .collect()
}

/// Reveals a file in Windows Explorer with the specific file selected/highlighted,
/// making it effortless to drag-and-drop into Discord or attach to messages.
pub fn reveal_file_in_explorer(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        if path.exists() {
            let _ = std::process::Command::new("explorer.exe")
                .arg(format!("/select,{}", path.display()))
                .spawn();
        } else if let Some(parent) = path.parent() {
            let _ = std::process::Command::new("explorer.exe")
                .arg(parent)
                .spawn();
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Some(parent) = path.parent() {
            open_folder_in_explorer(parent);
        }
    }
}

/// Reads and parses an individual stint file from disk.
pub fn import_stint_from_file(path: &Path) -> Result<SavedStintRecord, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Could not read file '{}': {}", path.display(), e))?;
    let mut rec = import_from_str(&content)?;
    if rec.id.trim().is_empty() {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported");
        rec.id = format!("file_{}", sanitize_filename_part(stem, 32));
        rec.stint.saved_id = rec.id.clone();
    }
    Ok(rec)
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[allow(non_snake_case)]
struct OPENFILENAMEW {
    lStructSize: u32,
    hwndOwner: *mut std::ffi::c_void,
    hInstance: *mut std::ffi::c_void,
    lpstrFilter: *const u16,
    lpstrCustomFilter: *mut u16,
    nMaxCustFilter: u32,
    nFilterIndex: u32,
    lpstrFile: *mut u16,
    nMaxFile: u32,
    lpstrFileTitle: *mut u16,
    nMaxFileTitle: u32,
    lpstrInitialDir: *const u16,
    lpstrTitle: *const u16,
    Flags: u32,
    nFileOffset: u16,
    nFileExtension: u16,
    lpstrDefExt: *const u16,
    lCustData: usize,
    lpfnHook: *mut std::ffi::c_void,
    lpTemplateName: *const u16,
    pvReserved: *mut std::ffi::c_void,
    dwReserved: u32,
    FlagsEx: u32,
}

#[cfg(target_os = "windows")]
#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
}

/// Opens native Windows file dialog to select an existing .s4s.json stint file for importing.
pub fn pick_open_stint_file_dialog() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter: Vec<u16> = "Search For Speed Stint Files (*.s4s.json;*.json)\0*.s4s.json;*.json\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "Select Stint File to Import (.s4s.json)\0"
            .encode_utf16()
            .collect();

        // Default initial directory: prioritize Downloads (where Discord/browser downloads land) or stints folder
        let init_dir = std::env::var_os("USERPROFILE")
            .map(|u| PathBuf::from(u).join("Downloads"))
            .filter(|p| p.exists())
            .unwrap_or_else(|| get_settings_dir().join("stints"));
        let init_dir_w: Vec<u16> = init_dir
            .to_string_lossy()
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut buf = [0u16; 1024];
        let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        ofn.lpstrInitialDir = init_dir_w.as_ptr();
        ofn.lpstrTitle = title.as_ptr();
        // OFN_FILEMUSTEXIST (0x1000) | OFN_PATHMUSTEXIST (0x800) | OFN_EXPLORER (0x80000) | OFN_NOCHANGEDIR (0x8)
        ofn.Flags = 0x00081808;

        let res = unsafe { GetOpenFileNameW(&mut ofn) };
        if res != 0 {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let path_str = String::from_utf16_lossy(&buf[..len]);
            let p = PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Opens native Windows save dialog to export a stint file to a chosen location.
pub fn pick_save_stint_file_dialog(default_filename: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter: Vec<u16> = "Search For Speed Stint Files (*.s4s.json)\0*.s4s.json\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "Export Stint File As (.s4s.json)\0"
            .encode_utf16()
            .collect();
        let def_ext: Vec<u16> = "s4s.json\0".encode_utf16().collect();

        let mut buf = [0u16; 1024];
        let def_name_w: Vec<u16> = default_filename.encode_utf16().collect();
        let copy_len = def_name_w.len().min(buf.len() - 1);
        buf[..copy_len].copy_from_slice(&def_name_w[..copy_len]);

        let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        ofn.lpstrTitle = title.as_ptr();
        ofn.lpstrDefExt = def_ext.as_ptr();
        // OFN_OVERWRITEPROMPT (0x2) | OFN_PATHMUSTEXIST (0x800) | OFN_EXPLORER (0x80000) | OFN_NOCHANGEDIR (0x8)
        ofn.Flags = 0x0008080A;

        let res = unsafe { GetSaveFileNameW(&mut ofn) };
        if res != 0 {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let path_str = String::from_utf16_lossy(&buf[..len]);
            return Some(PathBuf::from(path_str));
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

pub fn open_folder_in_explorer(path: &Path) {
    let _ = fs::create_dir_all(path);
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
    }
}

pub fn auto_save_session_player_stints(s: &WarRoomState) {
    if is_placeholder_track(&s.track_name) {
        return;
    }
    let has_valid_player_stint = s
        .drivers
        .get(&s.player_car_idx)
        .map_or(false, |d| d.stints.iter().any(|st| !st.is_imported && st.laps_completed >= 1));
    if !has_valid_player_stint {
        return;
    }
    let mut db = StintDatabase::load();
    let _ = db.sync_player_stints_from_state(s);
}

/// Migrates existing stint_database.json and stints/ folder from legacy locations
/// (%APPDATA%\SearchForSpeedAnalyzer, next to exe, or CWD) to the new target folder (e.g. Documents).
fn migrate_legacy_stint_db(target_base: &Path) {
    let mut candidate_dirs: Vec<PathBuf> = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA").filter(|v| !v.is_empty()) {
        candidate_dirs.push(PathBuf::from(appdata).join("SearchForSpeedAnalyzer"));
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        candidate_dirs.push(exe_dir);
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidate_dirs.push(cwd);
    }

    for cand in candidate_dirs {
        if cand == target_base {
            continue;
        }
        let legacy_db = cand.join("stint_database.json");
        let target_db = target_base.join("stint_database.json");
        if legacy_db.is_file() && !target_db.exists() {
            let _ = fs::create_dir_all(target_base);
            let _ = fs::copy(&legacy_db, &target_db);
        }

        let legacy_stints = cand.join("stints");
        let target_stints = target_base.join("stints");
        if legacy_stints.is_dir() {
            let _ = fs::create_dir_all(&target_stints);
            if let Ok(entries) = fs::read_dir(&legacy_stints) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Some(fname) = path.file_name() {
                            let dest = target_stints.join(fname);
                            if !dest.exists() {
                                let _ = fs::copy(&path, &dest);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StintTireSet, TireCornerData};

    #[test]
    fn test_stint_file_import_export_clipboard() {
        let temp_dir = std::env::temp_dir().join(format!("s4s_file_test_{}", current_unix_millis()));
        let _ = fs::create_dir_all(&temp_dir);

        let mut stint = StintData::default();
        stint.stint_number = 2;
        stint.laps_completed = 10;
        stint.lap_times = vec![85.200; 10];
        stint.fast_lap = 84.800;

        let rec = SavedStintRecord {
            id: "file_test_s2".to_string(),
            saved_at_unix: 1700000000,
            saved_at_display: "2026-10-27 19:30".to_string(),
            track_name: "Watkins Glen".to_string(),
            driver_name: "Peebs".to_string(),
            setup_name: "Race_v1".to_string(),
            stint,
            ..Default::default()
        };

        let file_path = temp_dir.join("Watkins_Glen_Peebs_S2_file_test_s2.s4s.json");
        let json = serde_json::to_string_pretty(&rec).expect("serialize");
        fs::write(&file_path, json).expect("write file");

        // Verify import from file
        let imported = import_stint_from_file(&file_path).expect("import from file");
        assert_eq!(imported.track_name, "Watkins Glen");
        assert_eq!(imported.stint.laps_completed, 10);
        assert_eq!(imported.stint.fast_lap, 84.800);

        // Verify copying file to Windows clipboard (when window station/clipboard is available in session)
        if let Ok(()) = copy_stint_file_to_clipboard(&file_path) {
            let clip_files = read_clipboard_file_paths();
            assert!(
                clip_files.iter().any(|p| p.file_name() == file_path.file_name()),
                "Clipboard should contain the copied file: {:?}",
                clip_files
            );
        }

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_track_matching_and_normalization() {
        assert!(tracks_match(
            "Daytona International Speedway",
            "  daytona international speedway "
        ));
        assert!(tracks_match(
            "Bristol Motor Speedway - Dirt",
            "Bristol Motor Speedway Dirt"
        ));
        assert!(!tracks_match("Waiting for iRacing...", "Waiting for iRacing..."));
        assert!(!tracks_match("Daytona", "Talladega"));
    }

    #[test]
    fn test_stint_db_upsert_query_and_share_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("s4s_stint_db_test_{}", current_unix_millis()));
        let _ = fs::remove_dir_all(&temp_dir);

        let mut db = StintDatabase::load_from_dir(temp_dir.clone());
        assert_eq!(db.records.len(), 0);

        let mut stint = StintData::default();
        stint.stint_number = 1;
        stint.laps_completed = 15;
        stint.lap_times = vec![15.200; 15];
        stint.fast_lap = 15.100;
        stint.last_lap = 15.250;
        stint.fuel_used = 7.5;
        stint.latched_fuel_burn_rate = 0.500;
        stint.setup_name = "Bristol_Race_v2".to_string();
        stint.tires = StintTireSet {
            has_data: true,
            lf: TireCornerData {
                temp_l: 85.0,
                temp_m: 82.0,
                temp_r: 79.0,
                wear_l: 92.0,
                wear_m: 94.0,
                wear_r: 96.0,
                pressure: 172.0,
            },
            ..Default::default()
        };

        let rec = SavedStintRecord {
            id: "test_bristol_s1".to_string(),
            saved_at_unix: 1700000000,
            saved_at_display: "2026-10-24 16:30".to_string(),
            track_name: "Bristol Motor Speedway".to_string(),
            track_length: "0.533 mi".to_string(),
            car_name: "NASCAR Cup Series Next Gen".to_string(),
            session_type: "PRACTICE".to_string(),
            driver_name: "Test Driver".to_string(),
            car_num: "24".to_string(),
            setup_name: "Bristol_Race_v2".to_string(),
            track_state: "Moderate Rubber".to_string(),
            air_temp: "72 F".to_string(),
            track_temp: "88 F".to_string(),
            stint,
        };

        db.upsert_stint(rec.clone());
        assert_eq!(db.records.len(), 1);
        assert_eq!(db.count_for_track("bristol motor speedway"), 1);

        // Verify historical fuel burn lookup
        let burn = db
            .latest_fuel_burn_for_track_and_car("Bristol Motor Speedway", "NASCAR Cup Series Next Gen")
            .expect("should find fuel burn");
        assert!((burn - 0.500).abs() < 0.001);

        // Verify updating in-place (e.g., notes edited) keeps count at 1
        let mut updated_rec = rec.clone();
        updated_rec.stint.notes = "Dropped RR cold pressure 0.5 psi".to_string();
        db.upsert_stint(updated_rec);
        assert_eq!(db.records.len(), 1);
        assert_eq!(
            db.records[0].stint.notes,
            "Dropped RR cold pressure 0.5 psi"
        );

        // Verify reload from disk
        let reloaded = StintDatabase::load_from_dir(temp_dir.clone());
        assert_eq!(reloaded.records.len(), 1);
        assert_eq!(reloaded.records[0].id, "test_bristol_s1");

        // Verify Discord share string export & import (including code-block wrapping)
        let share_str = export_share_string(&reloaded.records[0]).expect("export share code");
        let wrapped = format!("```\n{}\n```", share_str);
        let imported = import_from_str(&wrapped).expect("import share code");
        assert_eq!(imported.track_name, "Bristol Motor Speedway");
        assert_eq!(imported.stint.laps_completed, 15);
        assert!(imported.stint.tires.has_data);

        // Delete stint
        assert!(db.delete_stint("test_bristol_s1"));
        assert_eq!(db.records.len(), 0);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_legacy_stint_migration() {
        let root = std::env::temp_dir().join(format!("s4s_test_mig_{}", std::process::id()));
        let legacy = root.join("legacy");
        let target = root.join("target");
        let _ = fs::create_dir_all(&legacy);
        let _ = fs::create_dir_all(&target);

        let legacy_file = legacy.join("stint_database.json");
        let dummy = r#"{"version":1,"records":[]}"#;
        let _ = fs::write(&legacy_file, dummy);

        let legacy_stints = legacy.join("stints");
        let _ = fs::create_dir_all(&legacy_stints);
        let _ = fs::write(legacy_stints.join("track_test_S1.s4s.json"), r#"{"id":"test"}"#);

        let cand = vec![legacy.clone()];
        for c in cand {
            let l_db = c.join("stint_database.json");
            let t_db = target.join("stint_database.json");
            if l_db.is_file() && !t_db.exists() {
                let _ = fs::copy(&l_db, &t_db);
            }
            let l_stints = c.join("stints");
            let t_stints = target.join("stints");
            if l_stints.is_dir() {
                let _ = fs::create_dir_all(&t_stints);
                if let Ok(entries) = fs::read_dir(&l_stints) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(fname) = path.file_name() {
                            let _ = fs::copy(&path, t_stints.join(fname));
                        }
                    }
                }
            }
        }

        assert!(target.join("stint_database.json").exists());
        assert!(target.join("stints").join("track_test_S1.s4s.json").exists());
        let _ = fs::remove_dir_all(&root);
    }
}
