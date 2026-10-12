//! The window shows each item and piece of equipment by its icon, which the
//! program carries.

use std::path::Path;

use aoi::{List, Recognizer};

#[test]
fn every_item_and_piece_of_equipment_has_its_icon_built_in() {
    let recognizer = Recognizer::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
    let missing: Vec<String> = [List::Items, List::Equipment]
        .into_iter()
        .flat_map(|list| recognizer.catalog(list).entries().iter())
        .filter(|e| aoi::icon(&e.icon).is_none())
        .map(|e| format!("{} {}", e.name, e.icon))
        .collect();
    assert!(missing.is_empty(), "{missing:?}");
}
