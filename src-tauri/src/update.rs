//! Updating the app from its GitHub releases: the latest release's exe,
//! checked against the SHA-256 published beside it, takes the running exe's
//! place, which is kept aside until the next start. On macOS the app is not
//! replaced: the release page is opened to take its disk image by hand.
//!
//! The latest release is told by where its web page sends to, not by GitHub's
//! API: the API allows a home 60 calls an hour without a token, shared by every
//! machine there, and once they were spent the check failed (2026-10-26).

use std::{fmt::Write as _, io::Read as _, path::PathBuf};

use sha2::{Digest, Sha256};

/// The repository's web address: `/releases/latest` sends to the latest
/// release's tag (`/releases/tag/v0.2.55`), or to `/releases` when there is
/// none, and its files lie at `/releases/download/<tag>/<name>`.
const REPOSITORY: &str = "https://github.com/miyabisun/bluearchive-aoi";
/// The release's file for this OS, and its SHA-256 beside it with this suffix.
#[cfg(windows)]
const ASSET: &str = "bluearchive-aoi-x86_64-pc-windows-msvc.exe";
#[cfg(not(windows))]
const ASSET: &str = "bluearchive-aoi-universal-apple-darwin.dmg";
const SHA: &str = ".sha256";
/// The release exe is about 20 MB; a download past this is no release exe.
const MAX_EXE: u64 = 200 * 1024 * 1024;

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

/// The tag of the release whose page `address` is (where `/releases/latest`
/// sent to), if it is one.
pub fn tag_of(address: &str) -> Option<&str> {
    let tag = address
        .strip_prefix(REPOSITORY)?
        .strip_prefix("/releases/tag/")?;
    (tag.starts_with('v') && numbers(tag).is_some()).then_some(tag)
}

/// The update the release `tag` brings to the app at `current`, if it is newer:
/// where its exe and the exe's SHA-256 are.
pub fn update_of(tag: &str, current: &str) -> Option<Update> {
    if numbers(tag)? <= numbers(current)? {
        return None;
    }
    let files = format!("{REPOSITORY}/releases/download/{tag}");
    Some(Update {
        version: tag.to_owned(),
        exe: format!("{files}/{ASSET}"),
        sha: format!("{files}/{ASSET}{SHA}"),
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

/// The latest release's tag on GitHub, from where its web page sends to.
pub fn latest() -> Result<String, String> {
    let page = format!("{REPOSITORY}/releases/latest");
    let response = ureq::head(&page)
        .set("User-Agent", "bluearchive-aoi")
        .call()
        .map_err(|e| e.to_string())?;
    tag_of(response.get_url())
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "公開されたリリースが見つかりません（{} へ転送）",
                response.get_url()
            )
        })
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
    let page = format!("{REPOSITORY}/releases/latest");
    let status = std::process::Command::new("open")
        .arg(&page)
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

    #[test]
    fn the_latest_tag_is_where_the_latest_release_page_sends_to() {
        let page =
            |rest: &str| format!("https://github.com/miyabisun/bluearchive-aoi/releases{rest}");
        assert_eq!(tag_of(&page("/tag/v0.2.55")), Some("v0.2.55"));
        // No release published: the list of releases.
        assert_eq!(tag_of(&page("")), None);
        assert_eq!(tag_of(&page("/latest")), None);
        assert_eq!(tag_of(&page("/tag/")), None);
        assert_eq!(tag_of(&page("/tag/nightly")), None);
        assert_eq!(tag_of(&page("/tag/v0.2.55/extra")), None);
        assert_eq!(
            tag_of("https://github.com/someone/else/releases/tag/v9.0.0"),
            None
        );
    }

    #[test]
    fn a_newer_release_is_an_update_from_its_download_addresses() {
        let update = update_of("v0.1.30", "0.1.29").unwrap();
        assert_eq!(update.version, "v0.1.30");
        let files = "https://github.com/miyabisun/bluearchive-aoi/releases/download/v0.1.30";
        assert_eq!(update.exe, format!("{files}/{ASSET}"));
        assert_eq!(update.sha, format!("{files}/{ASSET}.sha256"));
        // Numbers compare as numbers, not text.
        assert!(update_of("v0.1.100", "0.1.99").is_some());
        // The same or an older version is none.
        assert_eq!(update_of("v0.1.29", "0.1.29"), None);
        assert_eq!(update_of("v0.1.28", "0.1.29"), None);
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
