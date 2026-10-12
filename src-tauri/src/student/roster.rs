//! Keeps the training of every student seen while the user steps through them.
//!
//! A reading is taken once the same student reads the same in `CONFIRM` frames
//! in a row, so a frame caught while the screen changes never counts. A field
//! read as unknown keeps what was known of it.

use std::collections::BTreeMap;

use super::Training;
use crate::tally::CONFIRM;

#[derive(Default)]
pub struct Roster {
    students: BTreeMap<i64, Training>,
    /// The latest reading and in how many frames in a row it was read.
    streak: Option<(i64, Training, u8)>,
}

impl Training {
    /// This training with the fields `newer` knows replaced.
    fn merged(&self, newer: &Training) -> Training {
        let pick = |a: Option<u8>, b: Option<u8>| b.or(a);
        Training {
            level: pick(self.level, newer.level),
            rarity: pick(self.rarity, newer.rarity),
            bond: pick(self.bond, newer.bond),
            skills: std::array::from_fn(|i| pick(self.skills[i], newer.skills[i])),
            weapon: newer.weapon.or(self.weapon),
            equipment: std::array::from_fn(|i| newer.equipment[i].or(self.equipment[i])),
            gear: pick(self.gear, newer.gear),
            ability: std::array::from_fn(|i| pick(self.ability[i], newer.ability[i])),
        }
    }
}

impl Roster {
    /// A roster starting from the students kept from before.
    pub fn seeded(students: BTreeMap<i64, Training>) -> Self {
        Self {
            students,
            streak: None,
        }
    }

    /// Take one frame's reading of student `id`; returns whether what is known
    /// of the students changed.
    pub fn observe(&mut self, id: i64, training: Training) -> bool {
        let frames = match &mut self.streak {
            Some((seen, read, frames)) if *seen == id && *read == training => {
                *frames = frames.saturating_add(1);
                *frames
            }
            _ => {
                self.streak = Some((id, training, 1));
                1
            }
        };
        if frames != CONFIRM {
            return false;
        }
        let known = self.students.get(&id);
        let merged = known.map_or(training, |k| k.merged(&training));
        if known == Some(&merged) {
            return false;
        }
        self.students.insert(id, merged);
        true
    }

    /// What is known of each student seen, by id.
    pub fn students(&self) -> &BTreeMap<i64, Training> {
        &self.students
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn training(level: Option<u8>, bond: Option<u8>) -> Training {
        Training {
            level,
            rarity: Some(5),
            bond,
            skills: [Some(5), Some(10), Some(10), Some(10)],
            weapon: Some(None),
            equipment: [None; 3],
            gear: Some(0),
            ability: [Some(0); 3],
        }
    }

    fn frames(roster: &mut Roster, id: i64, t: &Training, n: u8) -> Vec<bool> {
        (0..n).map(|_| roster.observe(id, *t)).collect()
    }

    #[test]
    fn a_student_is_taken_after_agreeing_frames() {
        let mut roster = Roster::default();
        let t = training(Some(90), Some(34));
        assert_eq!(frames(&mut roster, 7, &t, CONFIRM), [false, false, true]);
        assert_eq!(roster.students().get(&7), Some(&t));
        assert_eq!(frames(&mut roster, 7, &t, 2), [false, false]);
    }

    #[test]
    fn a_frame_of_another_student_starts_over() {
        let mut roster = Roster::default();
        let t = training(Some(90), Some(34));
        frames(&mut roster, 7, &t, CONFIRM - 1);
        frames(&mut roster, 8, &t, 1);
        frames(&mut roster, 7, &t, CONFIRM - 1);
        assert!(roster.students().is_empty());
    }

    #[test]
    fn an_unknown_field_keeps_what_was_known() {
        let mut roster = Roster::default();
        frames(&mut roster, 7, &training(Some(90), Some(34)), CONFIRM);
        frames(&mut roster, 7, &training(None, Some(35)), CONFIRM);
        assert_eq!(
            roster.students().get(&7),
            Some(&training(Some(90), Some(35)))
        );
    }
}
