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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable(ReleaseInfo),
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
        if self.status == UpdateStatus::Checking {
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

    pub fn poll(&mut self) -> Option<&UpdateStatus> {
        if let Some(ref rx) = self.receiver {
            if let Ok(new_status) = rx.try_recv() {
                self.status = new_status;
                self.receiver = None;
                return Some(&self.status);
            }
        }
        None
    }

    pub fn dismiss(&mut self) {
        if matches!(self.status, UpdateStatus::UpdateAvailable(_)) {
            self.status = UpdateStatus::Idle;
        }
    }
}

fn fetch_latest_release(include_prereleases: bool) -> UpdateStatus {
    let url = format!("https://api.github.com/repos/{}/releases?per_page=10", GITHUB_REPO);

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(6))
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

    if is_newer_version(&tag, CURRENT_VERSION) {
        UpdateStatus::UpdateAvailable(ReleaseInfo {
            tag_name: tag,
            name,
            html_url,
            body,
            is_prerelease,
        })
    } else {
        UpdateStatus::UpToDate
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
}
