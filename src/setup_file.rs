use std::fs;
use std::path::{Path, PathBuf};
use crate::settings::get_user_documents_dir;

#[cfg(target_os = "windows")]
use crate::stint_db::{GetOpenFileNameW, GetSaveFileNameW, OPENFILENAMEW};

/// Resolves the default iRacing setups root folder: `%USERPROFILE%\Documents\iRacing\setups\`
pub fn get_iracing_setups_dir() -> Option<PathBuf> {
    get_user_documents_dir().map(|docs| docs.join("iRacing").join("setups"))
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

/// Resolves the best iRacing car setup directory (`Documents\iRacing\setups\<car>`) for opening File Explorer dialogs.
pub fn resolve_car_setups_dir(raw_setup_path: &str, car_path: &str, car_name: &str) -> Option<PathBuf> {
    let setups_dir = get_iracing_setups_dir()?;

    if !raw_setup_path.trim().is_empty() {
        let raw_p = Path::new(raw_setup_path.trim());
        if raw_p.is_absolute() {
            if let Some(parent) = raw_p.parent() {
                if parent.is_dir() {
                    return Some(parent.to_path_buf());
                }
            }
        } else {
            let norm = raw_setup_path.replace('/', "\\");
            let candidate = setups_dir.join(&norm);
            if let Some(parent) = candidate.parent() {
                if parent.is_dir() {
                    return Some(parent.to_path_buf());
                }
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

    if setups_dir.is_dir() {
        Some(setups_dir)
    } else {
        None
    }
}

/// Resolves the physical path of an iRacing `.sto` setup file from available session metadata.
/// Checks the direct raw path, car-specific subdirectories (recursively), and local setup folders.
pub fn resolve_sto_file(
    raw_setup_path: &str,
    setup_name: &str,
    car_path: &str,
    car_name: &str,
) -> Option<PathBuf> {
    // 1. Direct absolute path check
    let raw_p = Path::new(raw_setup_path.trim());
    if !raw_setup_path.trim().is_empty() && raw_p.is_absolute() && raw_p.is_file() {
        return Some(raw_p.to_path_buf());
    }

    let setups_dir = get_iracing_setups_dir()?;

    // 2. Relative path directly from iRacing telemetry (e.g. "mustanggt3/Atlanta_Race_v2.sto")
    if !raw_setup_path.trim().is_empty() {
        let norm = raw_setup_path.trim().replace('/', "\\");
        let candidate = setups_dir.join(&norm);
        if candidate.is_file() {
            return Some(candidate);
        }
        let candidate_ext = setups_dir.join(ensure_sto_extension(&norm));
        if candidate_ext.is_file() {
            return Some(candidate_ext);
        }
    }

    // 3. Known car directory & recursive search (e.g. setups/mustanggt3/.../Atlanta_Race_v2.sto)
    let clean_name = setup_name.trim();
    if !clean_name.is_empty() && clean_name != "—" && !clean_name.eq_ignore_ascii_case("Baseline") {
        let sto_filename = ensure_sto_extension(clean_name);
        let sto_lower = sto_filename.to_lowercase();

        // Try with car_path (including nested subfolders)
        if !car_path.trim().is_empty() {
            let car_dir = setups_dir.join(car_path.trim().replace('/', "\\"));
            if let Some(found) = find_sto_recursive(&car_dir, &sto_lower, 4) {
                return Some(found);
            }
        }

        // Try with sanitized car_name (including nested subfolders)
        if !car_name.trim().is_empty() {
            let car_slug = car_name.to_lowercase().replace(' ', "");
            let car_dir = setups_dir.join(&car_slug);
            if let Some(found) = find_sto_recursive(&car_dir, &sto_lower, 4) {
                return Some(found);
            }
        }

        // 4. Recursively scan all car folders under iRacing/setups/ for matching <setup_name>.sto
        if let Some(found) = find_sto_recursive(&setups_dir, &sto_lower, 4) {
            return Some(found);
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

        let init_dir_buf = initial_dir
            .filter(|p| p.is_dir())
            .map(|p| p.to_path_buf())
            .or_else(get_iracing_setups_dir);

        let init_dir_w: Vec<u16> = init_dir_buf
            .map(|p| p.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect())
            .unwrap_or_default();

        let clean_default = ensure_sto_extension(default_filename);
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

/// Saves or renames a source `.sto` file using the native Windows File Explorer Save As dialog,
/// opening directly in the source setup's folder with `default_name` pre-populated.
pub fn export_sto_file(source_sto: &Path, default_name: &str) -> Result<PathBuf, String> {
    if !source_sto.is_file() {
        return Err(format!("Source setup file does not exist: {}", source_sto.display()));
    }
    let init_dir = source_sto.parent();
    let chosen_path = pick_save_sto_file_dialog_in_dir(default_name, init_dir)
        .ok_or_else(|| "Export cancelled by user".to_string())?;

    let src_canon = source_sto.canonicalize().unwrap_or_else(|_| source_sto.to_path_buf());
    let dst_canon = chosen_path.canonicalize().unwrap_or_else(|_| chosen_path.clone());
    if src_canon != dst_canon {
        fs::copy(source_sto, &chosen_path)
            .map_err(|e| format!("Failed to save setup file: {}", e))?;
    }

    Ok(chosen_path)
}

/// Resolves the stint's `.sto` setup file (or lets the user pick it from their car's setup folder if moved)
/// and immediately opens the Windows File Explorer Save As dialog to save/rename it directly to their files.
pub fn save_or_rename_stint_setup(
    raw_setup_path: &str,
    setup_name: &str,
    car_path: &str,
    car_name: &str,
) -> Result<PathBuf, String> {
    let source_sto = if let Some(found) = resolve_sto_file(raw_setup_path, setup_name, car_path, car_name) {
        found
    } else {
        let start_dir = resolve_car_setups_dir(raw_setup_path, car_path, car_name);
        pick_open_sto_file_dialog_in_dir(start_dir.as_deref())
            .ok_or_else(|| "Export cancelled by user".to_string())?
    };

    let default_name = if !setup_name.trim().is_empty()
        && setup_name.trim() != "—"
        && !setup_name.trim().eq_ignore_ascii_case("Baseline")
    {
        ensure_sto_extension(setup_name.trim())
    } else {
        source_sto
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("setup.sto")
            .to_string()
    };

    export_sto_file(&source_sto, &default_name)
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
}
