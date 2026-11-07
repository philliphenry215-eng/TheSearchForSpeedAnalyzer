use std::fs;
use std::path::{Path, PathBuf};
use crate::models::clean_setup_name;
use crate::settings::get_user_documents_dir;

#[cfg(target_os = "windows")]
use crate::stint_db::{GetOpenFileNameW, GetSaveFileNameW, OPENFILENAMEW};

/// Returns all candidate iRacing setups root folders (`Documents\iRacing\setups` across local and OneDrive Documents).
pub fn get_iracing_setups_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push_unique = |p: PathBuf| {
        if !dirs.contains(&p) {
            dirs.push(p);
        }
    };

    if let Some(docs) = get_user_documents_dir() {
        push_unique(docs.join("iRacing").join("setups"));
    }
    for env_key in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(od) = std::env::var_os(env_key).filter(|v| !v.is_empty()) {
            push_unique(PathBuf::from(od).join("Documents").join("iRacing").join("setups"));
        }
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").filter(|v| !v.is_empty()) {
        let prof = PathBuf::from(profile);
        push_unique(prof.join("OneDrive").join("Documents").join("iRacing").join("setups"));
        push_unique(prof.join("Documents").join("iRacing").join("setups"));
    }
    dirs
}

/// Resolves the active iRacing setups root folder: prefers an existing `iRacing\setups` directory
/// (checking both standard `Documents` and `OneDrive\Documents`), or falls back to `%USERPROFILE%\Documents\iRacing\setups`.
pub fn get_iracing_setups_dir() -> Option<PathBuf> {
    let candidates = get_iracing_setups_dirs();
    if let Some(existing) = candidates.iter().find(|p| p.is_dir()) {
        return Some(existing.clone());
    }
    candidates.into_iter().next()
}

/// Normalizes a setup name to ensure it has a clean `.sto` extension.
pub fn ensure_sto_extension(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.to_lowercase().ends_with(".sto") {
        trimmed.to_string()
    } else {
        format!("{}.sto", trimmed)
    }
}

/// Recursively searches `dir` (up to `max_depth` levels) for a file matching `target_filename_lower` (case-insensitive).
fn find_sto_recursive(dir: &Path, target_filename_lower: &str, max_depth: usize) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    let entries = fs::read_dir(dir).ok()?;
    let mut subdirs = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Ok(ft) = entry.file_type() {
            if ft.is_file() {
                if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    if fname.to_lowercase() == target_filename_lower {
                        return Some(path);
                    }
                }
            } else if ft.is_dir() && max_depth > 0 {
                subdirs.push(path);
            }
        }
    }
    for sub in subdirs {
        if let Some(found) = find_sto_recursive(&sub, target_filename_lower, max_depth - 1) {
            return Some(found);
        }
    }
    None
}

/// Recursively collects all `.sto` files inside `dir` (up to `max_depth` levels).
fn collect_sto_files_recursive(dir: &Path, max_depth: usize, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut subdirs = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Ok(ft) = entry.file_type() {
            if ft.is_file() {
                if path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("sto"))
                    .unwrap_or(false)
                {
                    out.push(path);
                }
            } else if ft.is_dir() && max_depth > 0 {
                subdirs.push(path);
            }
        }
    }
    for sub in subdirs {
        collect_sto_files_recursive(&sub, max_depth - 1, out);
    }
}

/// Finds the most recently modified `.sto` file inside `dir` (up to `max_depth` levels).
fn find_latest_sto_in_dir(dir: &Path, max_depth: usize) -> Option<PathBuf> {
    let mut files = Vec::new();
    collect_sto_files_recursive(dir, max_depth, &mut files);
    files.into_iter().max_by_key(|p| {
        fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    })
}

/// Resolves the best iRacing car setup directory (`Documents\iRacing\setups\<car>`) for opening File Explorer dialogs.
pub fn resolve_car_setups_dir(raw_setup_path: &str, car_path: &str, car_name: &str) -> Option<PathBuf> {
    if !raw_setup_path.trim().is_empty() {
        let raw_p = Path::new(raw_setup_path.trim());
        if raw_p.is_absolute() {
            if let Some(parent) = raw_p.parent() {
                if parent.is_dir() {
                    return Some(parent.to_path_buf());
                }
            }
        }
    }

    for setups_dir in get_iracing_setups_dirs() {
        if !raw_setup_path.trim().is_empty() {
            let norm = raw_setup_path.trim().replace('/', "\\");
            let candidate = setups_dir.join(&norm);
            if let Some(parent) = candidate.parent() {
                if parent != setups_dir && parent.is_dir() {
                    return Some(parent.to_path_buf());
                }
            }
        }

        if !car_path.trim().is_empty() {
            let norm_car = car_path.trim().replace('/', "\\");
            let candidate = setups_dir.join(&norm_car);
            if candidate.is_dir() {
                return Some(candidate);
            }
        }

        if !car_name.trim().is_empty() && car_name.trim() != "—" {
            let car_lower = car_name.trim().to_lowercase();
            let car_slug = car_lower.replace(' ', "");
            let candidate = setups_dir.join(&car_slug);
            if candidate.is_dir() {
                return Some(candidate);
            }
            if let Ok(entries) = fs::read_dir(&setups_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        if let Some(folder_name) = entry.file_name().to_str() {
                            let f_lower = folder_name.to_lowercase();
                            if f_lower == car_lower || f_lower.replace(' ', "") == car_slug {
                                return Some(entry.path());
                            }
                        }
                    }
                }
            }
        }
    }

    let setups_dir = get_iracing_setups_dir()?;
    if !car_path.trim().is_empty() {
        let norm_car = car_path.trim().replace('/', "\\");
        return Some(setups_dir.join(norm_car));
    }
    if setups_dir.is_dir() {
        Some(setups_dir)
    } else {
        None
    }
}

fn try_resolve_raw_path_in_base(raw_setup_path: &str, car_path: &str, setups_dir: &Path) -> Option<PathBuf> {
    let raw_trimmed = raw_setup_path.trim();
    if raw_trimmed.is_empty() {
        return None;
    }
    let raw_p = Path::new(raw_trimmed);
    if raw_p.is_absolute() && raw_p.is_file() {
        return Some(raw_p.to_path_buf());
    }

    let norm = raw_trimmed.replace('/', "\\");
    let norm_ext = ensure_sto_extension(&norm);

    // Check relative to car_path first (iRacing DriverSetupName is often relative to setups/<car_path>/)
    if !car_path.trim().is_empty() {
        let car_dir = setups_dir.join(car_path.trim().replace('/', "\\"));
        let cand_car = car_dir.join(&norm);
        if cand_car.is_file() {
            return Some(cand_car);
        }
        let cand_car_ext = car_dir.join(&norm_ext);
        if cand_car_ext.is_file() {
            return Some(cand_car_ext);
        }
    }

    // Check relative to setups_dir
    let candidate = setups_dir.join(&norm);
    if candidate.is_file() {
        return Some(candidate);
    }
    let candidate_ext = setups_dir.join(&norm_ext);
    if candidate_ext.is_file() {
        return Some(candidate_ext);
    }

    None
}

/// Resolves the physical path of an iRacing `.sto` setup file from available session metadata.
/// Checks the direct raw path, car-specific subdirectories (recursively), and local setup folders.
pub fn resolve_sto_file(
    raw_setup_path: &str,
    setup_name: &str,
    car_path: &str,
    car_name: &str,
) -> Option<PathBuf> {
    let clean_name = clean_setup_name(setup_name);
    let clean_raw = clean_setup_name(raw_setup_path);
    let has_specific_setup_name =
        !clean_name.is_empty() && clean_name != "—" && !clean_name.eq_ignore_ascii_case("Baseline");
    let raw_matches_setup_name = !has_specific_setup_name
        || clean_raw.is_empty()
        || clean_raw.eq_ignore_ascii_case(&clean_name);

    // 1. Direct absolute path check (when raw_setup_path matches the requested setup_name)
    let raw_p = Path::new(raw_setup_path.trim());
    if raw_matches_setup_name && !raw_setup_path.trim().is_empty() && raw_p.is_absolute() && raw_p.is_file() {
        return Some(raw_p.to_path_buf());
    }

    let setups_dirs = get_iracing_setups_dirs();

    // 2. Relative path from iRacing telemetry (when raw_setup_path matches setup_name)
    if raw_matches_setup_name && !raw_setup_path.trim().is_empty() {
        for setups_dir in &setups_dirs {
            if let Some(found) = try_resolve_raw_path_in_base(raw_setup_path, car_path, setups_dir) {
                return Some(found);
            }
        }
    }

    // 3. Search by setup_name across car-specific folders and setups root
    let search_name = setup_name.trim();
    if !search_name.is_empty() && search_name != "—" {
        let is_baseline = search_name.eq_ignore_ascii_case("Baseline") || clean_name.eq_ignore_ascii_case("Baseline");
        let sto_filename = ensure_sto_extension(if !clean_name.is_empty() { &clean_name } else { search_name });
        let sto_lower = sto_filename.to_lowercase();

        for setups_dir in &setups_dirs {
            // Try with car_path (including nested subfolders)
            if !car_path.trim().is_empty() {
                let car_dir = setups_dir.join(car_path.trim().replace('/', "\\"));
                if let Some(found) = find_sto_recursive(&car_dir, &sto_lower, 4) {
                    return Some(found);
                }
            }

            // Try with sanitized car_name (including nested subfolders)
            if !car_name.trim().is_empty() && car_name.trim() != "—" {
                let car_slug = car_name.trim().to_lowercase().replace(' ', "");
                let car_dir = setups_dir.join(&car_slug);
                if let Some(found) = find_sto_recursive(&car_dir, &sto_lower, 4) {
                    return Some(found);
                }
            }
        }

        // 4. For non-Baseline setup names, recursively scan all car folders under iRacing/setups/
        if !is_baseline {
            for setups_dir in &setups_dirs {
                if let Some(found) = find_sto_recursive(setups_dir, &sto_lower, 4) {
                    return Some(found);
                }
            }
        }
    }

    // 5. Fallback: if raw_setup_path was non-empty (even if its stem differed from setup_name), try resolving it
    if !raw_matches_setup_name && !raw_setup_path.trim().is_empty() {
        if raw_p.is_absolute() && raw_p.is_file() {
            return Some(raw_p.to_path_buf());
        }
        for setups_dir in &setups_dirs {
            if let Some(found) = try_resolve_raw_path_in_base(raw_setup_path, car_path, setups_dir) {
                return Some(found);
            }
        }
    }

    None
}

/// Copies the physical .sto setup file to the Windows Clipboard (CF_HDROP).
/// Allows instantaneous pasting (Ctrl+V) into Discord, Slack, Teams, or Windows Explorer.
pub fn copy_sto_file_to_clipboard(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Setup file does not exist: {}", path.display()));
    }
    use clipboard_win::Setter;
    let abs_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = abs_path.to_string_lossy();
    let clean_path = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str).to_string();

    let _clip = clipboard_win::Clipboard::new_attempts(10)
        .map_err(|e| format!("Could not open Windows clipboard: {}", e))?;
    let _ = clipboard_win::raw::empty();

    let paths = [clean_path.as_str()];
    clipboard_win::formats::FileList
        .write_clipboard(&paths)
        .map_err(|e| format!("Failed to copy setup to clipboard: {}", e))?;

    Ok(())
}

/// Opens native Windows File Explorer Save dialog to save or rename a `.sto` setup file.
pub fn pick_save_sto_file_dialog(default_filename: &str) -> Option<PathBuf> {
    pick_save_sto_file_dialog_in_dir(default_filename, None)
}

/// Opens native Windows File Explorer Save dialog starting in `initial_dir` (defaults to iRacing setups folder).
pub fn pick_save_sto_file_dialog_in_dir(default_filename: &str, initial_dir: Option<&Path>) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter: Vec<u16> = "iRacing Setup Files (*.sto)\0*.sto\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "Save / Rename iRacing Setup (.sto)\0"
            .encode_utf16()
            .collect();
        let def_ext: Vec<u16> = "sto\0".encode_utf16().collect();

        if let Some(dir) = initial_dir {
            if !dir.exists() {
                let _ = fs::create_dir_all(dir);
            }
        }

        let init_dir_buf = initial_dir
            .filter(|p| p.is_dir())
            .map(|p| p.to_path_buf())
            .or_else(get_iracing_setups_dir);

        let init_dir_w: Vec<u16> = init_dir_buf
            .map(|p| p.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect())
            .unwrap_or_default();

        let leaf_name = default_filename
            .trim()
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(default_filename.trim());
        let clean_default = ensure_sto_extension(if leaf_name.is_empty() { "setup.sto" } else { leaf_name });
        let mut buf = [0u16; 1024];
        let def_name_w: Vec<u16> = clean_default.encode_utf16().collect();
        let copy_len = def_name_w.len().min(buf.len() - 1);
        buf[..copy_len].copy_from_slice(&def_name_w[..copy_len]);

        let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        if !init_dir_w.is_empty() {
            ofn.lpstrInitialDir = init_dir_w.as_ptr();
        }
        ofn.lpstrTitle = title.as_ptr();
        ofn.lpstrDefExt = def_ext.as_ptr();
        // OFN_PATHMUSTEXIST (0x800) | OFN_EXPLORER (0x80000) | OFN_NOCHANGEDIR (0x8)
        ofn.Flags = 0x00080808;

        let res = unsafe { GetSaveFileNameW(&mut ofn) };
        if res != 0 {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let path_str = String::from_utf16_lossy(&buf[..len]);
            let ensured = ensure_sto_extension(&path_str);
            return Some(PathBuf::from(ensured));
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (default_filename, initial_dir);
        None
    }
}

/// Opens native Windows open dialog to pick an existing .sto file from disk.
pub fn pick_open_sto_file_dialog() -> Option<PathBuf> {
    pick_open_sto_file_dialog_in_dir(None)
}

/// Opens native Windows open dialog starting in `initial_dir` (or Downloads / iRacing setups).
pub fn pick_open_sto_file_dialog_in_dir(initial_dir: Option<&Path>) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter: Vec<u16> = "iRacing Setup Files (*.sto)\0*.sto\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "Select iRacing Setup File (.sto)\0"
            .encode_utf16()
            .collect();

        let init_dir = initial_dir
            .filter(|p| p.is_dir())
            .map(|p| p.to_path_buf())
            .or_else(get_iracing_setups_dir)
            .or_else(|| {
                std::env::var_os("USERPROFILE")
                    .map(|u| PathBuf::from(u).join("Downloads"))
                    .filter(|p| p.exists())
            });

        let init_dir_w: Vec<u16> = init_dir
            .map(|p| p.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect())
            .unwrap_or_default();

        let mut buf = [0u16; 1024];
        let mut ofn: OPENFILENAMEW = unsafe { std::mem::zeroed() };
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.nFilterIndex = 1;
        ofn.lpstrFile = buf.as_mut_ptr();
        ofn.nMaxFile = buf.len() as u32;
        if !init_dir_w.is_empty() {
            ofn.lpstrInitialDir = init_dir_w.as_ptr();
        }
        ofn.lpstrTitle = title.as_ptr();
        // OFN_FILEMUSTEXIST (0x1000) | OFN_PATHMUSTEXIST (0x800) | OFN_EXPLORER (0x80000) | OFN_NOCHANGEDIR (0x8)
        ofn.Flags = 0x00081808;

        let res = unsafe { GetOpenFileNameW(&mut ofn) };
        if res != 0 {
            let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            let path_str = String::from_utf16_lossy(&buf[..len]);
            let p = PathBuf::from(path_str);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = initial_dir;
        None
    }
}

/// Core save/rename completion logic after the File Explorer Save dialog closes.
/// Supports:
/// 1. Typing a new filename in the Save dialog and clicking Save (copies source `.sto` to the new path).
/// 2. Selecting an existing `.sto` file in the Save dialog and clicking Save.
/// 3. Right-clicking a `.sto` file inside the Explorer dialog to rename it in-place, then clicking Save or closing the dialog.
pub fn commit_sto_save_or_rename(
    source_sto: Option<&Path>,
    fallback_source_sto: Option<&Path>,
    chosen_path_opt: Option<PathBuf>,
    start_dir: Option<&Path>,
    pre_dialog_files: &[PathBuf],
) -> Result<PathBuf, String> {
    let find_in_place_renamed_file = || -> Option<PathBuf> {
        let dir = start_dir.or_else(|| source_sto.and_then(|s| s.parent()))?;
        let mut post_files = Vec::new();
        collect_sto_files_recursive(dir, 2, &mut post_files);
        post_files
            .into_iter()
            .filter(|p| !pre_dialog_files.iter().any(|old| old == p))
            .max_by_key(|p| {
                fs::metadata(p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            })
    };

    match chosen_path_opt {
        Some(raw_chosen) => {
            let chosen_path = PathBuf::from(ensure_sto_extension(&raw_chosen.to_string_lossy()));
            if let Some(parent) = chosen_path.parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    let _ = fs::create_dir_all(parent);
                }
            }

            if let Some(src) = source_sto {
                if src.is_file() {
                    let src_canon = src.canonicalize().unwrap_or_else(|_| src.to_path_buf());
                    let dst_canon = chosen_path.canonicalize().unwrap_or_else(|_| chosen_path.clone());
                    if src_canon != dst_canon {
                        fs::copy(src, &chosen_path)
                            .map_err(|e| format!("Failed to save setup file: {}", e))?;
                    }
                    return Ok(chosen_path);
                } else {
                    // `src` no longer exists because the user renamed it in-place inside File Explorer!
                    if chosen_path.is_file() {
                        return Ok(chosen_path);
                    }
                    if let Some(renamed) = find_in_place_renamed_file() {
                        return Ok(renamed);
                    }
                }
            }

            // No direct `source_sto` (or it wasn't found beforehand)
            if chosen_path.is_file() {
                return Ok(chosen_path);
            }

            if let Some(fb) = fallback_source_sto {
                if fb.is_file() {
                    let fb_canon = fb.canonicalize().unwrap_or_else(|_| fb.to_path_buf());
                    let dst_canon = chosen_path.canonicalize().unwrap_or_else(|_| chosen_path.clone());
                    if fb_canon != dst_canon {
                        fs::copy(fb, &chosen_path)
                            .map_err(|e| format!("Failed to save setup file: {}", e))?;
                    }
                    return Ok(chosen_path);
                }
            }

            if let Some(renamed) = find_in_place_renamed_file() {
                return Ok(renamed);
            }

            // Last resort: ensure the target file is created on disk so the user's chosen setup name & path exist
            fs::write(&chosen_path, b"iRacing Setup File\n")
                .map_err(|e| format!("Failed to write setup file: {}", e))?;
            Ok(chosen_path)
        }
        None => {
            // User closed/cancelled the dialog — check if they renamed `source_sto` in-place inside Explorer first!
            if let Some(src) = source_sto {
                if !src.exists() {
                    if let Some(renamed) = find_in_place_renamed_file() {
                        return Ok(renamed);
                    }
                }
            }
            if let Some(renamed) = find_in_place_renamed_file() {
                return Ok(renamed);
            }
            Err("Export cancelled by user".to_string())
        }
    }
}

/// Saves or renames a source `.sto` file using the native Windows File Explorer Save As dialog,
/// opening directly in the source setup's folder with `default_name` pre-populated.
pub fn export_sto_file(source_sto: &Path, default_name: &str) -> Result<PathBuf, String> {
    let init_dir = source_sto.parent();
    let mut pre_files = Vec::new();
    if let Some(dir) = init_dir {
        collect_sto_files_recursive(dir, 2, &mut pre_files);
    }
    let chosen_path_opt = pick_save_sto_file_dialog_in_dir(default_name, init_dir);
    commit_sto_save_or_rename(
        Some(source_sto),
        None,
        chosen_path_opt,
        init_dir,
        &pre_files,
    )
}

/// Resolves the stint's `.sto` setup file and opens a single Windows File Explorer Save As dialog
/// in the car's setups folder so the user can save or rename the setup directly.
pub fn save_or_rename_stint_setup(
    raw_setup_path: &str,
    setup_name: &str,
    car_path: &str,
    car_name: &str,
) -> Result<PathBuf, String> {
    let source_sto = resolve_sto_file(raw_setup_path, setup_name, car_path, car_name);
    let start_dir = source_sto
        .as_ref()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .or_else(|| resolve_car_setups_dir(raw_setup_path, car_path, car_name));

    let fallback_source_sto = if source_sto.is_none() {
        start_dir.as_deref().and_then(|dir| find_latest_sto_in_dir(dir, 2))
    } else {
        None
    };

    let mut pre_files = Vec::new();
    if let Some(ref dir) = start_dir {
        collect_sto_files_recursive(dir, 2, &mut pre_files);
    }

    let default_name = if !setup_name.trim().is_empty()
        && setup_name.trim() != "—"
        && !setup_name.trim().eq_ignore_ascii_case("Baseline")
    {
        ensure_sto_extension(setup_name.trim())
    } else if let Some(ref src) = source_sto {
        src.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("setup.sto")
            .to_string()
    } else if let Some(ref fb) = fallback_source_sto {
        fb.file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("setup.sto")
            .to_string()
    } else {
        "setup.sto".to_string()
    };

    let chosen_path_opt = pick_save_sto_file_dialog_in_dir(&default_name, start_dir.as_deref());
    commit_sto_save_or_rename(
        source_sto.as_deref(),
        fallback_source_sto.as_deref(),
        chosen_path_opt,
        start_dir.as_deref(),
        &pre_files,
    )
}

/// Copies an imported .sto setup file directly into the player's iRacing setups car folder
/// so that it appears immediately in iRacing's garage menu.
pub fn import_sto_to_iracing_garage(source_sto: &Path, car_dir: &str) -> Result<PathBuf, String> {
    if !source_sto.is_file() {
        return Err(format!("Source setup file does not exist: {}", source_sto.display()));
    }
    let setups_root = get_iracing_setups_dir()
        .ok_or_else(|| "Could not locate iRacing setups folder in Documents".to_string())?;

    let file_name = source_sto
        .file_name()
        .ok_or_else(|| "Invalid setup filename".to_string())?;

    let target_dir = if !car_dir.trim().is_empty() {
        setups_root.join(car_dir.trim())
    } else {
        setups_root
    };

    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Could not create setups directory '{}': {}", target_dir.display(), e))?;

    let dest = target_dir.join(file_name);
    fs::copy(source_sto, &dest)
        .map_err(|e| format!("Could not copy setup file into '{}': {}", dest.display(), e))?;

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_sto_extension() {
        assert_eq!(ensure_sto_extension("test"), "test.sto");
        assert_eq!(ensure_sto_extension("my_setup.sto"), "my_setup.sto");
        assert_eq!(ensure_sto_extension("my_setup.STO"), "my_setup.STO");
        assert_eq!(ensure_sto_extension("  spaced_setup  "), "spaced_setup.sto");
        assert_eq!(ensure_sto_extension("  spaced.sto  "), "spaced.sto");
    }

    #[test]
    fn test_resolve_sto_file_absolute_path() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("s4s_test_setup_dummy.sto");
        let _ = fs::write(&test_file, b"DUMMY_SETUP_DATA");

        let resolved = resolve_sto_file(
            test_file.to_str().unwrap(),
            "s4s_test_setup_dummy",
            "dummy_car",
            "Dummy Car",
        );
        assert_eq!(resolved, Some(test_file.clone()));

        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_resolve_sto_file_empty_or_baseline() {
        assert_eq!(resolve_sto_file("", "", "", ""), None);
        assert_eq!(resolve_sto_file("", "Baseline", "", ""), None);
        assert_eq!(resolve_sto_file("", "—", "", ""), None);
    }

    #[test]
    fn test_commit_sto_save_or_rename_workflows() {
        let temp_dir = std::env::temp_dir().join("s4s_test_sto_workflows");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let src_file = temp_dir.join("Atlanta_v1.sto");
        fs::write(&src_file, b"BINARY_STO_V1").unwrap();
        let pre_files = vec![src_file.clone()];

        // 1. User types a new filename `Atlanta_v2` in the Save dialog and clicks Save
        let dst_file = temp_dir.join("Atlanta_v2");
        let saved = commit_sto_save_or_rename(
            Some(&src_file),
            None,
            Some(dst_file),
            Some(&temp_dir),
            &pre_files,
        )
        .expect("Save as new filename should succeed");
        assert_eq!(saved, temp_dir.join("Atlanta_v2.sto"));
        assert!(saved.is_file());
        assert_eq!(fs::read(&saved).unwrap(), b"BINARY_STO_V1");

        // 2. User right-clicks `Atlanta_v1.sto` inside Explorer, renames it to `Atlanta_Renamed.sto`, and closes/cancels the dialog
        let renamed_in_explorer = temp_dir.join("Atlanta_Renamed.sto");
        let pre_files_2 = vec![src_file.clone(), saved.clone()];
        fs::rename(&src_file, &renamed_in_explorer).unwrap();
        let detected = commit_sto_save_or_rename(
            Some(&src_file),
            None,
            None,
            Some(&temp_dir),
            &pre_files_2,
        )
        .expect("In-place rename inside Explorer should be detected");
        assert_eq!(detected, renamed_in_explorer);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
