//! What the plan counts the students from: their training as read when the app started or the
//! plan was last reloaded (the window's 再読込), so a student trained meanwhile leaves the plan
//! as it is; and who was trained since, for the window to tell. What is held (items, equipment,
//! credits, the shops) is counted as read now.

use serde::Serialize;

use crate::{plan::star_of, student::Training, watch::StudentRow};

/// The students as they were read, and when the last of them was.
#[derive(Debug, Clone, Default)]
pub struct Ground {
    pub students: Vec<StudentRow>,
    /// Local time ("2026-11-03 14:37:12"), none before any was read.
    pub read_at: Option<String>,
}

/// The marked students read trained otherwise since the ground, by name, and when the ground's
/// students were read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Behind {
    pub since: Option<String>,
    pub students: Vec<String>,
}

impl Ground {
    /// The `marked` students the plan would count otherwise as read `now`; none where it counts
    /// them the same. A student the ground lacks is counted as read now, so is never behind.
    pub fn behind(&self, now: &[StudentRow], marked: &[i64]) -> Option<Behind> {
        fn find(students: &[StudentRow], id: i64) -> Option<&StudentRow> {
            students.iter().find(|s| s.id == id)
        }
        let students: Vec<String> = marked
            .iter()
            .filter_map(|&id| Some((find(&self.students, id)?, find(now, id)?)))
            .filter(|(then, now)| !alike(&then.training, &now.training))
            .map(|(_, now)| now.name.clone())
            .collect();
        (!students.is_empty()).then(|| Behind {
            since: self.read_at.clone(),
            students,
        })
    }
}

/// Whether the plan counts the trainings alike: the level, the star (see `star_of`), the
/// skills and the equipment.
fn alike(a: &Training, b: &Training) -> bool {
    (a.level, star_of(a), a.skills, a.equipment) == (b.level, star_of(b), b.skills, b.equipment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::student::{Gear, Weapon};

    fn lana(rarity: u8, weapon: Option<u8>, bond: u8) -> StudentRow {
        StudentRow {
            id: 26017,
            name: "ラナ".to_owned(),
            training: Training {
                level: Some(90),
                rarity: Some(rarity),
                bond: Some(bond),
                skills: [Some(5), Some(8), Some(7), Some(7)],
                weapon: Some(weapon.map(|stars| Weapon { level: 30, stars })),
                equipment: [Some(Gear { tier: 10 }); 3],
                gear: Some(0),
                ability: [Some(0); 3],
            },
        }
    }

    /// Lana at ★4.
    fn ground() -> Ground {
        Ground {
            students: vec![lana(4, None, 3)],
            read_at: Some("2026-11-03 14:37:12".to_owned()),
        }
    }

    #[test]
    fn students_read_again_alike_are_not_behind() {
        assert_eq!(ground().behind(&[lana(4, None, 3)], &[26017]), None);
    }

    #[test]
    fn a_marked_student_trained_since_is_named() {
        let now = [lana(5, Some(1), 3)];
        assert_eq!(
            ground().behind(&now, &[26017]),
            Some(Behind {
                since: Some("2026-11-03 14:37:12".to_owned()),
                students: vec!["ラナ".to_owned()],
            })
        );
        // Not marked, the plan does not count them.
        assert_eq!(ground().behind(&now, &[]), None);
    }

    #[test]
    fn what_the_plan_does_not_count_of_a_student_leaves_it_alone() {
        assert_eq!(ground().behind(&[lana(4, None, 4)], &[26017]), None);
    }

    #[test]
    fn a_student_the_ground_lacks_is_counted_as_read_now() {
        let first = Ground::default();
        assert_eq!(first.behind(&[lana(4, None, 3)], &[26017]), None);
    }
}
