pub(crate) type Snapshot = u32;

pub(crate) fn snapshot(rating: f64) -> Option<Snapshot> {
    bounded(rating.round())
}

pub(crate) fn rounded(rating: f64) -> Option<i32> {
    snapshot(rating).map(native)
}

pub(crate) fn floored(rating: f64) -> Option<i32> {
    bounded(rating.floor()).map(native)
}

pub(crate) fn native(snapshot: Snapshot) -> i32 {
    i32::try_from(snapshot).expect("normalized rating snapshot fits the native rating domain")
}

fn bounded(rating: f64) -> Option<Snapshot> {
    if !rating.is_finite() {
        return None;
    }
    let rating = rating.max(0.0);
    if rating <= f64::from(i32::MAX) {
        Some(rating as Snapshot)
    } else {
        None
    }
}
