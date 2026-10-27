use std::collections::HashMap;
use crate::stint_db;
use iracing::Connection;
use iracing::telemetry::Value;
use std::thread;
use crate::models::*;

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

pub fn is_real_track_name(name: &str) -> bool {
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
        || s.drivers.values().any(|d| d.has_live_stints() || d.total_laps_completed > 0)
        || s.session_id != 0
        || s.subsession_id != 0;

    let track_changed = is_real_track_name(&s.track_name)
        && is_real_track_name(new_track_name)
        && !stint_db::tracks_match(&s.track_name, new_track_name);

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

        let cur_sess = if s.session_num >= 0 {
            sessions.iter().find(|sess| sess["SessionNum"].as_i64().unwrap_or(-1) as i32 == s.session_num)
        } else {
            sessions.first()
        };
        if let Some(sess) = cur_sess {
            if s.session_laps_total == 0 {
                if let Some(laps_val) = sess["SessionLaps"].as_i64() {
                    if laps_val > 0 && laps_val < 10000 {
                        s.session_laps_total = laps_val as i32;
                    }
                } else if let Some(laps_str) = sess["SessionLaps"].as_str() {
                    if let Ok(laps_val) = laps_str.trim().parse::<i32>() {
                        if laps_val > 0 && laps_val < 10000 {
                            s.session_laps_total = laps_val;
                        }
                    }
                }
            }
        }
    }

    // 3. Driver Info
    let driver_info = &root["DriverInfo"];
    let prev_player_idx = s.player_car_idx;
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

    // Preserve any re-imported stints if player_car_idx changed when attaching to a session at the same track
    let mut carried_imported_stints: Vec<StintData> = Vec::new();
    if prev_player_idx != player_idx && !track_changed {
        if let Some(prev_drv) = s.drivers.get_mut(&prev_player_idx) {
            for st in &prev_drv.stints {
                if st.is_imported {
                    carried_imported_stints.push(st.clone());
                }
            }
            prev_drv.stints.retain(|st| !st.is_imported);
        }
    }

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
                // If a different real driver took over this CarIdx slot, reset their live stints/laps
                let had_real_name = !target.driver_name.is_empty()
                    && target.driver_name != "Player Car"
                    && !target.driver_name.starts_with("Car #");
                if had_real_name && !user_name.is_empty() && target.driver_name != user_name {
                    target.stints.retain(|st| st.is_imported);
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
                pending_lap_count_since: 0.0,
                lap_time_ahead_since: 0.0,
            });
        }

        if !active_yaml_indices.is_empty() {
            s.drivers.retain(|idx, _| active_yaml_indices.contains(idx));
        }
    }

    // Re-attach any carried imported stints to the active player driver slot
    if !carried_imported_stints.is_empty() {
        if let Some(player_drv) = s.drivers.get_mut(&player_idx) {
            for imp in carried_imported_stints {
                if !player_drv.stints.iter().any(|st| st.is_imported && st.saved_id == imp.saved_id) {
                    let insert_idx = player_drv.stints.iter().position(|st| !st.is_imported).unwrap_or(player_drv.stints.len());
                    player_drv.stints.insert(insert_idx, imp);
                }
            }
        }
    }

    // Auto-seed historical fuel burn rate from local Stint Database when Track + Car are known
    if is_real_track_name(&s.track_name) {
        let needs_seed = s.drivers.get(&player_idx).map_or(false, |d| d.historical_fuel_burn_rate <= 0.001);
        if needs_seed {
            if let Some(cached_burn) = stint_db::StintDatabase::load().latest_fuel_burn_for_track_and_car(&s.track_name, &s.car_name) {
                if let Some(player_drv) = s.drivers.get_mut(&player_idx) {
                    player_drv.historical_fuel_burn_rate = cached_burn;
                }
            }
        }
    }

    // Backfill setup_name into player driver and any live stints that were started before setup was known
    if effective_setup != "Baseline" {
        if let Some(player_drv) = s.drivers.get_mut(&player_idx) {
            player_drv.setup_name = effective_setup.clone();
            for stint in &mut player_drv.stints {
                if !stint.is_imported && (stint.setup_name.is_empty() || stint.setup_name == "—" || stint.setup_name == "Baseline") {
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
                let mut triggered_stint_num = None;
                if let Some(player_drv) = s.drivers.get_mut(&player_idx) {
                    if let Some(last_stint) = player_drv.last_live_stint_mut() {
                        if !last_stint.is_active && last_stint.laps_completed > 0 {
                            if last_stint.tires.should_trigger_popup(&garage_tires) {
                                triggered_stint_num = Some(last_stint.stint_number);
                            }
                            last_stint.tires = garage_tires;
                        }
                    }
                }
                if let Some(s_num) = triggered_stint_num {
                    s.tire_popup_trigger_seq += 1;
                    s.tire_popup_stint_num = s_num;
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

fn node_has_tire_fields(node: &serde_yaml::Value) -> bool {
    let tire_keys = [
        "LastHotPressure",
        "Last Hot Pressure",
        "LastPressure",
        "HotPressure",
        "Hot Pressure",
        "LastTempsOMI",
        "Last Temps OMI",
        "TempsOMI",
        "LastTempsIMO",
        "Last Temps IMO",
        "TempsIMO",
        "LastTemps",
        "Temps",
        "TreadRemaining",
        "Tread Remaining",
        "Wear",
        "StartingPressure",
        "ColdPressure",
    ];
    tire_keys.iter().any(|k| node.get(*k).is_some())
}

fn find_setup_corner<'a>(root: &'a serde_yaml::Value, corner_keys: &[&str]) -> Option<&'a serde_yaml::Value> {
    let mut fallback: Option<&'a serde_yaml::Value> = None;

    for parent in &["Tires", "TiresAero", "Tire", "Chassis", "Suspension"] {
        if let Some(p) = root.get(*parent) {
            for key in corner_keys {
                if let Some(c) = p.get(*key) {
                    if node_has_tire_fields(c) {
                        return Some(c);
                    }
                    if fallback.is_none() {
                        fallback = Some(c);
                    }
                }
            }
        }
    }
    for key in corner_keys {
        if let Some(c) = root.get(*key) {
            if node_has_tire_fields(c) {
                return Some(c);
            }
            if fallback.is_none() {
                fallback = Some(c);
            }
        }
    }
    if let Some(map) = root.as_mapping() {
        for (_k, v) in map {
            for key in corner_keys {
                if let Some(c) = v.get(*key) {
                    if node_has_tire_fields(c) {
                        return Some(c);
                    }
                    if fallback.is_none() {
                        fallback = Some(c);
                    }
                }
            }
        }
    }
    fallback
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
    let lf_node = find_setup_corner(car_setup, &["LeftFrontTire", "Left Front Tire", "LeftFront", "Left Front", "LF"])?;
    let rf_node = find_setup_corner(car_setup, &["RightFrontTire", "Right Front Tire", "RightFront", "Right Front", "RF"])?;
    let lr_node = find_setup_corner(car_setup, &["LeftRearTire", "Left Rear Tire", "LeftRear", "Left Rear", "LR"])?;
    let rr_node = find_setup_corner(car_setup, &["RightRearTire", "Right Rear Tire", "RightRear", "Right Rear", "RR"])?;

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

pub fn parse_car_setup_cold_pressures_kpa(car_setup: &serde_yaml::Value) -> Option<[f32; 4]> {
    let lf_node = find_setup_corner(car_setup, &["LeftFrontTire", "Left Front Tire", "LeftFront", "Left Front", "LF"])?;
    let rf_node = find_setup_corner(car_setup, &["RightFrontTire", "Right Front Tire", "RightFront", "Right Front", "RF"])?;
    let lr_node = find_setup_corner(car_setup, &["LeftRearTire", "Left Rear Tire", "LeftRear", "Left Rear", "LR"])?;
    let rr_node = find_setup_corner(car_setup, &["RightRearTire", "Right Rear Tire", "RightRear", "Right Rear", "RR"])?;

    let extract_cold = |node: &serde_yaml::Value| -> f32 {
        node.get("StartingPressure")
            .and_then(|v| v.as_str())
            .or_else(|| node.get("Starting Pressure").and_then(|v| v.as_str()))
            .or_else(|| node.get("ColdPressure").and_then(|v| v.as_str()))
            .or_else(|| node.get("Cold Pressure").and_then(|v| v.as_str()))
            .map(parse_pressure_str_to_kpa)
            .unwrap_or(0.0)
    };

    let c_lf = extract_cold(lf_node);
    let c_rf = extract_cold(rf_node);
    let c_lr = extract_cold(lr_node);
    let c_rr = extract_cold(rr_node);

    if c_lf > 1.0 || c_rf > 1.0 || c_lr > 1.0 || c_rr > 1.0 {
        Some([c_lf, c_rf, c_lr, c_rr])
    } else {
        None
    }
}

pub fn get_sample_f32(live: &iracing::telemetry::Sample, key: &'static str) -> f32 {
    match live.get(key) {
        Ok(Value::FLOAT(v)) => v,
        Ok(Value::DOUBLE(v)) => v as f32,
        _ => 0.0,
    }
}

pub fn read_player_tires(live: &iracing::telemetry::Sample, previous_hot: Option<&StintTireSet>) -> StintTireSet {
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
