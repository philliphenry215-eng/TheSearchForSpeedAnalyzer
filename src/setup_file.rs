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

/// Resolves the physical path of an iRacing `.sto` setup file from available session metadata.
/// Checks the direct raw path, car-specific subdirectories, and searches local setup folders.
pub fn resolve_sto_file(
    raw_setup_path: &str,
    setup_name: &str,
    car_path: &str,
    car_name: &str,
) -> Option<PathBuf> {
    let setups_dir = get_iracing_setups_dir()?;

    // 1. Direct absolute path check
    let raw_p = Path::new(raw_setup_path);
    if raw_p.is_absolute() && raw_p.is_file() {
        return Some(raw_p.to_path_buf());
    }

    // 2. Relative path directly from iRacing telemetry (e.g. "mustanggt3/Atlanta_Race_v2.sto")
    if !raw_setup_path.trim().is_empty() {
        let norm = raw_setup_path.replace('/', "\\");
        let candidate = setups_dir.join(&norm);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    // 3. Known car directory (e.g. setups/mustanggt3/Atlanta_Race_v2.sto)
    let clean_name = setup_name.trim();
    if !clean_name.is_empty() && clean_name != "—" && clean_name != "Baseline" {
        let sto_filename = ensure_sto_extension(clean_name);

        // Try with car_path
        if !car_path.trim().is_empty() {
            let candidate = setups_dir.join(car_path.trim()).join(&sto_filename);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        // Try with sanitized car_name
        if !car_name.trim().is_empty() {
            let car_slug = car_name.to_lowercase().replace(' ', "");
            let candidate = setups_dir.join(&car_slug).join(&sto_filename);
            if candidate.is_file() {
                return Some(candidate);
            }
        }

        // 4. Scan immediate subdirectories under iRacing/setups/ for matching setup_name.sto
        if let Ok(entries) = fs::read_dir(&setups_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let candidate = entry.path().join(&sto_filename);
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
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

/// Opens native Windows save dialog to export a copy of the .sto file.
pub fn pick_save_sto_file_dialog(default_filename: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter: Vec<u16> = "iRacing Setup Files (*.sto)\0*.sto\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "Export iRacing Setup As (.sto)\0"
            .encode_utf16()
            .collect();
        let def_ext: Vec<u16> = "sto\0".encode_utf16().collect();

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

/// Opens native Windows open dialog to pick an existing .sto file from disk.
pub fn pick_open_sto_file_dialog() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let filter: Vec<u16> = "iRacing Setup Files (*.sto)\0*.sto\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "Select iRacing Setup File (.sto)\0"
            .encode_utf16()
            .collect();

        let init_dir = std::env::var_os("USERPROFILE")
            .map(|u| PathBuf::from(u).join("Downloads"))
            .filter(|p| p.exists())
            .or_else(get_iracing_setups_dir);

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
        None
    }
}

/// Exports a setup: saves the source .sto file to a user-chosen destination.
pub fn export_sto_file(source_sto: &Path, default_name: &str) -> Result<PathBuf, String> {
    if !source_sto.is_file() {
        return Err(format!("Source setup file does not exist: {}", source_sto.display()));
    }
    let chosen_path = pick_save_sto_file_dialog(default_name)
        .ok_or_else(|| "Export cancelled by user".to_string())?;

    fs::copy(source_sto, &chosen_path)
        .map_err(|e| format!("Failed to save setup file: {}", e))?;

    Ok(chosen_path)
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
