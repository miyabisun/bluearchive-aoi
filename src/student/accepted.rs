//! Numbers the students in the order they were accepted, from what frames of
//! the student list show (see `list`): the student accepted first is 1, each
//! one after a number higher, so a student keeps the number for good as others
//! join (the user's 2026-11-24 「生徒の受け入れ順番を生徒のindex値として保存」).
//!
//! A frame shows a stretch of the list, a place after another. Where the
//! numbering starts shows in two ways: with the newest first (the game's
//! default), the held card before the list's first grey or missing card is the
//! student accepted first; with the oldest first, the list's first card is.
//! Any other frame counts from a student whose number is known. Students seen
//! beside one another before any of them has a number are kept as they stand
//! (`loose`, in memory alone) and numbered together once a frame ties them to
//! a number. A frame that contradicts what is known is not taken at all, and
//! no number once given is changed.

use std::collections::BTreeMap;

use super::list::{Cell, Seen, Sight, apart};
use crate::tally::CONFIRM;

/// What became of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Took {
    /// It names no student.
    Nothing,
    /// Its students stand beside one another, none with a number yet.
    Loose,
    /// It gave this many students their numbers (none where all were known).
    Numbered(usize),
    /// It contradicts itself or what is known, and why.
    Refused(&'static str),
}

/// The students' numbers so far.
#[derive(Debug, Default)]
pub struct Accepted {
    known: BTreeMap<i64, u32>,
    /// Students seen beside one another whose numbers are not known yet: each
    /// group's students by where they stand, a number less a shift not known.
    loose: Vec<BTreeMap<i64, i64>>,
}

impl Accepted {
    /// From the numbers kept from before.
    pub fn seeded(known: BTreeMap<i64, u32>) -> Self {
        Self {
            known,
            loose: Vec::new(),
        }
    }

    /// Each student's number, by id.
    pub fn known(&self) -> &BTreeMap<i64, u32> {
        &self.known
    }

    /// Takes what a frame shows for sure.
    pub fn observe(&mut self, sight: &Sight) -> Took {
        let stretch = match Stretch::of(sight) {
            Ok(stretch) => stretch,
            Err(why) => return Took::Refused(why),
        };
        if stretch.students.is_empty() {
            return Took::Nothing;
        }
        match self.base(&stretch) {
            Err(why) => Took::Refused(why),
            Ok(Some(base)) => match self.number(&stretch, base) {
                Ok(new) => Took::Numbered(new),
                Err(why) => Took::Refused(why),
            },
            Ok(None) => match self.join(&stretch.students) {
                Ok(()) => Took::Loose,
                Err(why) => Took::Refused(why),
            },
        }
    }

    /// The number of the stretch's place 0, where its start of the numbers or
    /// a student known tells it; an error where they tell it differently.
    fn base(&self, stretch: &Stretch) -> Result<Option<i64>, &'static str> {
        let known = stretch
            .students
            .iter()
            .filter_map(|(id, at)| Some(i64::from(*self.known.get(id)?) - at));
        let mut bases = stretch.base.into_iter().chain(known);
        let Some(base) = bases.next() else {
            return Ok(None);
        };
        if bases.any(|other| other != base) {
            return Err("the students known stand elsewhere");
        }
        Ok(Some(base))
    }

    /// Gives the stretch's students their numbers, place 0's being `base`, and
    /// with them the students hanging loose beside them; how many are new.
    fn number(&mut self, stretch: &Stretch, base: i64) -> Result<usize, &'static str> {
        // Every held card takes a number from 1 up; a card not held stands
        // past the students', before 1 or after the highest.
        if stretch.held.iter().any(|at| base + at < 1) {
            return Err("a held card stands before the student accepted first");
        }
        let highest = self.known.values().max().map_or(0, |&n| i64::from(n));
        if stretch
            .others
            .iter()
            .any(|at| (1..=highest).contains(&(base + at)))
        {
            return Err("a card not held stands among the students known");
        }
        let numbers: Vec<(i64, i64)> = stretch
            .students
            .iter()
            .map(|(&id, at)| (id, base + at))
            .collect();
        if !self.free(&numbers) {
            return Err("a number is another student's");
        }
        let mut new = self.take(&numbers);
        // The groups hanging loose that hold one of these students.
        let (tied, loose): (Vec<_>, Vec<_>) = std::mem::take(&mut self.loose)
            .into_iter()
            .partition(|group| numbers.iter().any(|(id, _)| group.contains_key(id)));
        self.loose = loose;
        for group in tied {
            let shifts: Vec<i64> = numbers
                .iter()
                .filter_map(|(id, number)| Some(number - group.get(id)?))
                .collect();
            let shift = shifts[0];
            let numbers: Vec<(i64, i64)> = group.iter().map(|(&id, at)| (id, at + shift)).collect();
            // At odds with the numbers, the group was seen wrong: dropped.
            if shifts.iter().all(|&s| s == shift)
                && numbers.iter().all(|&(_, number)| number >= 1)
                && self.free(&numbers)
            {
                new += self.take(&numbers);
            }
        }
        Ok(new)
    }

    /// Whether each of `numbers` is its student's already, or no one's.
    fn free(&self, numbers: &[(i64, i64)]) -> bool {
        numbers.iter().all(|&(id, number)| {
            self.known
                .iter()
                .all(|(&other, &has)| (other == id) == (i64::from(has) == number))
        })
    }

    /// Keeps `numbers`; how many of them are new.
    fn take(&mut self, numbers: &[(i64, i64)]) -> usize {
        let mut new = 0;
        for &(id, number) in numbers {
            let number = u32::try_from(number).expect("a number checked to be 1 or more");
            new += usize::from(self.known.insert(id, number).is_none());
        }
        new
    }

    /// Keeps `students` (each by where it stands) with the groups hanging
    /// loose that hold some of them, as one group; an error, and nothing
    /// changed, where they stand differently there.
    fn join(&mut self, students: &BTreeMap<i64, i64>) -> Result<(), &'static str> {
        let mut joined = students.clone();
        let mut apart = Vec::new();
        for group in &self.loose {
            let mut shifts = students
                .iter()
                .filter_map(|(id, at)| Some(at - group.get(id)?));
            let Some(shift) = shifts.next() else {
                apart.push(group.clone());
                continue;
            };
            if shifts.any(|other| other != shift) {
                return Err("the students seen before stand elsewhere");
            }
            for (&id, at) in group {
                if *joined.entry(id).or_insert(at + shift) != at + shift {
                    return Err("the students seen before stand elsewhere");
                }
            }
        }
        // Two students at one place: one of them was told wrong.
        let mut places: Vec<i64> = joined.values().copied().collect();
        places.sort_unstable();
        if places.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err("two students seen at one place");
        }
        apart.push(joined);
        self.loose = apart;
        Ok(())
    }
}

/// A frame's stretch of the list as the numbers see it: every place by how
/// much its number is above place 0's (the places count down with the newest
/// first, up with the oldest first).
struct Stretch {
    /// The students named, by where they stand.
    students: BTreeMap<i64, i64>,
    /// Where held cards stand, named or not.
    held: Vec<i64>,
    /// Where cards not held stand, grey or missing.
    others: Vec<i64>,
    /// Place 0's number, where the frame shows the numbers' start.
    base: Option<i64>,
}

impl Stretch {
    fn of(sight: &Sight) -> Result<Self, &'static str> {
        let way: i64 = if sight.ascending { 1 } else { -1 };
        let told = sight
            .cells
            .iter()
            .enumerate()
            .filter_map(|(place, cell)| Some((place, (*cell)?)));
        let (mut students, mut held, mut others) = (BTreeMap::new(), Vec::new(), Vec::new());
        // The held cards come first: the grey ones start a row of their own
        // after them (the rest of the held cards' last row stands empty), so
        // no held card follows a grey or a missing one.
        let mut past_held = false;
        for (place, cell) in told {
            if past_held && matches!(cell, Cell::Held(_)) {
                return Err("a held card stands after one not held");
            }
            past_held |= !matches!(cell, Cell::Held(_));
            let at = way * place as i64;
            match cell {
                Cell::Held(id) => {
                    held.push(at);
                    if let Some(id) = id
                        && students.insert(id, at).is_some()
                    {
                        return Err("a student stands twice");
                    }
                }
                Cell::Lacked | Cell::Empty => others.push(at),
            }
        }
        // The numbers' start: with the newest first, the held card right
        // before a card not held; with the oldest first, the list's first.
        let is_held = |place: usize| matches!(sight.cells.get(place), Some(Some(Cell::Held(_))));
        let start = if sight.ascending {
            sight.first.filter(|&first| is_held(first))
        } else {
            (0..sight.cells.len()).find(|&place| {
                is_held(place)
                    && matches!(
                        sight.cells.get(place + 1),
                        Some(Some(Cell::Lacked | Cell::Empty))
                    )
            })
        };
        Ok(Self {
            students,
            held,
            others,
            base: start.map(|place| 1 - way * place as i64),
        })
    }
}

/// The numbers as the frames of the list come in: each frame's steady part
/// (see `Steady`) taken into the numbers.
#[derive(Debug, Default)]
pub struct Counting {
    steady: Steady,
    accepted: Accepted,
}

impl Counting {
    /// From the numbers kept from before.
    pub fn seeded(known: BTreeMap<i64, u32>) -> Self {
        Self {
            steady: Steady::default(),
            accepted: Accepted::seeded(known),
        }
    }

    /// Each student's number, by id.
    pub fn known(&self) -> &BTreeMap<i64, u32> {
        self.accepted.known()
    }

    /// Takes a frame of the list; what became of the stretch the frames agree
    /// on. A stretch at odds with the numbers is not trusted further: the
    /// frames start a new one.
    pub fn frame(&mut self, seen: &Seen) -> Took {
        let took = self.accepted.observe(&self.steady.observe(seen));
        if matches!(took, Took::Refused(_)) {
            self.away();
        }
        took
    }

    /// The game shows another screen: the frames before no longer count.
    pub fn away(&mut self) {
        self.steady = Steady::default();
    }
}

/// Lays the frames of the list over one another as it scrolls and keeps the
/// stretch they agree on: a place counts once `CONFIRM` frames in a row show
/// the same there, so a frame caught while the list changes never counts (as
/// frames in a row make a count or a training, see `tally`). Two frames are
/// laid over one another by the students both name, or else by the cards'
/// faces (the list jumps a wheel's notch at a time, so frames in a row may
/// name no student in common); where neither tells how, or the two tell it
/// differently, the stretch starts over.
#[derive(Debug, Default)]
pub struct Steady {
    /// The frames before, the newest last: each with how many places it
    /// starts ahead of the newest (the newest frame's place `p` is its place
    /// `p` more that).
    before: Vec<(i64, Seen)>,
    /// What the frames agreed on, by place as the newest frame counts them.
    stretch: BTreeMap<i64, Cell>,
    /// The place of the list's first card, where the frames agreed on it.
    first: Option<i64>,
}

/// A stretch longer than this is no list of students (the game has some 300).
const LONGEST: i64 = 4096;

impl Steady {
    /// Takes a frame; the stretch of the list the frames agree on so far.
    pub fn observe(&mut self, seen: &Seen) -> Sight {
        let sight = &seen.sight;
        match self.before.last().and_then(|(_, last)| ahead(last, seen)) {
            Some(ahead) => {
                // Everything kept starts as many places further ahead.
                self.before.iter_mut().for_each(|(by, _)| *by += ahead);
                self.stretch = std::mem::take(&mut self.stretch)
                    .into_iter()
                    .map(|(place, cell)| (place - ahead, cell))
                    .collect();
                self.first = self.first.map(|first| first - ahead);
            }
            None => self.forget(),
        }
        let enough = self.before.len() + 1 >= usize::from(CONFIRM);
        let agreed = |place: usize| {
            self.before.iter().all(|(by, before)| {
                let there = usize::try_from(place as i64 + by).ok();
                there.and_then(|at| before.sight.cells.get(at)) == Some(&sight.cells[place])
            })
        };
        let cells: Vec<(i64, Cell)> = (0..sight.cells.len())
            .filter(|&place| enough && agreed(place))
            .filter_map(|place| Some((place as i64, sight.cells[place]?)))
            .collect();
        let first = sight.first.map(|first| first as i64).filter(|first| {
            let starts = |(by, before): &(i64, Seen)| before.sight.first.map(|at| at as i64 - by);
            enough
                && self
                    .before
                    .iter()
                    .all(|before| starts(before) == Some(*first))
        });
        // At odds with what was agreed on before, neither is trusted.
        let fits = cells.iter().all(|(place, cell)| {
            self.stretch
                .get(place)
                .is_none_or(|kept| same_card(*kept, *cell))
        }) && (first.is_none() || self.first.is_none() || first == self.first);
        if fits {
            for (place, cell) in cells {
                let kept = self.stretch.entry(place).or_insert(cell);
                // A card named keeps its name where a later frame does not tell it.
                if matches!(kept, Cell::Held(None)) {
                    *kept = cell;
                }
            }
            self.first = self.first.or(first);
        } else {
            self.forget();
        }
        self.before.push((0, seen.clone()));
        let spare = self.before.len().saturating_sub(usize::from(CONFIRM) - 1);
        self.before.drain(..spare);
        self.told(sight.ascending)
    }

    /// Starts over: nothing of the frames before counts.
    fn forget(&mut self) {
        *self = Self::default();
    }

    /// The stretch as a run of places, the first place told at its start.
    fn told(&mut self, ascending: bool) -> Sight {
        let (Some((&from, _)), Some((&to, _))) = (
            self.stretch.first_key_value(),
            self.stretch.last_key_value(),
        ) else {
            return Sight {
                ascending,
                cells: Vec::new(),
                first: None,
            };
        };
        if to - from >= LONGEST {
            self.forget();
            return self.told(ascending);
        }
        Sight {
            ascending,
            cells: (from..=to)
                .map(|place| self.stretch.get(&place).copied())
                .collect(),
            first: self
                .first
                .and_then(|first| usize::try_from(first - from).ok()),
        }
    }
}

/// Whether two frames' readings of a place may be of one card: the same, or a
/// held card named in one and not in the other.
fn same_card(a: Cell, b: Cell) -> bool {
    a == b
        || matches!(
            (a, b),
            (Cell::Held(None), Cell::Held(_)) | (Cell::Held(_), Cell::Held(None))
        )
}

/// How many places `before` starts ahead of `now` (a place of `now` is that
/// many further on in `before`): told by the students both name, or else by
/// the cards' faces. `None` where the two run different ways, neither students
/// nor faces tell it, or they tell it differently.
fn ahead(before: &Seen, now: &Seen) -> Option<i64> {
    if before.sight.ascending != now.sight.ascending {
        return None;
    }
    let by_faces = ahead_by_faces(before, now);
    match ahead_by_students(&before.sight, &now.sight) {
        Told::Nothing => by_faces,
        Told::Odd => None,
        Told::Ahead(by) => by_faces.is_none_or(|faces| faces == by).then_some(by),
    }
}

/// How the students two frames name lay them over one another.
enum Told {
    /// No student in common.
    Nothing,
    /// The students in common stand differently in the two.
    Odd,
    Ahead(i64),
}

fn ahead_by_students(before: &Sight, now: &Sight) -> Told {
    let places = |sight: &Sight| -> BTreeMap<i64, i64> {
        let named = sight
            .cells
            .iter()
            .enumerate()
            .filter_map(|(place, cell)| match cell {
                Some(Cell::Held(Some(id))) => Some((*id, place as i64)),
                _ => None,
            });
        named.collect()
    };
    let (before, now) = (places(before), places(now));
    let mut ahead = now
        .iter()
        .filter_map(|(id, place)| Some(before.get(id)? - place));
    match ahead.next() {
        None => Told::Nothing,
        Some(by) if ahead.all(|other| other == by) => Told::Ahead(by),
        Some(_) => Told::Odd,
    }
}

/// The list scrolls by whole rows of `list::COLUMNS` cards as the faces see
/// it, this many rows either way at most between two frames.
const ROWS_APART: i64 = 4;
/// Faces are compared over this many bands at least, and are the same cards'
/// where a band's colour is off by `ALIKE` at most on the mean, while every
/// other way to lay the frames over one another leaves them off by `UNLIKE`
/// or more.
const BANDS_LEAST: u32 = 24;
const ALIKE: f32 = 6.0;
const UNLIKE: f32 = 14.0;

/// How many places `before` starts ahead of `now` as the cards' faces tell
/// it: the number of whole rows at which the faces of the two are alike, and
/// at no other.
fn ahead_by_faces(before: &Seen, now: &Seen) -> Option<i64> {
    let columns = super::list::COLUMNS as i64;
    let off = |rows: i64| -> Option<f32> {
        let (mut sum, mut bands) = (0, 0);
        for (place, face) in now.faces.iter().enumerate() {
            let there = usize::try_from(place as i64 + rows * columns).ok();
            if let Some(other) = there.and_then(|at| before.faces.get(at)) {
                let (off, compared) = apart(face, other);
                sum += off;
                bands += compared;
            }
        }
        (bands >= BANDS_LEAST).then(|| sum as f32 / bands as f32)
    };
    let all: Vec<(i64, f32)> = (-ROWS_APART..=ROWS_APART)
        .filter_map(|rows| Some((rows, off(rows)?)))
        .collect();
    let &(rows, least) = all.iter().min_by(|a, b| a.1.total_cmp(&b.1))?;
    let alone = all
        .iter()
        .all(|&(other, off)| other == rows || off >= UNLIKE);
    (least <= ALIKE && alone).then_some(rows * columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::student::list::{BANDS, COLUMNS, Face};

    const A: i64 = 10001;
    const B: i64 = 10002;
    const C: i64 = 10003;
    const D: i64 = 10004;
    const E: i64 = 10005;

    #[allow(clippy::unnecessary_wraps)] // A place of a frame, as the constants below.
    fn held(id: i64) -> Option<Cell> {
        Some(Cell::Held(Some(id)))
    }

    const UNNAMED: Option<Cell> = Some(Cell::Held(None));
    const LACKED: Option<Cell> = Some(Cell::Lacked);
    const EMPTY: Option<Cell> = Some(Cell::Empty);

    /// A frame of the list with the newest first.
    fn newest_first(cells: &[Option<Cell>]) -> Sight {
        Sight {
            ascending: false,
            cells: cells.to_vec(),
            first: None,
        }
    }

    /// A frame of the list with the oldest first, its start in view or not.
    fn oldest_first(cells: &[Option<Cell>], first: Option<usize>) -> Sight {
        Sight {
            ascending: true,
            cells: cells.to_vec(),
            first,
        }
    }

    fn numbers(accepted: &Accepted) -> Vec<(i64, u32)> {
        accepted.known().iter().map(|(&id, &n)| (id, n)).collect()
    }

    /// The held card before the first grey or missing one is the student
    /// accepted first; those before it count up.
    #[test]
    fn the_last_held_card_starts_the_numbers() {
        let mut accepted = Accepted::default();
        let frame = newest_first(&[held(C), held(B), held(A), LACKED, LACKED, EMPTY]);
        assert_eq!(accepted.observe(&frame), Took::Numbered(3));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3)]);
        // The same before a missing card, with no grey ones.
        let mut accepted = Accepted::default();
        accepted.observe(&newest_first(&[held(B), held(A), EMPTY]));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2)]);
    }

    /// A frame without the list's end counts from a student already numbered.
    #[test]
    fn a_frame_counts_from_a_student_known() {
        let mut accepted = Accepted::seeded(BTreeMap::from([(A, 1), (B, 2), (C, 3)]));
        let frame = newest_first(&[held(E), held(D), held(C), held(B)]);
        assert_eq!(accepted.observe(&frame), Took::Numbered(2));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3), (D, 4), (E, 5)]);
        // All known already: nothing new.
        assert_eq!(accepted.observe(&frame), Took::Numbered(0));
    }

    /// Scrolling down from the top, no student has a number until the list's
    /// end comes into view: then every one seen on the way gets one.
    #[test]
    fn students_seen_on_the_way_down_are_numbered_at_the_end() {
        let mut accepted = Accepted::default();
        assert_eq!(
            accepted.observe(&newest_first(&[held(E), held(D)])),
            Took::Loose
        );
        assert_eq!(
            accepted.observe(&newest_first(&[held(D), held(C)])),
            Took::Loose
        );
        assert!(accepted.known().is_empty());
        let end = newest_first(&[held(C), held(B), held(A), LACKED]);
        assert_eq!(accepted.observe(&end), Took::Numbered(5));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3), (D, 4), (E, 5)]);
    }

    /// Scrolling up from the end, each frame counts on from the one before.
    #[test]
    fn students_seen_on_the_way_up_are_numbered_as_they_come() {
        let mut accepted = Accepted::default();
        accepted.observe(&newest_first(&[held(C), held(B), held(A), LACKED]));
        assert_eq!(
            accepted.observe(&newest_first(&[held(D), held(C)])),
            Took::Numbered(1)
        );
        assert_eq!(
            accepted.observe(&newest_first(&[held(E), held(D)])),
            Took::Numbered(1)
        );
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3), (D, 4), (E, 5)]);
    }

    /// Stretches seen apart are joined by a frame that shows students of both.
    #[test]
    fn stretches_seen_apart_join_where_a_frame_shows_both() {
        let mut accepted = Accepted::default();
        accepted.observe(&newest_first(&[held(E), held(D)]));
        accepted.observe(&newest_first(&[held(B), held(A), LACKED]));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2)]);
        // D and E hang loose until C ties them to B.
        assert_eq!(
            accepted.observe(&newest_first(&[held(D), held(C), held(B)])),
            Took::Numbered(3)
        );
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3), (D, 4), (E, 5)]);
    }

    /// A frame at odds with the numbers known gives nothing, and changes none.
    #[test]
    fn a_frame_at_odds_with_what_is_known_is_not_taken() {
        let known = BTreeMap::from([(A, 1), (B, 2), (C, 3)]);
        let mut accepted = Accepted::seeded(known.clone());
        // C right before A: a place short of what is known.
        let frame = newest_first(&[held(D), held(C), held(A)]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        // B as the student accepted first.
        let frame = newest_first(&[held(D), held(B), LACKED]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        // A held card where the numbers would come to zero.
        let frame = newest_first(&[held(B), held(A), held(D)]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        // A grey card among the students known to be held.
        let frame = newest_first(&[held(C), LACKED, held(D)]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        let frame = newest_first(&[held(D), held(C), LACKED]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        assert_eq!(accepted.known(), &known);
    }

    /// Two students never share a number.
    #[test]
    fn a_number_taken_is_given_to_no_other_student() {
        let mut accepted = Accepted::seeded(BTreeMap::from([(A, 1)]));
        let frame = newest_first(&[held(C), held(B), LACKED]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        assert_eq!(numbers(&accepted), [(A, 1)]);
    }

    /// The same student twice in a frame is a name told wrong.
    #[test]
    fn a_student_twice_in_a_frame_is_not_taken() {
        let mut accepted = Accepted::default();
        let frame = newest_first(&[held(B), held(A), held(A), LACKED]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        assert!(accepted.known().is_empty());
    }

    /// A held card whose name is not told is a place like any other, with no
    /// number kept for it; so is a place the frame tells nothing of.
    #[test]
    fn a_card_not_named_is_still_a_place() {
        let mut accepted = Accepted::default();
        accepted.observe(&newest_first(&[held(D), None, UNNAMED, held(A), LACKED]));
        assert_eq!(numbers(&accepted), [(A, 1), (D, 4)]);
    }

    /// The numbers start only where the place after the last held card is told.
    #[test]
    fn the_end_not_in_view_starts_no_numbers() {
        let mut accepted = Accepted::default();
        assert_eq!(
            accepted.observe(&newest_first(&[held(B), held(A), None, LACKED])),
            Took::Loose
        );
        assert_eq!(
            accepted.observe(&newest_first(&[held(B), held(A)])),
            Took::Loose
        );
        assert!(accepted.known().is_empty());
    }

    /// A student accepted later stands before the others and takes the next
    /// number; theirs stay.
    #[test]
    fn a_new_student_takes_the_next_number() {
        let mut accepted = Accepted::seeded(BTreeMap::from([(A, 1), (B, 2)]));
        accepted.observe(&newest_first(&[held(C), held(B), held(A), LACKED]));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3)]);
    }

    /// Grey and missing cards get no number, and none stands before a held one.
    #[test]
    fn cards_not_held_get_no_number() {
        let mut accepted = Accepted::default();
        accepted.observe(&newest_first(&[held(A), LACKED, LACKED, EMPTY, EMPTY]));
        assert_eq!(numbers(&accepted), [(A, 1)]);
        // The grey cards start a row of their own: the held cards' last row
        // ends with missing ones.
        let mut accepted = Accepted::default();
        accepted.observe(&newest_first(&[
            held(B),
            held(A),
            EMPTY,
            EMPTY,
            LACKED,
            LACKED,
            EMPTY,
        ]));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2)]);
        let mut accepted = Accepted::default();
        let frame = newest_first(&[held(B), LACKED, held(A), EMPTY]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        let frame = newest_first(&[held(B), EMPTY, held(A), LACKED]);
        assert!(matches!(accepted.observe(&frame), Took::Refused(_)));
        assert!(accepted.known().is_empty());
    }

    /// With the oldest first, the list's first card is the student accepted
    /// first and the numbers count up along the list; the held card before the
    /// grey ones is the newest and starts nothing.
    #[test]
    fn the_oldest_first_counts_up_from_the_lists_start() {
        let mut accepted = Accepted::default();
        let top = oldest_first(&[held(A), held(B), held(C)], Some(0));
        assert_eq!(accepted.observe(&top), Took::Numbered(3));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3)]);
        // Further down, counted from a student known.
        accepted.observe(&oldest_first(&[held(C), held(D), held(E), LACKED], None));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (C, 3), (D, 4), (E, 5)]);
        // The list's end alone tells no number.
        let mut accepted = Accepted::default();
        assert_eq!(
            accepted.observe(&oldest_first(&[held(D), held(E), LACKED], None)),
            Took::Loose
        );
        assert!(accepted.known().is_empty());
        // The start in view below a row cut off: the first place is the second told.
        let mut accepted = Accepted::default();
        accepted.observe(&oldest_first(&[None, held(A), held(B)], Some(1)));
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2)]);
    }

    /// Both ways give a student the same number.
    #[test]
    fn either_direction_gives_the_same_numbers() {
        let mut down = Accepted::default();
        down.observe(&newest_first(&[held(C), held(B), held(A), EMPTY]));
        let mut up = Accepted::default();
        up.observe(&oldest_first(&[held(A), held(B), held(C), EMPTY], Some(0)));
        assert_eq!(down.known(), up.known());
        // A grey card where a student known stands, the oldest first.
        let frame = oldest_first(&[held(A), LACKED], Some(0));
        assert!(matches!(up.observe(&frame), Took::Refused(_)));
    }

    /// Students hanging loose that turn out at odds with the numbers known are
    /// dropped; the frame that tied them still counts.
    #[test]
    fn loose_students_at_odds_with_the_numbers_are_dropped() {
        let mut accepted = Accepted::seeded(BTreeMap::from([(A, 1), (B, 2)]));
        // C two places after D, neither with a number.
        assert_eq!(
            accepted.observe(&newest_first(&[held(D), None, held(C)])),
            Took::Loose
        );
        // D is 3: C would be 1, which is A's.
        assert_eq!(
            accepted.observe(&newest_first(&[held(D), held(B), held(A)])),
            Took::Numbered(1)
        );
        assert_eq!(numbers(&accepted), [(A, 1), (B, 2), (D, 3)]);
    }

    /// A frame with no faces told.
    fn seen(sight: &Sight) -> Seen {
        Seen {
            faces: vec![[None; BANDS]; sight.cells.len()],
            sight: sight.clone(),
        }
    }

    /// What `steady` passes on of `sight`, shown `times` in a row.
    fn shown(steady: &mut Steady, sight: &Sight, times: u8) -> Sight {
        let mut passed = None;
        for _ in 0..times {
            passed = Some(steady.observe(&seen(sight)));
        }
        passed.expect("shown once at least")
    }

    /// Only what `CONFIRM` frames in a row show the same is passed on, the
    /// frames laid over one another by the students they share; what was
    /// passed on stays in the stretch as the list scrolls on.
    #[test]
    fn a_place_is_passed_on_once_frames_in_a_row_agree() {
        let mut steady = Steady::default();
        let frame = newest_first(&[held(C), held(B), held(A), LACKED]);
        assert_eq!(shown(&mut steady, &frame, CONFIRM - 1), newest_first(&[]));
        assert_eq!(shown(&mut steady, &frame, 1), frame);
        // Scrolled a place up: D is new, the others were seen before.
        let scrolled = newest_first(&[held(D), held(C), held(B), held(A)]);
        assert_eq!(shown(&mut steady, &scrolled, 1), frame);
        let all = newest_first(&[held(D), held(C), held(B), held(A), LACKED]);
        assert_eq!(shown(&mut steady, &scrolled, CONFIRM - 1), all);
    }

    /// A name read otherwise in one frame changes nothing; read otherwise in
    /// frame after frame, what was passed on before is not trusted either.
    #[test]
    fn a_place_read_otherwise_for_good_starts_over() {
        let mut steady = Steady::default();
        let frame = newest_first(&[held(C), held(B), held(A)]);
        shown(&mut steady, &frame, CONFIRM);
        let other = newest_first(&[held(C), held(D), held(A)]);
        assert_eq!(shown(&mut steady, &other, 1), frame);
        assert_eq!(shown(&mut steady, &frame, 1), frame);
        assert_eq!(shown(&mut steady, &other, CONFIRM), newest_first(&[]));
        assert_eq!(shown(&mut steady, &other, CONFIRM), other);
    }

    /// A card named in one frame and not in the next is the same card.
    #[test]
    fn a_card_not_named_for_a_frame_keeps_its_name() {
        let mut steady = Steady::default();
        let frame = newest_first(&[held(B), held(A)]);
        shown(&mut steady, &frame, CONFIRM);
        let unnamed = newest_first(&[held(B), UNNAMED]);
        assert_eq!(shown(&mut steady, &unnamed, CONFIRM), frame);
    }

    /// Frames sharing no student, or laid over one another in two ways, start
    /// over; so does the list turned the other way.
    #[test]
    fn frames_that_do_not_lie_over_one_another_start_over() {
        let mut steady = Steady::default();
        let frame = newest_first(&[held(C), held(B), held(A)]);
        shown(&mut steady, &frame, CONFIRM);
        // No student in common.
        let apart = newest_first(&[held(E), held(D)]);
        assert_eq!(shown(&mut steady, &apart, CONFIRM - 1), newest_first(&[]));
        assert_eq!(shown(&mut steady, &apart, 1), apart);
        // E moved a place against D.
        let odd = newest_first(&[held(E), None, held(D)]);
        assert_eq!(shown(&mut steady, &odd, 1), newest_first(&[]));
        // Turned the other way.
        shown(&mut steady, &apart, CONFIRM);
        let turned = oldest_first(&[held(D), held(E)], None);
        assert_eq!(shown(&mut steady, &turned, 1), oldest_first(&[], None));
    }

    /// The list's start is passed on as the places are, and stays where it is
    /// in the stretch as the list scrolls on.
    #[test]
    fn the_lists_start_is_passed_on_once_frames_agree() {
        let mut steady = Steady::default();
        let frame = oldest_first(&[held(A), held(B)], Some(0));
        assert_eq!(shown(&mut steady, &frame, CONFIRM - 1).first, None);
        assert_eq!(shown(&mut steady, &frame, 1), frame);
        let scrolled = oldest_first(&[held(B), held(C)], None);
        let all = oldest_first(&[held(A), held(B), held(C)], Some(0));
        assert_eq!(shown(&mut steady, &scrolled, CONFIRM), all);
    }

    /// A row of cards, each with a face of its own (`shade` and the five after).
    fn row(
        cells: [Option<Cell>; COLUMNS],
        shade: u8,
    ) -> ([Option<Cell>; COLUMNS], [Face; COLUMNS]) {
        (
            cells,
            std::array::from_fn(|column| [Some([shade + column as u8 * 20; 3]); BANDS]),
        )
    }

    fn rows(rows: &[([Option<Cell>; COLUMNS], [Face; COLUMNS])]) -> Seen {
        Seen {
            sight: newest_first(&rows.iter().flat_map(|row| row.0).collect::<Vec<_>>()),
            faces: rows.iter().flat_map(|row| row.1).collect(),
        }
    }

    const NAMED: [Option<Cell>; COLUMNS] = [
        Some(Cell::Held(Some(1))),
        Some(Cell::Held(Some(2))),
        Some(Cell::Held(Some(3))),
        Some(Cell::Held(Some(4))),
        Some(Cell::Held(Some(5))),
        Some(Cell::Held(Some(6))),
    ];
    const OTHERS: [Option<Cell>; COLUMNS] = [
        Some(Cell::Held(Some(11))),
        Some(Cell::Held(Some(12))),
        Some(Cell::Held(Some(13))),
        Some(Cell::Held(Some(14))),
        Some(Cell::Held(Some(15))),
        Some(Cell::Held(Some(16))),
    ];

    /// Frames that name no student in common are laid over one another by the
    /// cards' faces: the row whose names were cut off below is the row named
    /// after the list scrolled on.
    #[test]
    fn frames_lie_over_one_another_by_the_cards_faces() {
        let mut steady = Steady::default();
        let before = rows(&[row(NAMED, 0), row([UNNAMED; COLUMNS], 5)]);
        let after = rows(&[row(OTHERS, 7), row([UNNAMED; COLUMNS], 100)]);
        for _ in 0..CONFIRM {
            steady.observe(&before);
        }
        let mut passed = None;
        for _ in 0..CONFIRM {
            passed = Some(steady.observe(&after));
        }
        let whole: Vec<_> = [NAMED, OTHERS, [UNNAMED; COLUMNS]]
            .into_iter()
            .flatten()
            .collect();
        assert_eq!(passed, Some(newest_first(&whole)));
    }

    /// Faces that are no more alike a row on than where they stood, or alike
    /// nowhere, lay nothing over: the stretch starts over.
    #[test]
    fn faces_alike_in_two_ways_or_none_start_over() {
        let before = rows(&[row(NAMED, 0), row([UNNAMED; COLUMNS], 5)]);
        // No card of the frame before.
        let mut steady = Steady::default();
        for _ in 0..CONFIRM {
            steady.observe(&before);
        }
        let apart = rows(&[row(OTHERS, 130), row([UNNAMED; COLUMNS], 135)]);
        assert_eq!(steady.observe(&apart), newest_first(&[]));
        // Every card alike: any row may be any.
        let mut steady = Steady::default();
        let same = |cells| (cells, [[Some([90; 3]); BANDS]; COLUMNS]);
        let before = rows(&[same(NAMED), same([UNNAMED; COLUMNS])]);
        for _ in 0..CONFIRM {
            steady.observe(&before);
        }
        let after = rows(&[same(OTHERS), same([UNNAMED; COLUMNS])]);
        assert_eq!(steady.observe(&after), newest_first(&[]));
    }

    /// Students and faces telling different ways to lay two frames over one
    /// another start over.
    #[test]
    fn faces_at_odds_with_the_students_start_over() {
        let mut steady = Steady::default();
        let before = rows(&[row(NAMED, 0), row(OTHERS, 5)]);
        for _ in 0..CONFIRM {
            steady.observe(&before);
        }
        // The students stand as before, the faces a row on.
        let after = rows(&[row(NAMED, 5), row(OTHERS, 100)]);
        assert_eq!(steady.observe(&after), newest_first(&[]));
    }

    /// The numbers take a frame's steady part; leaving the list, or a stretch
    /// at odds with the numbers, starts the stretch over.
    #[test]
    fn the_counting_starts_its_stretch_over_when_refused_or_away() {
        let mut counting = Counting::default();
        let end = seen(&newest_first(&[held(B), held(A), LACKED]));
        for _ in 1..CONFIRM {
            assert_eq!(counting.frame(&end), Took::Nothing);
        }
        assert_eq!(counting.frame(&end), Took::Numbered(2));
        // Away and back: the frames before no longer count.
        counting.away();
        assert_eq!(counting.frame(&end), Took::Nothing);
        // A stretch at odds with the numbers is refused once, then forgotten.
        let odd = seen(&newest_first(&[held(A), held(B), LACKED]));
        let mut counting = Counting::seeded(BTreeMap::from([(A, 1), (B, 2)]));
        for _ in 1..CONFIRM {
            counting.frame(&odd);
        }
        assert!(matches!(counting.frame(&odd), Took::Refused(_)));
        assert_eq!(counting.frame(&odd), Took::Nothing);
        assert_eq!(numbers_of(&counting), [(A, 1), (B, 2)]);
    }

    fn numbers_of(counting: &Counting) -> Vec<(i64, u32)> {
        counting.known().iter().map(|(&id, &n)| (id, n)).collect()
    }
}
