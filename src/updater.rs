use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO: &str = "philliphenry215-eng/TheSearchForSpeedAnalyzer";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub html_url: String,
    pub body: String,
    pub is_prerelease: bool,
    pub binary_download_url: Option<String>,
    pub binary_size: u64,
    #[serde(default)]
    pub installer_download_url: Option<String>,
    #[serde(default)]
    pub installer_size: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable(ReleaseInfo),
    Downloading {
        tag: String,
        percent: f32,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    ReadyToRestart {
        tag: String,
    },
    Error(String),
}

pub struct UpdateChecker {
    pub status: UpdateStatus,
    receiver: Option<Receiver<UpdateStatus>>,
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self {
            status: UpdateStatus::Idle,
            receiver: None,
        }
    }
}

impl UpdateChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger_check(&mut self, include_prereleases: bool) {
        if self.status == UpdateStatus::Checking || matches!(self.status, UpdateStatus::Downloading { .. }) {
            return;
        }

        self.status = UpdateStatus::Checking;
        let (tx, rx): (Sender<UpdateStatus>, Receiver<UpdateStatus>) = channel();
        self.receiver = Some(rx);

        thread::spawn(move || {
            let res = fetch_latest_release(include_prereleases);
            let _ = tx.send(res);
        });
    }

    pub fn trigger_download_and_install(&mut self, info: &ReleaseInfo) {
        if matches!(self.status, UpdateStatus::Downloading { .. }) {
            return;
        }

        // If the current folder was installed by Inno Setup (unins000.exe present), prefer the
        // installer (which will be given /DIR=<current_dir> so custom C:/D:/E: paths stay in place).
        // If running as a standalone portable .exe on C:/D:/E:/Desktop, prefer in-place binary swap.
        let is_setup_install = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("unins000.exe").exists() || d.join("unins001.exe").exists()))
            .unwrap_or(false);

        let (url, kind, total_size) = if is_setup_install {
            match (&info.installer_download_url, &info.binary_download_url) {
                (Some(u), _) => (u.clone(), PendingKind::Installer, info.installer_size),
                (None, Some(u)) => (u.clone(), PendingKind::Binary, info.binary_size),
                (None, None) => {
                    open_browser(&info.html_url);
                    return;
                }
            }
        } else {
            match (&info.binary_download_url, &info.installer_download_url) {
                (Some(u), _) => (u.clone(), PendingKind::Binary, info.binary_size),
                (None, Some(u)) => (u.clone(), PendingKind::Installer, info.installer_size),
                (None, None) => {
                    open_browser(&info.html_url);
                    return;
                }
            }
        };

        let tag = info.tag_name.clone();

        self.status = UpdateStatus::Downloading {
            tag: tag.clone(),
            percent: 0.0,
            downloaded_bytes: 0,
            total_bytes: total_size,
        };

        let (tx, rx): (Sender<UpdateStatus>, Receiver<UpdateStatus>) = channel();
        self.receiver = Some(rx);

        thread::spawn(move || {
            let res = download_update(&url, kind, &tag, total_size, &tx);
            let _ = tx.send(res);
        });
    }

    pub fn poll(&mut self) -> Option<&UpdateStatus> {
        let mut got_update = false;
        let mut should_clear = false;

        if let Some(ref rx) = self.receiver {
            while let Ok(new_status) = rx.try_recv() {
                let is_terminal = matches!(
                    new_status,
                    UpdateStatus::Idle
                        | UpdateStatus::UpToDate
                        | UpdateStatus::UpdateAvailable(_)
                        | UpdateStatus::ReadyToRestart { .. }
                        | UpdateStatus::Error(_)
                );
                self.status = new_status;
                got_update = true;
                if is_terminal {
                    should_clear = true;
                    break;
                }
            }
        }

        if should_clear {
            self.receiver = None;
        }

        if got_update {
            Some(&self.status)
        } else {
            None
        }
    }

    pub fn dismiss(&mut self) {
        if matches!(self.status, UpdateStatus::UpdateAvailable(_))
            || matches!(self.status, UpdateStatus::Error(_))
        {
            self.status = UpdateStatus::Idle;
        }
    }
}

fn fetch_latest_release(include_prereleases: bool) -> UpdateStatus {
    let url = format!("https://api.github.com/repos/{}/releases?per_page=10", GITHUB_REPO);

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .build();

    let response = match agent
        .get(&url)
        .set("User-Agent", &format!("SearchForSpeedAnalyzer/{}", CURRENT_VERSION))
        .set("Accept", "application/vnd.github.v3+json")
        .call()
    {
        Ok(resp) => resp,
        Err(e) => {
            return UpdateStatus::Error(format!("Network error: {}", e));
        }
    };

    let releases: Result<Vec<serde_json::Value>, _> = response.into_json();
    match releases {
        Ok(list) => {
            if list.is_empty() {
                return UpdateStatus::UpToDate;
            }

            // If include_prereleases is true, take the latest release (stable or pre-release)
            // If include_prereleases is false, find the latest full stable release
            let target_release = if include_prereleases {
                list.into_iter().next()
            } else {
                list.into_iter().find(|r| !r["prerelease"].as_bool().unwrap_or(false))
            };

            if let Some(rel) = target_release {
                parse_release_json(&rel)
            } else {
                // If only beta/pre-releases exist and user hasn't opted into betas, stable is up to date
                UpdateStatus::UpToDate
            }
        }
        Err(e) => UpdateStatus::Error(format!("Invalid response: {}", e)),
    }
}

fn parse_release_json(json: &serde_json::Value) -> UpdateStatus {
    let tag = json["tag_name"].as_str().unwrap_or("").to_string();
    if tag.is_empty() {
        return UpdateStatus::UpToDate;
    }

    let is_prerelease = json["prerelease"].as_bool().unwrap_or(false);
    let html_url = json["html_url"].as_str().unwrap_or("").to_string();
    let name = json["name"].as_str().unwrap_or(&tag).to_string();
    let body = json["body"].as_str().unwrap_or("").to_string();

    let mut binary_download_url = None;
    let mut binary_size = 0u64;
    let mut installer_download_url = None;
    let mut installer_size = 0u64;

    if let Some(assets) = json["assets"].as_array() {
        let current_is_beta = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .map_or(false, |n| n.contains("beta"));

        // Preferred: Inno Setup installer (handles both per-user and per-machine Program Files installs)
        if let Some(asset) = pick_exe_asset(assets, true, current_is_beta) {
            if let Some(url) = asset["browser_download_url"].as_str() {
                installer_download_url = Some(url.to_string());
                installer_size = asset["size"].as_u64().unwrap_or(0);
            }
        }

        // Legacy fallback: raw standalone binary (older releases)
        if let Some(asset) = pick_exe_asset(assets, false, current_is_beta) {
            if let Some(url) = asset["browser_download_url"].as_str() {
                binary_download_url = Some(url.to_string());
                binary_size = asset["size"].as_u64().unwrap_or(0);
            }
        }
    }

    if is_newer_version(&tag, CURRENT_VERSION) {
        UpdateStatus::UpdateAvailable(ReleaseInfo {
            tag_name: tag,
            name,
            html_url,
            body,
            is_prerelease,
            binary_download_url,
            binary_size,
            installer_download_url,
            installer_size,
        })
    } else {
        UpdateStatus::UpToDate
    }
}

/// Picks the best matching `.exe` release asset.
/// `want_installer` selects `*-Setup.exe` assets; otherwise raw standalone binaries.
/// Beta builds prefer `beta` assets, stable builds prefer non-beta assets, falling back to any match.
fn pick_exe_asset<'a>(
    assets: &'a [serde_json::Value],
    want_installer: bool,
    current_is_beta: bool,
) -> Option<&'a serde_json::Value> {
    let name_of = |a: &serde_json::Value| a["name"].as_str().unwrap_or("").to_lowercase();
    let kind_matches = |n: &str| n.ends_with(".exe") && (n.contains("setup") == want_installer);

    assets
        .iter()
        .find(|a| {
            let n = name_of(a);
            kind_matches(&n) && (n.contains("beta") == current_is_beta)
        })
        .or_else(|| assets.iter().find(|a| kind_matches(&name_of(a))))
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PendingKind {
    /// Per-user Inno Setup installer, run silently after the app exits.
    Installer,
    /// Raw standalone binary, swapped in place (elevating via UAC only if the folder is protected).
    Binary,
}

#[derive(Debug, Clone)]
struct PendingUpdate {
    path: PathBuf,
    kind: PendingKind,
}

static PENDING_UPDATE: Mutex<Option<PendingUpdate>> = Mutex::new(None);

/// Scratch folder for downloaded updates. Always user-writable, regardless of install location.
fn update_temp_dir() -> PathBuf {
    std::env::temp_dir().join("SearchForSpeedAnalyzer-update")
}

fn download_update(
    url: &str,
    kind: PendingKind,
    tag: &str,
    total_expected: u64,
    tx: &Sender<UpdateStatus>,
) -> UpdateStatus {
    let dir = update_temp_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return UpdateStatus::Error(format!("Cannot create update folder: {}", e));
    }

    // Keep the original asset file name (e.g. SearchForSpeedAnalyzer-Setup.exe)
    let file_name = url
        .rsplit('/')
        .next()
        .filter(|n| n.to_lowercase().ends_with(".exe"))
        .unwrap_or(if kind == PendingKind::Installer { "SearchForSpeedAnalyzer-Setup.exe" } else { "SearchForSpeedAnalyzer.exe" })
        .to_string();
    let temp_exe = dir.join(file_name);
    let _ = std::fs::remove_file(&temp_exe);

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(120))
        .build();

    let response = match agent
        .get(url)
        .set("User-Agent", &format!("SearchForSpeedAnalyzer/{}", CURRENT_VERSION))
        .set("Accept", "application/octet-stream")
        .call()
    {
        Ok(resp) => resp,
        Err(e) => return UpdateStatus::Error(format!("Download failed: {}", e)),
    };

    let content_len = response
        .header("Content-Length")
        .and_then(|l| l.parse::<u64>().ok())
        .unwrap_or(total_expected);

    let mut reader = response.into_reader();
    let mut file = match std::fs::File::create(&temp_exe) {
        Ok(f) => f,
        Err(e) => return UpdateStatus::Error(format!("Failed to create update file: {}", e)),
    };

    let mut downloaded = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    let mut last_progress_report = std::time::Instant::now();

    loop {
        match std::io::Read::read(&mut reader, &mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                use std::io::Write;
                if let Err(e) = file.write_all(&buffer[..n]) {
                    let _ = std::fs::remove_file(&temp_exe);
                    return UpdateStatus::Error(format!("Write error: {}", e));
                }
                downloaded += n as u64;

                if last_progress_report.elapsed().as_millis() >= 80 {
                    let pct = if content_len > 0 {
                        ((downloaded as f32 / content_len as f32) * 100.0).clamp(0.0, 99.0)
                    } else {
                        50.0
                    };
                    let _ = tx.send(UpdateStatus::Downloading {
                        tag: tag.to_string(),
                        percent: pct,
                        downloaded_bytes: downloaded,
                        total_bytes: content_len,
                    });
                    last_progress_report = std::time::Instant::now();
                }
            }
            Err(e) => {
                let _ = std::fs::remove_file(&temp_exe);
                return UpdateStatus::Error(format!("Download interrupted: {}", e));
            }
        }
    }

    use std::io::Write;
    let _ = file.flush();
    drop(file);

    if downloaded < 500_000 {
        let _ = std::fs::remove_file(&temp_exe);
        return UpdateStatus::Error("Incomplete update downloaded".to_string());
    }

    if let Ok(mut pending) = PENDING_UPDATE.lock() {
        *pending = Some(PendingUpdate { path: temp_exe, kind });
    }

    UpdateStatus::ReadyToRestart {
        tag: tag.to_string(),
    }
}

/// Applies the downloaded update and exits the current process.
/// Only returns if something went wrong before handing off.
pub fn apply_update_and_restart() -> Result<(), String> {
    let pending = PENDING_UPDATE
        .lock()
        .ok()
        .and_then(|p| p.clone())
        .ok_or_else(|| "No downloaded update found. Please check for updates again.".to_string())?;

    if !pending.path.exists() {
        return Err("Update file not found. Please check for updates again.".to_string());
    }

    match pending.kind {
        PendingKind::Installer => launch_installer(&pending.path),
        PendingKind::Binary => apply_binary_swap(&pending.path),
    }
}

/// Runs the Inno Setup installer silently once this process has exited, targeting the
/// current application directory (`/DIR=...`) so custom C:/D:/E: drive installs stay in place.
/// If the target directory is protected (e.g. `C:\Program Files`), elevates via UAC (`-Verb RunAs`)
/// for the install step and then relaunches the app unelevated.
fn launch_installer(setup_path: &Path) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate current executable: {}", e))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "Cannot locate application folder".to_string())?;

    let dir_arg = format!("/DIR={}", exe_dir.to_string_lossy());
    let dir_arg_quoted = format!("'{}'", dir_arg.replace('\'', "''"));
    let needs_admin = !is_dir_writable(exe_dir);

    let script = if needs_admin {
        format!(
            "Wait-Process -Id {pid} -Timeout 30 -ErrorAction SilentlyContinue; \
             try {{ \
               $p = Start-Process -FilePath {setup} -Verb RunAs -Wait -PassThru \
                    -ArgumentList '/SILENT','/SUPPRESSMSGBOXES','/NORESTART','/ALLUSERS',{dir_arg}; \
               if ($p.ExitCode -ne 0) {{ throw 'installer failed' }}; \
               Start-Process -FilePath {cur} \
             }} catch {{ \
               Start-Process -FilePath {cur} -ArgumentList '{cancel_flag}' \
             }}",
            pid = std::process::id(),
            setup = ps_quote(setup_path),
            dir_arg = dir_arg_quoted,
            cur = ps_quote(&current_exe),
            cancel_flag = UPDATE_CANCELLED_ARG,
        )
    } else {
        format!(
            "Wait-Process -Id {pid} -Timeout 30 -ErrorAction SilentlyContinue; \
             Start-Process -FilePath {setup} -ArgumentList '/SILENT','/SUPPRESSMSGBOXES','/NORESTART','/CURRENTUSER','/RELAUNCH',{dir_arg}",
            pid = std::process::id(),
            setup = ps_quote(setup_path),
            dir_arg = dir_arg_quoted,
        )
    };

    spawn_hidden_powershell(&script)
        .map_err(|e| format!("Failed to launch installer: {}", e))?;
    std::process::exit(0);
}

/// Legacy path: swaps the raw binary in place. If the install folder is protected
/// (e.g. Program Files), a UAC prompt is shown only for the file copy, and the app
/// is relaunched afterwards *without* admin rights.
fn apply_binary_swap(new_exe: &Path) -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate current executable: {}", e))?;
    let exe_dir = current_exe
        .parent()
        .ok_or_else(|| "Cannot locate application folder".to_string())?;

    if is_dir_writable(exe_dir) {
        let old_exe = current_exe.with_extension("old.exe");
        let _ = std::fs::remove_file(&old_exe);
        if std::fs::rename(&current_exe, &old_exe).is_ok() {
            // copy (not rename): the download lives in %TEMP%, which may be on another drive
            if std::fs::copy(new_exe, &current_exe).is_ok() {
                let _ = std::fs::remove_file(new_exe);
                let _ = std::process::Command::new(&current_exe).spawn();
                std::process::exit(0);
            }
            // Roll back so the app is never left missing
            let _ = std::fs::rename(&old_exe, &current_exe);
        }
        // Fall through to the helper (e.g. file locked) - it waits for this process to exit
    }

    // Elevated copy step (runs as admin via UAC). Only does the file copy.
    let elevated_script = format!(
        "Wait-Process -Id {pid} -Timeout 30 -ErrorAction SilentlyContinue; \
         Start-Sleep -Milliseconds 300; \
         Copy-Item -LiteralPath {new} -Destination {cur} -Force -ErrorAction Stop",
        pid = std::process::id(),
        new = ps_quote(new_exe),
        cur = ps_quote(&current_exe),
    );

    // Outer non-elevated helper: requests UAC only when needed, then relaunches unelevated.
    let needs_admin = !is_dir_writable(exe_dir);
    let verb = if needs_admin { "-Verb RunAs " } else { "" };
    let outer_script = format!(
        "try {{ \
           $p = Start-Process -FilePath 'powershell.exe' {verb}-WindowStyle Hidden -Wait -PassThru \
                -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand','{inner}'; \
           if ($p.ExitCode -ne 0) {{ throw 'copy failed' }}; \
           Remove-Item -LiteralPath {new} -Force -ErrorAction SilentlyContinue; \
           Start-Process -FilePath {cur} \
         }} catch {{ \
           Wait-Process -Id {pid} -Timeout 30 -ErrorAction SilentlyContinue; \
           Start-Process -FilePath {cur} -ArgumentList '{cancel_flag}' \
         }}",
        verb = verb,
        inner = encode_ps_command(&elevated_script),
        new = ps_quote(new_exe),
        cur = ps_quote(&current_exe),
        pid = std::process::id(),
        cancel_flag = UPDATE_CANCELLED_ARG,
    );

    spawn_hidden_powershell(&outer_script)
        .map_err(|e| format!("Failed to launch updater process: {}", e))?;
    std::process::exit(0);
}

/// Command-line flag passed back to the app when an elevated update was declined or failed.
pub const UPDATE_CANCELLED_ARG: &str = "--update-cancelled";

fn is_dir_writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".s4s_write_test_{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Wraps a path in a PowerShell single-quoted literal.
fn ps_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "''"))
}

/// Encodes a script for `powershell -EncodedCommand` (Base64 of UTF-16LE), avoiding all quoting issues.
fn encode_ps_command(script: &str) -> String {
    let bytes: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    base64_encode(&bytes)
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
    }
    out
}

fn spawn_hidden_powershell(script: &str) -> std::io::Result<std::process::Child> {
    let mut cmd = std::process::Command::new("powershell.exe");
    cmd.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-WindowStyle",
        "Hidden",
        "-EncodedCommand",
        &encode_ps_command(script),
    ]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn()
}

pub fn cleanup_old_updates() {
    if let Ok(current_exe) = std::env::current_exe() {
        let old_exe = current_exe.with_extension("old.exe");
        if old_exe.exists() {
            let _ = std::fs::remove_file(old_exe);
        }
        let temp_exe = current_exe.with_extension("new.exe");
        if temp_exe.exists() {
            let _ = std::fs::remove_file(temp_exe);
        }
    }
    let _ = std::fs::remove_dir_all(update_temp_dir());
}

pub fn is_newer_version(remote: &str, current: &str) -> bool {
    let clean_remote = remote.trim().trim_start_matches(|c| c == 'v' || c == 'V');
    let clean_current = current.trim().trim_start_matches(|c| c == 'v' || c == 'V');

    let (r_core, _r_pre) = split_version(clean_remote);
    let (c_core, _c_pre) = split_version(clean_current);

    let r_nums = parse_version_numbers(r_core);
    let c_nums = parse_version_numbers(c_core);

    for i in 0..3 {
        let r = r_nums.get(i).copied().unwrap_or(0);
        let c = c_nums.get(i).copied().unwrap_or(0);
        if r > c {
            return true;
        } else if r < c {
            return false;
        }
    }

    // If core numbers match (e.g. 0.1.0 vs 0.1.0-beta.1), a release without prerelease tag is newer than one with prerelease tag
    if _c_pre.is_some() && _r_pre.is_none() {
        return true;
    }
    if _c_pre.is_none() && _r_pre.is_some() {
        return false;
    }
    if let (Some(cp), Some(rp)) = (_c_pre, _r_pre) {
        let r_pre_nums = parse_version_numbers(rp);
        let c_pre_nums = parse_version_numbers(cp);
        for i in 0..r_pre_nums.len().max(c_pre_nums.len()) {
            let r = r_pre_nums.get(i).copied().unwrap_or(0);
            let c = c_pre_nums.get(i).copied().unwrap_or(0);
            if r > c {
                return true;
            } else if r < c {
                return false;
            }
        }
        return rp > cp;
    }

    false
}

fn split_version(s: &str) -> (&str, Option<&str>) {
    if let Some((core, pre)) = s.split_once('-') {
        (core, Some(pre))
    } else if let Some((core, pre)) = s.split_once('+') {
        (core, Some(pre))
    } else {
        (s, None)
    }
}

fn parse_version_numbers(s: &str) -> Vec<u32> {
    s.split(|c: char| !c.is_ascii_digit())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

pub fn open_browser(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparisons() {
        assert!(is_newer_version("v0.2.0", "0.1.0"));
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(is_newer_version("v1.0.0", "0.9.9"));
        assert!(is_newer_version("v0.2.0-beta.1", "0.1.0"));
        assert!(is_newer_version("0.1.0", "0.1.0-beta.1"));
        assert!(is_newer_version("v0.1.0-beta.3", "0.1.0-beta.2"));

        assert!(!is_newer_version("v0.1.0-beta.2", "0.1.0-beta.3"));
        assert!(!is_newer_version("v0.1.0-beta.3", "0.1.0-beta.3"));
        assert!(!is_newer_version("v0.1.0-beta.3", "0.1.0"));
        assert!(!is_newer_version("v0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.0.9", "0.1.0"));
        assert!(!is_newer_version("v0.1.0-alpha", "0.1.0"));
    }

    #[test]
    fn test_parse_release_json_extracts_binary_asset() {
        let json = serde_json::json!({
            "tag_name": "v9.9.9",
            "name": "Release v9.9.9",
            "html_url": "https://github.com/philliphenry215-eng/TheSearchForSpeedAnalyzer/releases/tag/v9.9.9",
            "body": "Test notes",
            "prerelease": false,
            "assets": [
                {
                    "name": "SearchForSpeedAnalyzer-Setup.exe",
                    "size": 5000000,
                    "browser_download_url": "https://example.com/SearchForSpeedAnalyzer-Setup.exe"
                },
                {
                    "name": "SearchForSpeedAnalyzer.exe",
                    "size": 8800000,
                    "browser_download_url": "https://example.com/SearchForSpeedAnalyzer.exe"
                }
            ]
        });

        match parse_release_json(&json) {
            UpdateStatus::UpdateAvailable(info) => {
                assert_eq!(info.tag_name, "v9.9.9");
                assert_eq!(info.binary_download_url.as_deref(), Some("https://example.com/SearchForSpeedAnalyzer.exe"));
                assert_eq!(info.binary_size, 8800000);
                assert_eq!(info.installer_download_url.as_deref(), Some("https://example.com/SearchForSpeedAnalyzer-Setup.exe"));
                assert_eq!(info.installer_size, 5000000);
            }
            other => panic!("Expected UpdateAvailable, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_release_json_installer_only() {
        let json = serde_json::json!({
            "tag_name": "v9.9.9",
            "prerelease": false,
            "assets": [
                { "name": "SearchForSpeedAnalyzer-Beta-Setup.exe", "size": 4000000, "browser_download_url": "https://example.com/beta-setup.exe" },
                { "name": "SearchForSpeedAnalyzer-Setup.exe", "size": 5000000, "browser_download_url": "https://example.com/setup.exe" }
            ]
        });

        match parse_release_json(&json) {
            UpdateStatus::UpdateAvailable(info) => {
                assert_eq!(info.binary_download_url, None);
                assert_eq!(info.installer_download_url.as_deref(), Some("https://example.com/setup.exe"));
            }
            other => panic!("Expected UpdateAvailable, got {:?}", other),
        }
    }

    #[test]
    fn test_base64_encode() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
