use shared_types::rounded_rating;

pub(crate) type Snapshot = u32;

pub(crate) fn snapshot(rating: f64) -> Option<Snapshot> {
    rounded_rating(rating)
}

pub(crate) fn rounded(rating: f64) -> Option<i32> {
    snapshot(rating).map(native)
}

pub(crate) fn native(snapshot: Snapshot) -> i32 {
    i32::try_from(snapshot).expect("normalized rating snapshot fits the native rating domain")
}
