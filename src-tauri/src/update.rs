//! Updating the app from its GitHub releases: the latest release's exe,
//! checked against the SHA-256 published beside it, takes the running exe's
//! place, which is kept aside until the next start. On macOS the app is not
//! replaced: the release page is opened to take its disk image by hand.

use std::{fmt::Write as _, io::Read as _, path::PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

const RELEASES: &str = "https://api.github.com/repos/miyabisun/bluearchive-aoi/releases/latest";
/// The release's file for this OS, and its SHA-256 beside it with this suffix.
#[cfg(windows)]
const ASSET: &str = "bluearchive-aoi-x86_64-pc-windows-msvc.exe";
#[cfg(not(windows))]
const ASSET: &str = "bluearchive-aoi-universal-apple-darwin.dmg";
const SHA: &str = ".sha256";
/// The release exe is about 20 MB; a download past this is no release exe.
const MAX_EXE: u64 = 200 * 1024 * 1024;

/// A GitHub release as its API tells it.
#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// An update to take: its version ("v0.1.30") and where its exe and the
/// exe's SHA-256 are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub version: String,
    pub exe: String,
    pub sha: String,
}

/// A version's numbers ("v0.1.30" or "0.1.30" is [0, 1, 30]).
fn numbers(version: &str) -> Option<Vec<u64>> {
    version
        .trim_start_matches('v')
        .split('.')
        .map(|n| n.parse().ok())
        .collect()
}

/// The update `release` brings to the app at `current`, if it is newer and
/// carries the exe and its SHA-256.
pub fn update_of(release: &Release, current: &str) -> Option<Update> {
    if numbers(&release.tag_name)? <= numbers(current)? {
        return None;
    }
    let url = |name: &str| {
        release
            .assets
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.browser_download_url.clone())
    };
    Some(Update {
        version: release.tag_name.clone(),
        exe: url(ASSET)?,
        sha: url(&format!("{ASSET}{SHA}"))?,
    })
}

/// Whether `bytes` are what the SHA-256 file `published` names (its first
/// word, the hash in hex).
pub fn verified(bytes: &[u8], published: &str) -> bool {
    let hash = Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        });
    published
        .split_whitespace()
        .next()
        .is_some_and(|want| want.eq_ignore_ascii_case(&hash))
}

fn get(url: &str) -> Result<ureq::Response, String> {
    ureq::get(url)
        .set("User-Agent", "bluearchive-aoi")
        .call()
        .map_err(|e| e.to_string())
}

/// The latest release on GitHub.
pub fn latest() -> Result<Release, String> {
    let text = get(RELEASES)?.into_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Where the running exe is kept aside while its update takes its place.
fn aside(exe: &std::path::Path) -> PathBuf {
    exe.with_extension("old")
}

/// Downloads `update`'s exe, checks it against its SHA-256, and puts it where
/// the running exe is, which is kept aside (see `clean_up`).
pub fn install(update: &Update) -> Result<(), String> {
    let published = get(&update.sha)?.into_string().map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    get(&update.exe)?
        .into_reader()
        .take(MAX_EXE)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if !verified(&bytes, &published) {
        return Err("ダウンロードした実行ファイルの SHA-256 がリリースと一致しません".to_owned());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new = exe.with_extension("new");
    std::fs::write(&new, &bytes).map_err(|e| e.to_string())?;
    let old = aside(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        // Put the running exe back where it was.
        let _ = std::fs::rename(&old, &exe);
        return Err(e.to_string());
    }
    Ok(())
}

/// Opens the latest release's page, where the app cannot replace itself.
#[cfg(not(windows))]
pub fn open_page() -> Result<(), String> {
    let page = "https://github.com/miyabisun/bluearchive-aoi/releases/latest";
    let status = std::process::Command::new("open")
        .arg(page)
        .status()
        .map_err(|e| e.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("リリースのページを開けませんでした（{status}）"))
}

/// Removes the exe an update kept aside, once the updated one runs.
pub fn clean_up() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::fs::remove_file(aside(&exe));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, names: &[&str]) -> Release {
        Release {
            tag_name: tag.to_owned(),
            assets: names
                .iter()
                .map(|n| Asset {
                    name: (*n).to_owned(),
                    browser_download_url: format!("https://example.test/{tag}/{n}"),
                })
                .collect(),
        }
    }

    #[test]
    fn a_newer_release_with_its_exe_and_hash_is_an_update() {
        let sha = format!("{ASSET}.sha256");
        let both = [ASSET, &sha];
        let update = update_of(&release("v0.1.30", &both), "0.1.29").unwrap();
        assert_eq!(update.version, "v0.1.30");
        assert!(update.exe.ends_with(&format!("/v0.1.30/{ASSET}")));
        assert!(update.sha.ends_with(&format!("/v0.1.30/{sha}")));
        // Numbers compare as numbers, not text.
        assert!(update_of(&release("v0.1.100", &both), "0.1.99").is_some());
        // The same or an older version, or without its hash, is none.
        assert_eq!(update_of(&release("v0.1.29", &both), "0.1.29"), None);
        assert_eq!(update_of(&release("v0.1.28", &both), "0.1.29"), None);
        assert_eq!(update_of(&release("v0.1.30", &[ASSET]), "0.1.29"), None);
    }

    #[test]
    fn a_download_is_taken_only_as_its_published_hash() {
        // SHA-256 of "abc".
        let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verified(b"abc", &format!("{hash}  {ASSET}\n")));
        assert!(verified(b"abc", &hash.to_uppercase()));
        assert!(!verified(b"abd", hash));
        assert!(!verified(b"abc", ""));
    }
}
