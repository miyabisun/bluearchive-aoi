//! Keeps the count of every item seen while the list scrolls.
//!
//! A count is taken once the same value has been read in `CONFIRM` frames in a
//! row, so a single misread frame never changes the tally, while a real change
//! (an item used in the game) replaces the old value after those frames. An item
//! used up leaves the game's list: counted before, it counts none from then on.

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
    /// What `shift` left out of a count in thousands: the count's thousands and this are
    /// how many it took the item for. Gone with the next count taken.
    rest: HashMap<i64, i64>,
}

/// From so many on the game shows a count in thousands.
const IN_THOUSANDS: i64 = 10_000;

impl Tally {
    /// A tally starting from counts kept from before; each stays until its
    /// item is read again.
    pub fn seeded(counts: HashMap<i64, Count>) -> Self {
        Self {
            counts,
            ..Self::default()
        }
    }

    /// Take one frame's cells, and the items the frame shows the list lacks
    /// (`gone`); returns whether a count changed. An item counted before and
    /// lacking in `CONFIRM` frames in a row counts none: the game lists no item it
    /// has none of (the user's of 2026-11-12: 「null化ではなく0個にするようにしてください」;
    /// dropped until then, a coin used up read 「?」 in its shop). An item never
    /// counted stays so: all the entries between two cards lack, held once or not.
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
            if streak.1 < CONFIRM {
                continue;
            }
            let taken = read.or_else(|| self.held(id).then_some(Count::Exact(0)));
            if let Some(count) = taken {
                changed |= self.take(id, count);
            }
        }
        changed
    }

    /// Whether some of the item are held, as counted: not one counted none, nor one never
    /// counted.
    pub fn held(&self, id: i64) -> bool {
        self.counts
            .get(&id)
            .is_some_and(|count| *count != Count::Exact(0))
    }

    /// Takes a count read elsewhere and confirmed there (a coin's held, shown
    /// over a shop's cards); returns whether it changed.
    pub fn take(&mut self, id: i64, count: Count) -> bool {
        self.rest.remove(&id);
        self.counts.insert(id, count) != Some(count)
    }

    /// How many of the item are held as counted, a count in thousands with what `shift` left
    /// out of its thousands; none of one never counted.
    pub fn amount(&self, id: i64) -> Option<u64> {
        let rest = self.rest.get(&id).copied().unwrap_or(0);
        match *self.counts.get(&id)? {
            Count::Exact(n) => Some(u64::from(n)),
            Count::Thousands(k) => u64::try_from(i64::from(k) * 1000 + rest).ok(),
        }
    }

    /// Counts the item `by` more (fewer, below zero) without the list read: what a training
    /// took or a purchase gave (see `crate::ledger`). One never counted stays so. A count in
    /// thousands stays in thousands, rounded as the game rounds it. A count that would fall
    /// under none is forgotten (some were had unseen), and so is one in thousands fallen
    /// under what the game shows so: the number it would show is not known.
    pub fn shift(&mut self, id: i64, by: i64) {
        let (Some(&count), Some(held)) = (self.counts.get(&id), self.amount(id)) else {
            return;
        };
        let now = i64::try_from(held).unwrap_or(i64::MAX).saturating_add(by);
        self.rest.remove(&id);
        let shown = match count {
            Count::Exact(_) => u32::try_from(now).ok().map(Count::Exact),
            Count::Thousands(_) if now < IN_THOUSANDS => None,
            Count::Thousands(_) => {
                let thousands = (now + 500) / 1000;
                self.rest.insert(id, now - thousands * 1000);
                u32::try_from(thousands).ok().map(Count::Thousands)
            }
        };
        match shown {
            Some(count) => {
                self.counts.insert(id, count);
            }
            None => self.forget(id),
        }
    }

    /// Takes the item for one never counted: how many are held is not known any more.
    pub fn forget(&mut self, id: i64) {
        self.counts.remove(&id);
        self.rest.remove(&id);
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

    fn thousands(id: i64, k: u32) -> Cell {
        Cell {
            quantity: Some(Count::Thousands(k)),
            ..cell(id, 0)
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

    /// The game lists no item it has none of: one counted before and lacking from the list in
    /// agreeing frames counts none, and stays counted (the user's of 2026-11-12: 「null化ではなく
    /// 0個にするようにしてください」).
    #[test]
    fn an_item_gone_from_the_list_counts_none_after_agreeing_frames() {
        let kept = HashMap::from([(110, Count::Exact(5)), (120, Count::Exact(3))]);
        let mut tally = Tally::seeded(kept);
        assert!(!tally.observe(&[], &[110]));
        let changes: Vec<bool> = (1..CONFIRM).map(|_| tally.observe(&[], &[110])).collect();
        assert_eq!(changes.last(), Some(&true));
        assert_eq!(tally.counts().get(&110), Some(&Count::Exact(0)));
        assert_eq!(tally.counts().get(&120), Some(&Count::Exact(3)));
        // Counted none already, lacking again is no change.
        assert!(!tally.observe(&[], &[110]));
        // None of it are held; of an item never counted neither.
        assert!(!tally.held(110) && tally.held(120) && !tally.held(130));
    }

    /// An item never counted is not counted none for lacking from the list: every entry
    /// between two cards lacks, held once or not.
    #[test]
    fn an_item_never_counted_stays_uncounted_when_the_list_lacks_it() {
        let mut tally = Tally::default();
        let changes: Vec<bool> = (0..CONFIRM).map(|_| tally.observe(&[], &[110])).collect();
        assert!(changes.iter().all(|changed| !changed));
        assert_eq!(tally.counts().get(&110), None);
    }

    /// A coin counted none in its shop stays so when the item list, read after, lacks it.
    #[test]
    fn a_count_of_none_taken_elsewhere_stays_when_the_list_lacks_the_item() {
        let mut tally = Tally::default();
        assert!(tally.take(9, Count::Exact(0)));
        let changes: Vec<bool> = (0..CONFIRM).map(|_| tally.observe(&[], &[9])).collect();
        assert!(changes.iter().all(|changed| !changed));
        assert_eq!(tally.counts().get(&9), Some(&Count::Exact(0)));
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
    fn a_count_shifts_by_what_was_spent_or_bought() {
        let mut tally = Tally::seeded(HashMap::from([(1, Count::Exact(120))]));
        tally.shift(1, -100);
        assert_eq!(tally.counts().get(&1), Some(&Count::Exact(20)));
        tally.shift(1, 5);
        assert_eq!(tally.amount(1), Some(25));
        // One never counted stays so: how many there were is not known.
        tally.shift(2, 5);
        assert_eq!((tally.counts().get(&2), tally.amount(2)), (None, None));
    }

    /// 51K less 300 four times is 49.8K: each alone would round back to 51K.
    #[test]
    fn a_count_in_thousands_keeps_what_its_thousands_leave_out() {
        let mut tally = Tally::seeded(HashMap::from([(2, Count::Thousands(51))]));
        let shown: Vec<Count> = (0..4)
            .map(|_| {
                tally.shift(2, -300);
                tally.counts()[&2]
            })
            .collect();
        let k = Count::Thousands;
        assert_eq!(shown, [k(51), k(50), k(50), k(50)]);
        assert_eq!(tally.amount(2), Some(49_800));
        // Read again, the count is the list's alone.
        tally.take(2, k(50));
        assert_eq!(tally.amount(2), Some(50_000));
        tally.shift(2, -300);
        frames(&mut tally, &[thousands(2, 50)], CONFIRM);
        assert_eq!(tally.amount(2), Some(50_000));
    }

    /// A count short of what was spent tells of some had unseen, and one in thousands fallen
    /// under what the game shows in thousands has no number to show: neither is known.
    #[test]
    fn a_count_fallen_short_or_out_of_the_thousands_is_forgotten() {
        let kept = HashMap::from([(1, Count::Exact(20)), (2, Count::Thousands(10))]);
        let mut tally = Tally::seeded(kept);
        tally.shift(1, -30);
        tally.shift(2, -600);
        assert_eq!((tally.amount(1), tally.amount(2)), (None, None));
        assert!(tally.counts().is_empty());
        // Forgotten, an item is one never counted: lacking from the list it counts nothing.
        let changes: Vec<bool> = (0..CONFIRM).map(|_| tally.observe(&[], &[1])).collect();
        assert!(changes.iter().all(|changed| !changed));
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
}
