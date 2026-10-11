//! Keeps the count of every item seen while the list scrolls.
//!
//! A count is taken once the same value has been read in `CONFIRM` frames in a
//! row, so a single misread frame never changes the tally, while a real change
//! (an item used up in the game) replaces the old value after those frames.

use std::collections::HashMap;

use crate::Cell;

/// Frames in a row that must agree before a count is taken.
pub const CONFIRM: u8 = 3;

#[derive(Default)]
pub struct Tally {
    counts: HashMap<i64, u32>,
    /// The latest value read for each item and in how many frames in a row.
    streaks: HashMap<i64, (u32, u8)>,
}

impl Tally {
    /// Take one frame's cells; returns whether a count changed.
    pub fn observe(&mut self, cells: &[Cell]) -> bool {
        let mut changed = false;
        for cell in cells.iter().filter(|c| !c.ignored) {
            let (Some(id), Some(quantity)) = (cell.id, cell.quantity) else {
                continue;
            };
            let streak = self.streaks.entry(id).or_insert((quantity, 0));
            if streak.0 == quantity {
                streak.1 = streak.1.saturating_add(1);
            } else {
                *streak = (quantity, 1);
            }
            if streak.1 >= CONFIRM && self.counts.get(&id) != Some(&quantity) {
                self.counts.insert(id, quantity);
                changed = true;
            }
        }
        changed
    }

    /// The taken counts by item id.
    pub fn counts(&self) -> &HashMap<i64, u32> {
        &self.counts
    }
}

#[cfg(test)]
mod tests {
    use super::{CONFIRM, Tally};
    use crate::Cell;

    fn cell(id: i64, quantity: u32) -> Cell {
        Cell {
            id: Some(id),
            quantity: Some(quantity),
            ignored: false,
        }
    }

    fn frames(tally: &mut Tally, cells: &[Cell], n: u8) -> Vec<bool> {
        (0..n).map(|_| tally.observe(cells)).collect()
    }

    #[test]
    fn a_count_is_taken_after_agreeing_frames() {
        let mut tally = Tally::default();
        let changes = frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        assert_eq!(changes.last(), Some(&true));
        assert!(changes[..changes.len() - 1].iter().all(|c| !c));
        assert_eq!(tally.counts().get(&110), Some(&1875));
    }

    #[test]
    fn one_misread_frame_changes_nothing() {
        let mut tally = Tally::default();
        frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        assert!(!tally.observe(&[cell(110, 187)]));
        frames(&mut tally, &[cell(110, 1875)], 1);
        assert_eq!(tally.counts().get(&110), Some(&1875));
    }

    #[test]
    fn a_real_change_replaces_the_count() {
        let mut tally = Tally::default();
        frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        let changes = frames(&mut tally, &[cell(110, 1874)], CONFIRM);
        assert_eq!(changes.last(), Some(&true));
        assert_eq!(tally.counts().get(&110), Some(&1874));
    }

    #[test]
    fn items_out_of_sight_keep_their_count() {
        let mut tally = Tally::default();
        frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        frames(&mut tally, &[cell(111, 597)], CONFIRM);
        assert_eq!(tally.counts().get(&110), Some(&1875));
        assert_eq!(tally.counts().get(&111), Some(&597));
    }

    #[test]
    fn unknown_unread_and_ignored_cells_are_not_counted() {
        let mut tally = Tally::default();
        let cells = [
            Cell {
                id: None,
                quantity: Some(3),
                ignored: false,
            },
            Cell {
                id: Some(5),
                quantity: None,
                ignored: false,
            },
            Cell {
                id: Some(80660),
                quantity: Some(1480),
                ignored: true,
            },
        ];
        frames(&mut tally, &cells, CONFIRM);
        assert!(tally.counts().is_empty());
    }
}
