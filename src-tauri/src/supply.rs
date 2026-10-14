//! What the training plan falls short of, and what could make each line up:
//! the choice boxes, the shops' goods and, for elephs, the fragments. Which of
//! them to use is the player's to choose (the window pays them one by one).

use serde::{Deserialize, Serialize};

use crate::plan::{BlueprintRow, Held, Kind, NeedRow, RATE};

/// A choice box (`data/schaledb/boxes.json`): one box gives one of its
/// choices, items or (`equipment`) blueprints.
#[derive(Debug, Clone, Deserialize)]
pub struct ChoiceBox {
    pub id: i64,
    pub equipment: bool,
    pub choices: Vec<i64>,
}

/// The lines of `rows` held short of what they need, by how much (a line not
/// counted left out).
pub fn shorts(rows: &[NeedRow]) -> Vec<(Kind, i64, u64)> {
    rows.iter()
        .filter_map(|row| {
            let (Held::Exact(have) | Held::About(have)) = row.have?;
            (have < row.need).then(|| (row.kind, row.id, row.need - have))
        })
        .collect()
}

/// The boxes that give the item (or with `Kind::Equipment` the blueprint)
/// `id`, those of fewest choices first.
pub fn boxes_for(kind: Kind, id: i64, boxes: &[ChoiceBox]) -> Vec<&ChoiceBox> {
    let equipment = match kind {
        Kind::Item => false,
        Kind::Equipment => true,
        _ => return Vec::new(),
    };
    let mut found: Vec<&ChoiceBox> = boxes
        .iter()
        .filter(|b| b.equipment == equipment && b.choices.contains(&id))
        .collect();
    found.sort_by_key(|b| (b.choices.len(), b.id));
    found
}

/// A good a shop sells this month: the shop and its coin (by index), the
/// good, how many times more it sells, at what price, and what it makes up:
/// the plan's line (kind and id) and how much of it one purchase gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForSale {
    pub shop: usize,
    pub coin: usize,
    pub good: i64,
    pub left: u32,
    pub price: u32,
    pub line: (Kind, i64),
    pub gives: u64,
}

/// A line of the plan that falls short, for the window: what it is (as its
/// `NeedRow`), by how much, and what could make it up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProcureLine {
    pub kind: Kind,
    pub id: i64,
    pub name: String,
    pub icon: String,
    pub short: u64,
    pub sources: Vec<Source>,
}

/// What could make up a line, with how much of it is held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Source {
    /// A choice box: one opened gives one of the line, chosen among
    /// `choices` (boxes alike but for how many schools they take).
    Box {
        id: i64,
        name: String,
        icon: String,
        held: u64,
        choices: usize,
    },
    /// A shop's good: a purchase gives `gives` of the line for `price` of the
    /// shop's `coin` (held `None` where not read), `left` more this month.
    Shop {
        shop: String,
        coin: String,
        icon: String,
        held: Option<u64>,
        price: u32,
        gives: u64,
        left: u32,
    },
    /// A slot's universal blueprints, `held` those T2 to T9 leave: `rate` of
    /// them make a T10 blueprint.
    Universal {
        id: i64,
        name: String,
        icon: String,
        held: u64,
        rate: u64,
    },
    /// The カケラ(文字) shop: an eleph a purchase, for fragments from `rate`
    /// on (20 at each rate up to 4, then 5), `left` more.
    Fragments {
        name: String,
        icon: String,
        held: u64,
        rate: u32,
        left: u32,
    },
}

/// The lines a plan's blueprints fall short of: each slot's T10 ones, made up
/// from the choice boxes (each as `source` tells it) or from the universal
/// ones T2 to T9 leave; and the universal ones T2 to T9 take beyond those held.
pub fn blueprint_lines(
    rows: &[BlueprintRow],
    boxes: &[ChoiceBox],
    source: impl Fn(&ChoiceBox) -> Source,
) -> Vec<ProcureLine> {
    let mut lines = Vec::new();
    for row in rows {
        let universal = &row.universal;
        let (need, have) = (universal.need, universal.have.amount());
        let top = row.tiers.iter().find(|t| t.tier == 10);
        if let (Some(top), true) = (top, row.short > 0) {
            let boxes = boxes_for(Kind::Equipment, top.id, boxes).into_iter();
            let mut sources: Vec<Source> = boxes.map(&source).collect();
            sources.push(Source::Universal {
                id: universal.id,
                name: universal.name.clone(),
                icon: universal.icon.clone(),
                held: have.saturating_sub(need),
                rate: RATE[10],
            });
            lines.push(ProcureLine {
                kind: Kind::Equipment,
                id: top.id,
                name: format!("{}のT10設計図", row.name),
                icon: top.icon.clone(),
                short: row.short,
                sources,
            });
        }
        if need > have {
            let boxes = boxes_for(Kind::Equipment, universal.id, boxes).into_iter();
            lines.push(ProcureLine {
                kind: Kind::Equipment,
                id: universal.id,
                name: universal.name.clone(),
                icon: universal.icon.clone(),
                short: need - have,
                sources: boxes.map(&source).collect(),
            });
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{BlueprintRow, TierBlueprint};

    fn row(kind: Kind, id: i64, need: u64, have: Option<Held>) -> NeedRow {
        NeedRow {
            kind,
            id,
            name: String::new(),
            icon: String::new(),
            family: None,
            need,
            have,
        }
    }

    #[test]
    fn a_line_is_short_when_held_under_its_need() {
        let rows = [
            row(Kind::Item, 4033, 53, Some(Held::Exact(24))),
            row(Kind::Item, 4010, 10, Some(Held::Exact(10))),
            // Not counted: whether it falls short is not known.
            row(Kind::Credits, 0, 10, None),
            row(Kind::ReportExp, 0, 60_000, Some(Held::About(51_000))),
        ];
        assert_eq!(
            shorts(&rows),
            [(Kind::Item, 4033, 29), (Kind::ReportExp, 0, 9000)]
        );
    }

    #[test]
    fn a_slot_short_of_t10_blueprints_takes_them_from_a_box_or_the_universal_ones_left() {
        let tier = |tier, id, need, have| TierBlueprint {
            tier,
            id,
            name: format!("T{tier}"),
            icon: format!("icon_{id}"),
            need,
            have: Held::Exact(have),
        };
        // T2 to T9 take 10 universal ones of 1,000; T10 is 43 short.
        let row = |name: &str, universal_need, short| BlueprintRow {
            name: name.into(),
            icon: String::new(),
            tiers: vec![tier(2, 101_001, 15, 10), tier(10, 101_009, 60, 60 - short)],
            universal: TierBlueprint {
                name: format!("{name}の万能設計図"),
                ..tier(0, 501_000, universal_need, 1000)
            },
            short,
        };
        let boxes = [ChoiceBox {
            id: 150_048,
            equipment: true,
            choices: vec![101_009, 102_009],
        }];
        let source = |b: &ChoiceBox| Source::Box {
            id: b.id,
            name: String::new(),
            icon: String::new(),
            held: 3,
            choices: b.choices.len(),
        };
        let lines = blueprint_lines(&[row("帽子", 10, 43)], &boxes, source);
        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        assert_eq!(
            (line.kind, line.id, line.name.as_str(), line.short),
            (Kind::Equipment, 101_009, "帽子のT10設計図", 43)
        );
        assert_eq!(line.sources[0], source(&boxes[0]));
        assert_eq!(
            line.sources[1],
            Source::Universal {
                id: 501_000,
                name: "帽子の万能設計図".into(),
                icon: "icon_501000".into(),
                held: 990,
                rate: 50,
            }
        );
        // T2 to T9 taking more universal ones than held: those short too.
        let lines = blueprint_lines(&[row("帽子", 1200, 0)], &boxes, source);
        let short: Vec<(i64, u64)> = lines.iter().map(|l| (l.id, l.short)).collect();
        assert_eq!(short, [(501_000, 200)]);
        assert!(lines[0].sources.is_empty());
    }

    #[test]
    fn the_boxes_of_a_line_come_fewest_choices_first() {
        let choice = |id, equipment, choices: &[i64]| ChoiceBox {
            id,
            equipment,
            choices: choices.to_vec(),
        };
        let boxes = [
            choice(150_003, false, &[4003, 4013, 4033, 4043]),
            choice(150_015, false, &[4033, 4043]),
            choice(150_019, false, &[4003, 4013]),
            // The equipment list's ids are apart from the items'.
            choice(150_030, true, &[4033]),
        ];
        let ids = |kind, id| {
            boxes_for(kind, id, &boxes)
                .iter()
                .map(|b| b.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(Kind::Item, 4033), [150_015, 150_003]);
        assert_eq!(ids(Kind::Equipment, 4033), [150_030]);
        assert_eq!(ids(Kind::Credits, 4033), Vec::<i64>::new());
    }
}
