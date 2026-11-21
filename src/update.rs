//! Self-update via GitHub releases.

use serde_json::Value;
use std::time::Duration;

const REPO: &str = "manor7777/OrdisNexus";
const ASSET: &str = "OrdisNexus.exe";

/// e.g. `0.2.0` for tag builds, `0.2.0-build.7` for branch builds.
pub fn current_version() -> String {
    match option_env!("APP_BUILD") {
        Some(b) if !b.is_empty() => format!("{}-build.{b}", env!("CARGO_PKG_VERSION")),
        _ => env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// `v0.2.0-build.7` -> [0,2,0,7]; no build number (a final release) sorts last.
pub fn parse_version(tag: &str) -> Vec<u64> {
    let t = tag.trim_start_matches('v');
    let (core, build) = match t.split_once("-build.") {
        Some((c, b)) => (c, b.parse().unwrap_or(0)),
        None => (t.split('-').next().unwrap_or(t), u64::MAX),
    };
    let mut v: Vec<u64> = core.split('.').map(|p| p.parse().unwrap_or(0)).collect();
    v.resize(3, 0);
    v.push(build);
    v
}

#[derive(Clone)]
pub struct Release {
    pub tag: String,
    pub page: String,
    pub download: String,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(120))).build().into()
}

/// Returns the newest release if it is newer than this build.
pub fn check() -> Result<Option<Release>, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases?per_page=10");
    let mut resp = agent()
        .get(&url)
        .header("User-Agent", "OrdisNexus-updater")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("update check failed: {e}"))?;
    let text = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let cur = parse_version(&current_version());
    let best = v
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| !r["draft"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let tag = r["tag_name"].as_str()?.to_string();
            let download = r["assets"]
                .as_array()?
                .iter()
                .find(|a| a["name"] == ASSET)?["browser_download_url"]
                .as_str()?
                .to_string();
            Some(Release { page: r["html_url"].as_str().unwrap_or_default().to_string(), tag, download })
        })
        .max_by_key(|r| parse_version(&r.tag));
    Ok(best.filter(|r| parse_version(&r.tag) > cur))
}

/// Downloads the release exe, swaps it in for the running one and restarts.
pub fn install(rel: &Release) -> Result<(), String> {
    let mut resp = agent()
        .get(&rel.download)
        .header("User-Agent", "OrdisNexus-updater")
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let bytes = resp
        .body_mut()
        .with_config()
        .limit(200 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("download failed: {e}"))?;
    if bytes.len() < 100_000 {
        return Err("downloaded file is suspiciously small".into());
    }
    let tmp = std::env::temp_dir().join("OrdisNexus-update.exe");
    std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    self_replace::self_replace(&tmp).map_err(|e| format!("could not replace the running exe: {e}"))?;
    let _ = std::fs::remove_file(&tmp);
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(exe).spawn().map_err(|e| e.to_string())?;
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        assert!(parse_version("v0.2.0-build.2") > parse_version("v0.2.0-build.1"));
        assert!(parse_version("v0.2.0") > parse_version("v0.2.0-build.9"));
        assert!(parse_version("v0.3.0-build.1") > parse_version("v0.2.0"));
        assert!(parse_version("v0.1.0-build.2") < parse_version("0.2.0-build.1"));
    }
}
