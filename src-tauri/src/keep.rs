//! What Aoi keeps between runs: the counts of the item and equipment lists and
//! the students' training, with when each was last read. Kept as JSON in the
//! app's own folder; the window starts from it and goes on reading.

use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use crate::{plan::Target, quantity::Count, shop::Shelf, strategy::Reserve, student::Training};

/// When each list was last read (local time, "2026-10-12 15:30:00").
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReadAt {
    pub items: Option<String>,
    pub equipment: Option<String>,
    pub students: Option<String>,
    pub credits: Option<String>,
}

/// Everything read, by id. What an older version did not keep comes back as
/// nothing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kept {
    pub items: BTreeMap<i64, Count>,
    pub equipment: BTreeMap<i64, Count>,
    pub students: BTreeMap<i64, Training>,
    pub credits: Option<u64>,
    /// The shops' shelves by shop.
    pub shops: BTreeMap<String, Shelf>,
    /// The students marked for training, by id, and their targets.
    pub targets: BTreeMap<i64, Target>,
    /// The month-end shops' cards reserved.
    pub reserved: std::collections::BTreeSet<Reserve>,
    pub read_at: ReadAt,
}

/// The local time, as the kept data and the exports write it.
pub fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// What `path` keeps; nothing yet when there is no such file.
pub fn load(path: &Path) -> Result<Kept, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Kept::default()),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// Keeps `kept` in `path`, written whole beside it first so a run stopped
/// halfway never leaves half a file.
pub fn save(path: &Path, kept: &Kept) -> Result<(), String> {
    write(path, &serde_json::to_vec(kept).map_err(|e| e.to_string())?)
}

/// Writes `bytes` to `path` (making its folder) through a file beside it.
pub fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let fail = |e: std::io::Error| format!("{}: {e}", path.display());
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(fail)?;
    }
    let part = path.with_extension("part");
    std::fs::write(&part, bytes).map_err(fail)?;
    std::fs::rename(&part, path).map_err(fail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        shop::Offer,
        student::{Gear, Weapon},
    };

    /// A folder of its own under the temporary one, emptied.
    fn folder(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("aoi-keep-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn kept() -> Kept {
        Kept {
            items: BTreeMap::from([(1, Count::Exact(1383)), (2, Count::Thousands(51))]),
            equipment: BTreeMap::from([(1000, Count::Exact(249))]),
            students: BTreeMap::from([(
                10059,
                Training {
                    level: Some(90),
                    rarity: Some(5),
                    bond: None,
                    skills: [Some(5), Some(10), Some(10), Some(10)],
                    weapon: Some(Some(Weapon {
                        level: 50,
                        stars: 3,
                    })),
                    equipment: [
                        Some(Gear {
                            tier: 10,
                            level: 70,
                        }),
                        None,
                        None,
                    ],
                    gear: Some(0),
                    ability: [Some(25), Some(25), Some(0)],
                },
            )]),
            credits: Some(244_213_276),
            shops: BTreeMap::from([(
                "総力戦".to_owned(),
                Shelf {
                    read_at: "2026-10-12 22:00:00".to_owned(),
                    offers: vec![Offer {
                        id: 9000,
                        coin: 1,
                        left: 0,
                        price: None,
                    }],
                    held: vec![Some(530), None],
                },
            )]),
            targets: BTreeMap::from([(10059, Target::default())]),
            reserved: std::collections::BTreeSet::from([Reserve {
                shop: "総力戦".to_owned(),
                id: 10059,
                coin: "総力戦コイン".to_owned(),
            }]),
            read_at: ReadAt {
                items: Some("2026-10-12 15:30:00".to_owned()),
                equipment: None,
                students: Some("2026-10-12 16:00:00".to_owned()),
                credits: None,
            },
        }
    }

    #[test]
    fn what_is_kept_comes_back() {
        let dir = folder("back");
        let path = dir.join("state.json");
        save(&path, &kept()).unwrap();
        assert_eq!(load(&path).unwrap(), kept());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unread_weapon_and_a_locked_one_stay_apart() {
        let dir = folder("weapon");
        let path = dir.join("state.json");
        let mut both = kept();
        let mut locked = both.students[&10059];
        locked.weapon = Some(None);
        let mut unread = locked;
        unread.weapon = None;
        both.students.insert(1, locked);
        both.students.insert(2, unread);
        save(&path, &both).unwrap();
        assert_eq!(load(&path).unwrap(), both);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_an_older_version_kept_still_comes_back() {
        let dir = folder("older");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, r#"{"items":{"1":{"exact":5}},"equipment":{},"students":{},"read_at":{"items":"2026-10-12 15:30:00","equipment":null,"students":null}}"#).unwrap();
        let kept = load(&path).unwrap();
        assert_eq!(kept.items.get(&1), Some(&Count::Exact(5)));
        assert_eq!(kept.credits, None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_is_kept_before_the_first_run() {
        assert_eq!(
            load(&folder("none").join("state.json")).unwrap(),
            Kept::default()
        );
    }

    #[test]
    fn a_broken_file_is_told_not_taken_for_nothing() {
        let dir = folder("broken");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
