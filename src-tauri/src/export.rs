//! What Aoi read, written as YAML for the user to hand to an agent or another
//! tool: the students' training and the counts of the item and equipment lists.

use crate::{Row, keep::ReadAt, watch::StudentRow};

/// The YAML of everything read, headed by when it was written (`stamp`) and
/// when each list was last read.
pub fn yaml(
    stamp: &str,
    read_at: &ReadAt,
    students: &[StudentRow],
    items: &[Row],
    equipment: &[Row],
) -> String {
    let when = |at: &Option<String>| at.as_deref().unwrap_or("なし").to_owned();
    let mut out = format!(
        "# Aoi の読み取り結果 {stamp}\n# 最終読み取り: アイテム {} / 装備 {} / 生徒 {}\n",
        when(&read_at.items),
        when(&read_at.equipment),
        when(&read_at.students)
    );
    out += "# null は読めなかった欄。weapon: locked は固有武器が未解放、gear: 0 は愛用品なし。\n";
    out += "# skills は EX・ノーマル・パッシブ・サブ、ability は HP・攻撃力・治癒力の能力解放。\n";
    out += "# count の K は千単位（51K は 50,500〜51,499）。\n";
    out += "# id: null の生徒は SchaleDB に未登録（名前だけで照合）。\n";
    out += &list("students", students, |s| {
        let t = &s.training;
        let weapon = match t.weapon {
            None => "null".to_owned(),
            Some(None) => "locked".to_owned(),
            Some(Some(w)) => format!("{{stars: {}, level: {}}}", w.stars, w.level),
        };
        let equipment = t.equipment.map(|e| {
            e.map_or("null".to_owned(), |e| {
                format!("{{tier: {}, level: {}}}", e.tier, e.level)
            })
        });
        // A student SchaleDB does not list yet has no id but Aoi's own.
        let id = if s.id < 0 {
            "null".to_owned()
        } else {
            s.id.to_string()
        };
        vec![
            format!("id: {id}"),
            format!("name: {}", quoted(&s.name)),
            format!("level: {}", value(t.level)),
            format!("rarity: {}", value(t.rarity)),
            format!("bond: {}", value(t.bond)),
            format!("skills: [{}]", t.skills.map(value).join(", ")),
            format!("weapon: {weapon}"),
            format!("equipment: [{}]", equipment.join(", ")),
            format!("gear: {}", value(t.gear)),
            format!("ability: [{}]", t.ability.map(value).join(", ")),
        ]
    });
    out += &list("items", items, counted);
    out += &list("equipment", equipment, counted);
    out
}

/// A list under `key`, one mapping per entry with the fields `fields` gives.
fn list<T>(key: &str, entries: &[T], fields: impl Fn(&T) -> Vec<String>) -> String {
    if entries.is_empty() {
        return format!("{key}: []\n");
    }
    let mut out = format!("{key}:\n");
    for entry in entries {
        for (i, field) in fields(entry).iter().enumerate() {
            out += if i == 0 { "  - " } else { "    " };
            out += field;
            out += "\n";
        }
    }
    out
}

/// An entry of the item or the equipment list.
fn counted(row: &Row) -> Vec<String> {
    let mut fields = vec![
        format!("id: {}", row.id),
        format!("name: {}", quoted(&row.name)),
    ];
    if let Some(tier) = row.tier {
        fields.push(format!("tier: {tier}"));
    }
    fields.push(format!("count: {}", row.count));
    fields
}

/// A JSON string, which YAML reads as a double-quoted string whatever it holds.
fn quoted(text: &str) -> String {
    serde_json::Value::from(text).to_string()
}

fn value(v: Option<u8>) -> String {
    v.map_or("null".to_owned(), |v| v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        keep::ReadAt,
        quantity::Count,
        student::{Gear, Training, Weapon},
    };

    fn mika() -> StudentRow {
        StudentRow {
            id: 10059,
            name: "ミカ".to_owned(),
            training: Training {
                level: Some(90),
                rarity: Some(5),
                bond: Some(43),
                skills: [Some(5), Some(10), Some(10), Some(10)],
                weapon: Some(Some(Weapon {
                    level: 50,
                    stars: 3,
                })),
                equipment: [Some(Gear {
                    tier: 10,
                    level: 70,
                }); 3],
                gear: Some(0),
                ability: [Some(25), Some(25), Some(0)],
            },
        }
    }

    fn row(id: i64, name: &str, tier: Option<u8>, count: Count) -> Row {
        Row {
            id,
            name: name.to_owned(),
            tier,
            count,
        }
    }

    #[test]
    fn writes_every_list_with_a_heading() {
        let items = [row(1, "初級強化珠", None, Count::Exact(1383))];
        let equipment = [
            row(1000, "無地のキャップ", Some(1), Count::Exact(249)),
            row(501_000, "帽子の万能設計図", None, Count::Thousands(51)),
        ];
        let text = yaml(
            "2026-10-12 15:30:00",
            &ReadAt::default(),
            &[mika()],
            &items,
            &equipment,
        );
        assert!(text.starts_with("# Aoi の読み取り結果 2026-10-12 15:30:00\n"));
        let body: String = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .flat_map(|l| [l, "\n"])
            .collect();
        assert_eq!(
            body,
            r#"students:
  - id: 10059
    name: "ミカ"
    level: 90
    rarity: 5
    bond: 43
    skills: [5, 10, 10, 10]
    weapon: {stars: 3, level: 50}
    equipment: [{tier: 10, level: 70}, {tier: 10, level: 70}, {tier: 10, level: 70}]
    gear: 0
    ability: [25, 25, 0]
items:
  - id: 1
    name: "初級強化珠"
    count: 1383
equipment:
  - id: 1000
    name: "無地のキャップ"
    tier: 1
    count: 249
  - id: 501000
    name: "帽子の万能設計図"
    count: 51K
"#
        );
    }

    #[test]
    fn tells_unread_fields_from_what_the_student_lacks() {
        let mut student = mika();
        student.training = Training {
            level: None,
            bond: None,
            skills: [Some(2), None, Some(3), Some(6)],
            weapon: Some(None),
            equipment: [Some(Gear { tier: 4, level: 40 }), None, None],
            gear: None,
            ability: [Some(0), None, Some(0)],
            ..student.training
        };
        let text = yaml("", &ReadAt::default(), &[student.clone()], &[], &[]);
        for line in [
            "    level: null",
            "    bond: null",
            "    skills: [2, null, 3, 6]",
            "    weapon: locked",
            "    equipment: [{tier: 4, level: 40}, null, null]",
            "    gear: null",
            "    ability: [0, null, 0]",
        ] {
            assert!(text.lines().any(|l| l == line), "{line}\n{text}");
        }
        student.training.weapon = None;
        let text = yaml("", &ReadAt::default(), &[student], &[], &[]);
        assert!(text.lines().any(|l| l == "    weapon: null"), "{text}");
    }

    #[test]
    fn writes_no_id_for_a_student_schaledb_lacks() {
        let mut student = mika();
        student.id = -1;
        student.name = "ツムギ".to_owned();
        let text = yaml("", &ReadAt::default(), &[student], &[], &[]);
        assert!(text.lines().any(|l| l == "  - id: null"), "{text}");
        assert!(text.lines().any(|l| l == r#"    name: "ツムギ""#), "{text}");
    }

    #[test]
    fn tells_when_each_list_was_read() {
        let read_at = ReadAt {
            items: Some("2026-10-12 15:30:00".to_owned()),
            equipment: None,
            students: Some("2026-10-12 16:00:00".to_owned()),
        };
        let text = yaml("2026-10-12 16:05:00", &read_at, &[], &[], &[]);
        let line =
            "# 最終読み取り: アイテム 2026-10-12 15:30:00 / 装備 なし / 生徒 2026-10-12 16:00:00";
        assert!(text.lines().any(|l| l == line), "{text}");
    }

    #[test]
    fn writes_empty_lists_as_such() {
        let text = yaml("", &ReadAt::default(), &[], &[], &[]);
        for line in ["students: []", "items: []", "equipment: []"] {
            assert!(text.lines().any(|l| l == line), "{line}\n{text}");
        }
    }

    #[test]
    fn quotes_names_the_yaml_would_misread() {
        let items = [row(7, "Lv: \"MAX\" #1", None, Count::Exact(1))];
        let text = yaml("", &ReadAt::default(), &[], &items, &[]);
        assert!(
            text.lines().any(|l| l == r#"    name: "Lv: \"MAX\" #1""#),
            "{text}"
        );
    }
}
