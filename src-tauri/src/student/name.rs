//! Tells which student a screen shows from the name the OCR read.
//!
//! The OCR reads the name well, but around it picks up the bond heart (a ")" or
//! "第" before the name) and the rarity stars ("☆食を" after it), and confuses a
//! few look-alike characters. A name counts only when the text holds it with
//! nothing but such noise around it: a base name never stands for its variant
//! ("ホシノ" is not "ホシノ(臨戦)"), nor a shorter name for a longer one ("ハル" is
//! not "コハル").

/// Characters the OCR mixes up, each mapped to one of its group (the
/// half-voiced "パ" read as "バ" among them), and small kana to large ones;
/// applied to the read text and to every name alike.
const FOLD: &[(char, char)] = &[
    ('工', 'エ'),
    ('力', 'カ'),
    ('口', 'ロ'),
    ('二', 'ニ'),
    ('一', 'ー'),
    ('フ', 'ラ'),
    ('ァ', 'ア'),
    ('ィ', 'イ'),
    ('ゥ', 'ウ'),
    ('ェ', 'エ'),
    ('ォ', 'オ'),
    ('ャ', 'ヤ'),
    ('ュ', 'ユ'),
    ('ョ', 'ヨ'),
    ('ッ', 'ツ'),
    ('パ', 'バ'),
    ('ピ', 'ビ'),
    ('プ', 'ブ'),
    ('ペ', 'ベ'),
    ('ポ', 'ボ'),
    ('（', '('),
    ('）', ')'),
    ('＊', '*'),
];

/// The text without spaces, with look-alike characters folded together.
pub fn fold(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| {
            FOLD.iter()
                .find(|(from, _)| *from == c)
                .map_or(c, |&(_, to)| to)
        })
        .collect()
}

fn katakana(c: char) -> bool {
    ('\u{30A1}'..='\u{30FA}').contains(&c)
}

/// The most characters of noise the OCR puts before a name (the bond heart).
const MAX_PREFIX: usize = 2;

/// Whether `name` (folded) stands alone in `text` (folded): noise before it is
/// short and holds no katakana, and after it neither a "(" (a variant's
/// costume) nor any katakana follows.
fn stands_alone(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let (before, after) = (&text[..at], &text[at + name.len()..]);
        before.chars().count() <= MAX_PREFIX
            && !before.chars().any(katakana)
            && !after.starts_with('(')
            && !after.chars().any(katakana)
    })
}

/// The id of the one student whose name stands alone in the OCR's `text`.
/// SchaleDB lists a few students twice under one name (one id per form, as
/// "ホシノ（臨戦）"); such a name stands for its lowest id.
pub fn find(text: &str, students: &[(i64, String)]) -> Option<i64> {
    let text = fold(text);
    let found: Vec<&(i64, String)> = students
        .iter()
        .filter(|(_, name)| stands_alone(&text, &fold(name)))
        .collect();
    let name = &found.first()?.1;
    found
        .iter()
        .all(|(_, n)| n == name)
        .then(|| found.iter().map(|(id, _)| *id).min())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn students() -> Vec<(i64, String)> {
        [
            "ホシノ",
            "ホシノ（臨戦）",
            "ハル",
            "コハル",
            "ヒナ",
            "ヒナタ",
            "カヨコ",
            "トモエ",
            "シロコ＊テラー",
        ]
        .iter()
        .enumerate()
        .map(|(i, n)| (i as i64, (*n).to_owned()))
        .collect()
    }

    fn named(text: &str) -> Option<String> {
        let students = students();
        find(text, &students).map(|id| students[id as usize].1.clone())
    }

    #[test]
    fn a_name_is_found_among_the_noise_of_the_heart_and_stars() {
        assert_eq!(named(") ホ シ ノ"), Some("ホシノ".into()));
        assert_eq!(named("第ホシノ(臨戦)☆"), Some("ホシノ（臨戦）".into()));
        assert_eq!(named("STRIKER / ホ シ ノ ( 臨 戦 ) ☆"), None);
        assert_eq!(named(") カ ョ コ"), Some("カヨコ".into()));
        assert_eq!(named("ト モ 工"), Some("トモエ".into()));
        assert_eq!(named("シロコ*テフー"), Some("シロコ＊テラー".into()));
    }

    #[test]
    fn a_shorter_name_never_stands_for_a_longer_one() {
        let without = |gone: &str| -> Vec<(i64, String)> {
            students().into_iter().filter(|(_, n)| n != gone).collect()
        };
        assert_eq!(find("ホシノ(臨戦)☆", &without("ホシノ（臨戦）")), None);
        assert_eq!(find(") コ ハ ル", &without("コハル")), None);
        assert_eq!(find("ヒナタ", &without("ヒナタ")), None);
        assert_eq!(find("", &students()), None);
    }

    #[test]
    fn a_name_listed_twice_stands_for_its_lowest_id() {
        let students = vec![
            (30, "ホシノ（臨戦）".to_owned()),
            (12, "ホシノ（臨戦）".to_owned()),
        ];
        assert_eq!(find("ホシノ(臨戦)☆", &students), Some(12));
    }
}
