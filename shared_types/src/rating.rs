/// The displayed integer also serves as the bounded Arena rating snapshot.
pub fn rounded_rating(rating: f64) -> Option<u32> {
    if !rating.is_finite() {
        return None;
    }
    let rating = rating.round().max(0.0);
    (rating <= f64::from(i32::MAX)).then_some(rating as u32)
}

pub fn rounded_rating_change(change: f64) -> i32 {
    change.round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displayed_ratings_use_nearest_integer_with_bounded_snapshots() {
        for (raw, expected) in [
            (1879.49, Some(1879)),
            (1879.5, Some(1880)),
            (1879.99, Some(1880)),
            (-1.0, Some(0)),
            (f64::from(i32::MAX), Some(i32::MAX as u32)),
            (f64::from(i32::MAX) + 1.0, None),
            (f64::NAN, None),
            (f64::INFINITY, None),
        ] {
            assert_eq!(rounded_rating(raw), expected, "raw rating: {raw}");
        }
    }

    #[test]
    fn displayed_rating_changes_round_symmetrically() {
        for (raw, expected) in [
            (-1.5, -2),
            (-0.5, -1),
            (-0.49, 0),
            (0.49, 0),
            (0.5, 1),
            (1.5, 2),
        ] {
            assert_eq!(rounded_rating_change(raw), expected);
        }
    }
}
