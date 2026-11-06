//! Picks item ids for a row of recognized icons from the list order.
//!
//! An icon may stand for several items, but the list is sorted, so a cell's
//! item must come after the item before it and before the item after it.

/// For each cell the sorted list positions its icon allows (`None` when the
/// icon is unknown), returns the position when the order leaves only one.
/// When no ascending choice exists the order is broken (the list is sorted
/// differently, or an unknown item was read as its look-alike), and nothing
/// in the row is trusted.
pub fn resolve<K: Ord + Copy>(options: &[Option<Vec<K>>]) -> Vec<Option<K>> {
    let mut lowest = vec![None; options.len()];
    let mut floor: Option<K> = None;
    for (i, opts) in options.iter().enumerate() {
        let Some(opts) = opts else { continue };
        let Some(&k) = opts.iter().find(|&&k| floor.is_none_or(|f| k > f)) else {
            return vec![None; options.len()];
        };
        lowest[i] = Some(k);
        floor = Some(k);
    }
    let mut highest = vec![None; options.len()];
    let mut ceiling: Option<K> = None;
    for (i, opts) in options.iter().enumerate().rev() {
        let Some(opts) = opts else { continue };
        let Some(&k) = opts.iter().rev().find(|&&k| ceiling.is_none_or(|c| k < c)) else {
            return vec![None; options.len()];
        };
        highest[i] = Some(k);
        ceiling = Some(k);
    }
    lowest
        .into_iter()
        .zip(highest)
        .map(|(lo, hi)| if lo == hi { lo } else { None })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::resolve;

    #[test]
    fn single_items_stay_as_they_are() {
        assert_eq!(
            resolve(&[Some(vec![1]), Some(vec![5]), Some(vec![9])]),
            [Some(1), Some(5), Some(9)]
        );
    }

    #[test]
    fn a_later_item_settles_an_earlier_shared_icon() {
        // The book could be 12 or 16, but the next cell (14) must come after it.
        assert_eq!(
            resolve(&[Some(vec![12, 16]), Some(vec![14])]),
            [Some(12), Some(14)]
        );
    }

    #[test]
    fn an_earlier_item_settles_a_later_shared_icon() {
        assert_eq!(
            resolve(&[Some(vec![14]), Some(vec![12, 16])]),
            [Some(14), Some(16)]
        );
    }

    #[test]
    fn a_shared_icon_with_room_on_both_sides_stays_unknown() {
        assert_eq!(
            resolve(&[Some(vec![10]), Some(vec![12, 16]), Some(vec![20])]),
            [Some(10), None, Some(20)]
        );
    }

    #[test]
    fn unknown_icons_are_skipped() {
        assert_eq!(
            resolve(&[Some(vec![12, 16]), None, Some(vec![14])]),
            [Some(12), None, Some(14)]
        );
    }

    #[test]
    fn a_broken_order_names_nothing() {
        assert_eq!(
            resolve(&[Some(vec![20]), Some(vec![12, 16]), Some(vec![5])]),
            [None, None, None]
        );
    }

    #[test]
    fn the_same_item_twice_names_nothing() {
        // An unknown item read as its look-alike shows up as a repeat.
        assert_eq!(
            resolve(&[Some(vec![7]), Some(vec![8]), Some(vec![8])]),
            [None, None, None]
        );
    }
}
