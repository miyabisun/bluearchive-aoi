//! The credits the top bar shows ("243,797,319"), read by the OCR on the
//! screens that have the bar.

use image::RgbImage;

use crate::student::{Area, rect};

/// The credits, a number up to the bar's next field (window heights from the
/// right edge and the top; measured on 2026-10-12 at 2222x1250: the digits of
/// "243,797,319" span 0.777 to 0.621 from the right, 0.020 to 0.044 down).
pub const NUMBER: Area = Area {
    from_right: true,
    x: (0.779, 0.556),
    y: (0.012, 0.050),
};

/// The yellow card standing for credits, left of the number.
pub const ICON: Area = Area {
    from_right: true,
    x: (0.834, 0.797),
    y: (0.020, 0.047),
};

/// The card's yellow covers at least this share of its area.
const ICON_SHARE: f32 = 0.15;

/// Whether the bar shows the credits: their yellow card is there.
pub fn bar_shows(image: &RgbImage) -> bool {
    let Some((x0, y0, x1, y1)) = rect(image, ICON) else {
        return false;
    };
    let yellow = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let c = image.get_pixel(x, y).0;
            c[0] > 230 && c[1] > 180 && c[2] < 100
        })
        .count();
    yellow as f32 >= ICON_SHARE * ((x1 - x0) * (y1 - y0)) as f32
}

/// The credits the OCR's `text` spells, when it is a number written with
/// thousands separators as the bar writes it; anything else is no reading.
pub fn parse(text: &str) -> Option<u64> {
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let groups: Vec<&str> = text.split([',', '.']).collect();
    let (first, rest) = groups.split_first()?;
    let digits = |g: &str| g.chars().all(|c| c.is_ascii_digit());
    let well_formed = (1..=3).contains(&first.len())
        && digits(first)
        && rest.iter().all(|g| g.len() == 3 && digits(g));
    well_formed.then(|| groups.concat().parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_the_number_the_bar_writes() {
        assert_eq!(parse("243,797,319"), Some(243_797_319));
        assert_eq!(parse("244,213,276"), Some(244_213_276));
        assert_eq!(parse("1,234"), Some(1234));
        assert_eq!(parse("999"), Some(999));
    }

    #[test]
    fn takes_the_ocrs_spaces_and_periods_for_the_separators() {
        assert_eq!(parse("243, 797, 319"), Some(243_797_319));
        assert_eq!(parse("243.797.319"), Some(243_797_319));
    }

    #[test]
    fn reads_nothing_from_what_is_no_such_number() {
        for text in [
            "",
            "243,79,319",
            "2434,797",
            "243,797,31",
            "24a,797,319",
            ",797",
            "1,234/5",
        ] {
            assert_eq!(parse(text), None, "{text}");
        }
    }
}
