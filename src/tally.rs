//! Keeps the count of every item seen while the list scrolls.
//!
//! A count is taken once the same value has been read in `CONFIRM` frames in a
//! row, so a single misread frame never changes the tally, while a real change
//! (an item used up in the game) replaces the old value after those frames.

use std::collections::HashMap;

use crate::{Cell, quantity::Count};

/// Frames in a row that must agree before a count is taken.
pub const CONFIRM: u8 = 3;

#[derive(Default)]
pub struct Tally {
    counts: HashMap<i64, Count>,
    /// The latest value read for each item (`None`: lacking from the list) and
    /// in how many frames in a row.
    streaks: HashMap<i64, (Option<Count>, u8)>,
}

impl Tally {
    /// A tally starting from counts kept from before; each stays until its
    /// item is read again.
    pub fn seeded(counts: HashMap<i64, Count>) -> Self {
        Self {
            counts,
            streaks: HashMap::new(),
        }
    }

    /// Take one frame's cells, and the items the frame shows the list lacks
    /// (`gone`); returns whether a count changed. An item lacking in `CONFIRM`
    /// frames in a row is dropped: the game lists no item it has none of.
    pub fn observe(&mut self, cells: &[Cell], gone: &[i64]) -> bool {
        let shown = cells
            .iter()
            .filter(|c| !c.ignored)
            .filter_map(|c| Some((c.id?, Some(c.quantity?))));
        let lacking = gone.iter().map(|&id| (id, None));
        let seen: Vec<(i64, Option<Count>)> = shown.chain(lacking).collect();
        // A run is of frames in a row: one that tells nothing of an item ends it.
        self.streaks
            .retain(|id, _| seen.iter().any(|(s, _)| s == id));
        let mut changed = false;
        for (id, read) in seen {
            let streak = self.streaks.entry(id).or_insert((read, 0));
            if streak.0 == read {
                streak.1 = streak.1.saturating_add(1);
            } else {
                *streak = (read, 1);
            }
            if streak.1 >= CONFIRM && self.counts.get(&id).copied() != read {
                match read {
                    Some(count) => self.counts.insert(id, count),
                    None => self.counts.remove(&id),
                };
                changed = true;
            }
        }
        changed
    }

    /// Takes a count read elsewhere and confirmed there (a coin's held, shown
    /// over a shop's cards); returns whether it changed.
    pub fn take(&mut self, id: i64, count: Count) -> bool {
        self.counts.insert(id, count) != Some(count)
    }

    /// The taken counts by item id.
    pub fn counts(&self) -> &HashMap<i64, Count> {
        &self.counts
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{CONFIRM, Count, Tally};
    use crate::Cell;

    fn cell(id: i64, quantity: u32) -> Cell {
        Cell {
            id: Some(id),
            quantity: Some(Count::Exact(quantity)),
            ignored: false,
        }
    }

    fn frames(tally: &mut Tally, cells: &[Cell], n: u8) -> Vec<bool> {
        (0..n).map(|_| tally.observe(cells, &[])).collect()
    }

    #[test]
    fn a_count_confirmed_elsewhere_is_taken_at_once() {
        let mut tally = Tally::seeded(HashMap::from([(7, Count::Exact(5000))]));
        assert!(tally.take(7, Count::Exact(5940)));
        assert!(!tally.take(7, Count::Exact(5940)));
        assert_eq!(tally.counts().get(&7), Some(&Count::Exact(5940)));
    }

    #[test]
    fn counts_kept_from_before_stay_until_read_again() {
        let kept = HashMap::from([(110, Count::Exact(1875)), (120, Count::Exact(3))]);
        let mut tally = Tally::seeded(kept);
        assert_eq!(tally.counts().get(&120), Some(&Count::Exact(3)));
        frames(&mut tally, &[cell(110, 1900)], CONFIRM);
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(1900)));
        assert_eq!(tally.counts().get(&120), Some(&Count::Exact(3)));
    }

    #[test]
    fn an_item_gone_from_the_list_is_dropped_after_agreeing_frames() {
        let kept = HashMap::from([(110, Count::Exact(5)), (120, Count::Exact(3))]);
        let mut tally = Tally::seeded(kept);
        assert!(!tally.observe(&[], &[110]));
        let changes: Vec<bool> = (1..CONFIRM).map(|_| tally.observe(&[], &[110])).collect();
        assert_eq!(changes.last(), Some(&true));
        assert_eq!(tally.counts().get(&110), None);
        assert_eq!(tally.counts().get(&120), Some(&Count::Exact(3)));
    }

    #[test]
    fn an_item_missed_in_one_frame_stays() {
        let mut tally = Tally::seeded(HashMap::from([(110, Count::Exact(5))]));
        tally.observe(&[], &[110]);
        frames(&mut tally, &[cell(110, 5)], 1);
        tally.observe(&[], &[110]);
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(5)));
    }

    /// Frames that show other parts of the list between break the run: frames
    /// apart that each misread the list (a card pressed) never add up to a
    /// drop, nor to a count.
    #[test]
    fn frames_apart_never_add_up() {
        let mut tally = Tally::seeded(HashMap::from([(110, Count::Exact(5))]));
        for _ in 0..CONFIRM {
            tally.observe(&[], &[110]);
            tally.observe(&[cell(120, 3)], &[]);
        }
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(5)));
        for _ in 0..CONFIRM {
            tally.observe(&[cell(110, 6)], &[]);
            tally.observe(&[cell(120, 3)], &[]);
        }
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(5)));
    }

    #[test]
    fn a_count_is_taken_after_agreeing_frames() {
        let mut tally = Tally::default();
        let changes = frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        assert_eq!(changes.last(), Some(&true));
        assert!(changes[..changes.len() - 1].iter().all(|c| !c));
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(1875)));
    }

    #[test]
    fn one_misread_frame_changes_nothing() {
        let mut tally = Tally::default();
        frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        assert!(!tally.observe(&[cell(110, 187)], &[]));
        frames(&mut tally, &[cell(110, 1875)], 1);
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(1875)));
    }

    #[test]
    fn a_real_change_replaces_the_count() {
        let mut tally = Tally::default();
        frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        let changes = frames(&mut tally, &[cell(110, 1874)], CONFIRM);
        assert_eq!(changes.last(), Some(&true));
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(1874)));
    }

    #[test]
    fn items_out_of_sight_keep_their_count() {
        let mut tally = Tally::default();
        frames(&mut tally, &[cell(110, 1875)], CONFIRM);
        frames(&mut tally, &[cell(111, 597)], CONFIRM);
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(1875)));
        assert_eq!(tally.counts().get(&111), Some(&Count::Exact(597)));
    }

    #[test]
    fn unknown_unread_and_ignored_cells_are_not_counted() {
        let mut tally = Tally::default();
        let cells = [
            Cell {
                id: None,
                quantity: Some(Count::Exact(3)),
                ignored: false,
            },
            Cell {
                id: Some(5),
                quantity: None,
                ignored: false,
            },
            Cell {
                id: Some(80660),
                quantity: Some(Count::Exact(1480)),
                ignored: true,
            },
        ];
        frames(&mut tally, &cells, CONFIRM);
        assert!(tally.counts().is_empty());
    }

    #[test]
    fn a_count_shown_in_thousands_replaces_an_exact_one() {
        let mut tally = Tally::default();
        frames(&mut tally, &[cell(110, 9999)], CONFIRM);
        let rounded = Cell {
            id: Some(110),
            quantity: Some(Count::Thousands(10)),
            ignored: false,
        };
        frames(&mut tally, &[rounded], CONFIRM);
        assert_eq!(tally.counts().get(&110), Some(&Count::Thousands(10)));
    }
}
