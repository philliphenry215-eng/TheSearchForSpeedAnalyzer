use std::sync::mpsc::{channel, Receiver, Sender};
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

        let url = match &info.binary_download_url {
            Some(u) => u.clone(),
            None => {
                // If direct binary not found in release assets, fallback to browser
                open_browser(&info.html_url);
                return;
            }
        };

        let tag = info.tag_name.clone();
        let total_size = info.binary_size;

        self.status = UpdateStatus::Downloading {
            tag: tag.clone(),
            percent: 0.0,
            downloaded_bytes: 0,
            total_bytes: total_size,
        };

        let (tx, rx): (Sender<UpdateStatus>, Receiver<UpdateStatus>) = channel();
        self.receiver = Some(rx);

        thread::spawn(move || {
            let res = download_binary(&url, &tag, total_size, &tx);
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

    if let Some(assets) = json["assets"].as_array() {
        let current_is_beta = std::env::current_exe()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()))
            .map_or(false, |n| n.contains("beta"));

        let target_asset = if current_is_beta {
            assets.iter().find(|a| {
                let name = a["name"].as_str().unwrap_or("").to_lowercase();
                name.ends_with(".exe") && name.contains("beta") && !name.contains("setup")
            }).or_else(|| {
                assets.iter().find(|a| {
                    let name = a["name"].as_str().unwrap_or("").to_lowercase();
                    name.ends_with(".exe") && !name.contains("setup")
                })
            })
        } else {
            assets.iter().find(|a| {
                let name = a["name"].as_str().unwrap_or("").to_lowercase();
                name.ends_with(".exe") && !name.contains("setup") && !name.contains("beta")
            }).or_else(|| {
                assets.iter().find(|a| {
                    let name = a["name"].as_str().unwrap_or("").to_lowercase();
                    name.ends_with(".exe") && !name.contains("setup")
                })
            })
        };

        if let Some(asset) = target_asset {
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
        })
    } else {
        UpdateStatus::UpToDate
    }
}

fn download_binary(
    url: &str,
    tag: &str,
    total_expected: u64,
    tx: &Sender<UpdateStatus>,
) -> UpdateStatus {
    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => return UpdateStatus::Error(format!("Cannot locate application: {}", e)),
    };

    let temp_exe = current_exe.with_extension("new.exe");
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

    UpdateStatus::ReadyToRestart {
        tag: tag.to_string(),
    }
}

pub fn apply_update_and_restart() -> Result<(), String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("Cannot locate current executable: {}", e))?;
    let temp_exe = current_exe.with_extension("new.exe");
    let old_exe = current_exe.with_extension("old.exe");

    if !temp_exe.exists() {
        return Err("Update file not found".to_string());
    }

    // Attempt direct in-process rename and swap
    let _ = std::fs::remove_file(&old_exe);
    if std::fs::rename(&current_exe, &old_exe).is_ok() && std::fs::rename(&temp_exe, &current_exe).is_ok() {
        let _ = std::process::Command::new(&current_exe).spawn();
        std::process::exit(0);
    }

    // Fallback: spawn hidden PowerShell helper script to wait, move, and relaunch
    let pid = std::process::id();
    let script = format!(
        "Start-Sleep -Milliseconds 400; Wait-Process -Id {} -Timeout 10; Move-Item -Force -Path '{}' -Destination '{}'; Start-Process -FilePath '{}'",
        pid,
        temp_exe.to_string_lossy(),
        current_exe.to_string_lossy(),
        current_exe.to_string_lossy()
    );

    let res = std::process::Command::new("powershell")
        .args(["-WindowStyle", "Hidden", "-Command", &script])
        .spawn();

    match res {
        Ok(_) => {
            std::process::exit(0);
        }
        Err(e) => Err(format!("Failed to launch updater process: {}", e)),
    }
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
            }
            other => panic!("Expected UpdateAvailable, got {:?}", other),
        }
    }
}
