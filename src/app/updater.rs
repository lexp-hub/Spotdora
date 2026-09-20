use isahc::config::Configurable;
use isahc::AsyncReadResponseExt;
use isahc::Request;
use serde::Deserialize;
use std::time::Duration;

const GITHUB_REPO_LATEST_RELEASE: &str =
    "https://api.github.com/repos/lexp-hub/Spotdora/releases/latest";

#[derive(Deserialize, Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: Option<u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub html_url: String,
    pub name: Option<String>,
    pub body: Option<String>,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    UpToDate(String),
    NewVersionAvailable {
        version: String,
        rpm_url: Option<String>,
        html_url: String,
        notes: String,
    },
    Error(String),
}

fn parse_version_tuple(v: &str) -> Vec<u32> {
    let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
    clean
        .split(|c: char| c == '.' || c == '-' || !c.is_numeric())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let l_parts = parse_version_tuple(latest);
    let c_parts = parse_version_tuple(current);

    let max_len = l_parts.len().max(c_parts.len());
    for i in 0..max_len {
        let l_num = l_parts.get(i).copied().unwrap_or(0);
        let c_num = c_parts.get(i).copied().unwrap_or(0);
        if l_num > c_num {
            return true;
        } else if l_num < c_num {
            return false;
        }
    }
    false
}

pub async fn check_latest_release(current_version: &str) -> UpdateStatus {
    let request_result = Request::get(GITHUB_REPO_LATEST_RELEASE)
        .timeout(Duration::from_secs(8))
        .header("User-Agent", "Spotdora-Workstation-Client")
        .header("Accept", "application/vnd.github.v3+json")
        .body(());

    let request = match request_result {
        Ok(req) => req,
        Err(e) => return UpdateStatus::Error(format!("Failed to build request: {e}")),
    };

    let mut response = match isahc::send_async(request).await {
        Ok(res) => res,
        Err(e) => return UpdateStatus::Error(format!("Network connection error: {e}")),
    };

    let status = response.status();
    if status.as_u16() == 404 {
        // No published releases on GitHub yet
        return UpdateStatus::UpToDate(current_version.to_string());
    }

    if !status.is_success() {
        return UpdateStatus::Error(format!("GitHub API returned HTTP {}", status.as_u16()));
    }

    let text = match response.text().await {
        Ok(t) => t,
        Err(e) => return UpdateStatus::Error(format!("Failed to read response body: {e}")),
    };

    let release: GitHubRelease = match serde_json::from_str(&text) {
        Ok(r) => r,
        Err(e) => return UpdateStatus::Error(format!("Failed to parse release metadata: {e}")),
    };

    let latest_version = release.tag_name.trim().trim_start_matches('v');
    if is_newer_version(latest_version, current_version) {
        let rpm_asset = release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".rpm"))
            .map(|a| a.browser_download_url.clone());

        UpdateStatus::NewVersionAvailable {
            version: release.tag_name.clone(),
            rpm_url: rpm_asset,
            html_url: release.html_url.clone(),
            notes: release.body.unwrap_or_else(|| "No release notes provided.".to_string()),
        }
    } else {
        UpdateStatus::UpToDate(current_version.to_string())
    }
}
