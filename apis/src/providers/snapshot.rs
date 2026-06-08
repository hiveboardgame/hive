use std::{
    collections::{BTreeMap, HashMap, HashSet},
    hash::Hash,
};

pub(super) fn snapshot_keeps<T>(id: &T, snapshot_ids: &HashSet<T>, dirty_ids: &HashSet<T>) -> bool
where
    T: Eq + Hash,
{
    snapshot_ids.contains(id) || dirty_ids.contains(id)
}

pub(super) fn apply_snapshot_set<T>(
    current: &mut HashSet<T>,
    snapshot_ids: &HashSet<T>,
    dirty_ids: &HashSet<T>,
) where
    T: Clone + Eq + Hash,
{
    current.retain(|id| snapshot_keeps(id, snapshot_ids, dirty_ids));
    for id in snapshot_ids {
        if !dirty_ids.contains(id) {
            current.insert(id.clone());
        }
    }
}

pub(super) fn retain_snapshot_hash_map<K, V>(
    current: &mut HashMap<K, V>,
    snapshot_ids: &HashSet<K>,
    dirty_ids: &HashSet<K>,
) where
    K: Eq + Hash,
{
    current.retain(|id, _| snapshot_keeps(id, snapshot_ids, dirty_ids));
}

pub(super) fn apply_snapshot_hash_map<K, V, I, F>(
    current: &mut HashMap<K, V>,
    snapshot_ids: &HashSet<K>,
    dirty_ids: &HashSet<K>,
    values: I,
    key: F,
) where
    K: Eq + Hash,
    I: IntoIterator<Item = V>,
    F: Fn(&V) -> K,
{
    retain_snapshot_hash_map(current, snapshot_ids, dirty_ids);
    for value in values {
        let id = key(&value);
        if !dirty_ids.contains(&id) {
            current.insert(id, value);
        }
    }
}

pub(super) fn retain_snapshot_btree_map<K, V>(
    current: &mut BTreeMap<K, V>,
    snapshot_ids: &HashSet<K>,
    dirty_ids: &HashSet<K>,
) where
    K: Eq + Hash + Ord,
{
    current.retain(|id, _| snapshot_keeps(id, snapshot_ids, dirty_ids));
}
