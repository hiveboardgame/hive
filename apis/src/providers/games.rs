use super::{
    snapshot::{apply_snapshot_hash_map, retain_snapshot_hash_map, snapshot_keeps},
    AuthContext,
};
use crate::responses::{AccountResponse, GameResponse};
use chrono::{DateTime, Utc};
use hive_lib::{Color, GameControl};
use leptos::prelude::*;
use shared_types::{GameId, TimeMode};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
};

#[derive(Clone, Debug, Copy)]
pub struct GamesSignal {
    pub own: RwSignal<OwnGames>,
    pub live: RwSignal<LiveGames>,
    user: Signal<Option<AccountResponse>>,
    /// Live (TV) game IDs touched since the last `begin_resync`. Consulted by
    /// `live_snapshot_apply` so a TV game that arrived during the resync
    /// window isn't dropped by an older snapshot.
    live_resync_dirty: StoredValue<HashSet<GameId>>,
    /// Own/urgent game IDs touched since the last `begin_resync`. Same role
    /// for `urgent_snapshot_apply`.
    own_resync_dirty: StoredValue<HashSet<GameId>>,
}

impl GamesSignal {
    pub fn new(user: Signal<Option<AccountResponse>>) -> Self {
        Self {
            own: RwSignal::new(OwnGames::new()),
            live: RwSignal::new(LiveGames::new()),
            user,
            live_resync_dirty: StoredValue::new(HashSet::new()),
            own_resync_dirty: StoredValue::new(HashSet::new()),
        }
    }

    pub fn begin_resync(&self) {
        self.live_resync_dirty.update_value(|d| d.clear());
        self.own_resync_dirty.update_value(|d| d.clear());
    }

    pub fn visit(&mut self, time_mode: TimeMode, game_id: GameId) -> Option<GameId> {
        let user_id = self.user.with_untracked(|u| u.as_ref().map(|user| user.id));
        if let Some(user_id) = user_id {
            self.own.update(|s| {
                if let Some(game) = s.untimed.get(&game_id) {
                    if game.current_player_id == user_id {
                        if let Some(gp) = s
                            .next_untimed
                            .clone()
                            .iter()
                            .find(|gp| gp.game_id == game_id)
                        {
                            s.next_untimed.retain(|gp| gp.game_id != game_id);
                            if let Ok(time_left) = game.time_left() {
                                s.next_untimed.push(GamePriority {
                                    last_interaction: gp.last_interaction,
                                    time_left,
                                    skipped: gp.skipped + 1,
                                    game_id: gp.game_id.clone(),
                                });
                            }
                        }
                    }
                } else if let Some(game) = s.realtime.get(&game_id) {
                    if game.current_player_id == user_id {
                        if let Some(gp) = s
                            .next_realtime
                            .clone()
                            .iter()
                            .find(|gp| gp.game_id == game_id)
                        {
                            s.next_realtime.retain(|gp| gp.game_id != game_id);
                            if let Ok(time_left) = game.time_left() {
                                s.next_realtime.push(GamePriority {
                                    last_interaction: gp.last_interaction,
                                    time_left,
                                    skipped: gp.skipped + 1,
                                    game_id: gp.game_id.clone(),
                                });
                            }
                        }
                    }
                } else if let Some(game) = s.correspondence.get(&game_id) {
                    if game.current_player_id == user_id {
                        if let Some(gp) = s
                            .next_correspondence
                            .clone()
                            .iter()
                            .find(|gp| gp.game_id == game_id)
                        {
                            s.next_correspondence.retain(|gp| gp.game_id != game_id);
                            if let Ok(time_left) = game.time_left() {
                                s.next_correspondence.push(GamePriority {
                                    last_interaction: gp.last_interaction,
                                    time_left,
                                    skipped: gp.skipped + 1,
                                    game_id: gp.game_id.clone(),
                                });
                            }
                        }
                    }
                }
            });
            return self.own.with_untracked(|s| match time_mode {
                TimeMode::RealTime => s.next_realtime.peek().map(|gp| gp.game_id.clone()),
                TimeMode::Correspondence => {
                    s.next_correspondence.peek().map(|gp| gp.game_id.clone())
                }
                TimeMode::Untimed => s.next_untimed.peek().map(|gp| gp.game_id.clone()),
            });
        };
        None
    }

    pub fn own_games_add(&mut self, game: GameResponse) {
        self.own_games_insert(game, true, false);
    }

    fn own_games_insert(&mut self, game: GameResponse, mark_dirty: bool, force_next: bool) {
        if mark_dirty {
            self.own_resync_dirty.update_value(|d| {
                d.insert(game.game_id.clone());
            });
        }
        let mut next_required = force_next;
        let mut player_color = Color::White;
        self.user.with_untracked(|a| {
            if let Some(user) = a {
                if game.current_player_id == user.id {
                    next_required = true;
                }
                if game.black_player.uid == user.id {
                    player_color = Color::Black;
                }
            }
        });
        if let Some(last) = game.game_control_history.last() {
            match &last.1 {
                GameControl::DrawOffer(color) | GameControl::TakebackRequest(color)
                    if color != &player_color =>
                {
                    next_required = true;
                }
                _ => {}
            }
        }
        self.own.update(|s| {
            let mut update_required = true;
            if let Some(already_present_game) = match game.time_mode {
                TimeMode::Untimed => s.untimed.get(&game.game_id),
                TimeMode::Correspondence => s.correspondence.get(&game.game_id),
                TimeMode::RealTime => s.realtime.get(&game.game_id),
            } {
                if already_present_game.updated_at == game.updated_at {
                    update_required = false;
                }
            };
            if update_required {
                match game.time_mode {
                    TimeMode::Untimed => {
                        s.untimed.insert(game.game_id.to_owned(), game.clone());
                        s.next_untimed.retain(|gp| gp.game_id != game.game_id);
                        if next_required {
                            if let Ok(time_left) = game.time_left() {
                                s.next_untimed.push(GamePriority {
                                    last_interaction: Some(game.updated_at),
                                    time_left,
                                    skipped: 0,
                                    game_id: game.game_id.clone(),
                                });
                            }
                        }
                        if game.finished {
                            s.next_untimed.retain(|gp| gp.game_id != game.game_id);
                        }
                    }
                    TimeMode::Correspondence => {
                        s.correspondence
                            .insert(game.game_id.to_owned(), game.clone());
                        s.next_correspondence
                            .retain(|gp| gp.game_id != game.game_id);
                        if next_required {
                            if let Ok(time_left) = game.time_left() {
                                s.next_correspondence.push(GamePriority {
                                    last_interaction: game.last_interaction,
                                    time_left,
                                    skipped: 0,
                                    game_id: game.game_id.clone(),
                                });
                            }
                        }
                        if game.finished {
                            s.next_correspondence
                                .retain(|gp| gp.game_id != game.game_id);
                        }
                    }
                    TimeMode::RealTime => {
                        s.realtime.insert(game.game_id.to_owned(), game.clone());
                        s.next_realtime.retain(|gp| gp.game_id != game.game_id);
                        if next_required {
                            if let Ok(time_left) = game.time_left() {
                                s.next_realtime.push(GamePriority {
                                    last_interaction: game.last_interaction,
                                    time_left,
                                    skipped: 0,
                                    game_id: game.game_id.clone(),
                                });
                            }
                            if game.finished {
                                s.next_realtime.retain(|gp| gp.game_id != game.game_id);
                            }
                        }
                    }
                };
            }
        });
    }

    pub fn own_games_remove(&mut self, game_id: &GameId) {
        self.own_resync_dirty.update_value(|d| {
            d.insert(game_id.clone());
        });
        self.own.update(|s| {
            s.realtime.remove(game_id);
            s.next_realtime.retain(|gp| gp.game_id != *game_id);
            s.correspondence.remove(game_id);
            s.next_correspondence.retain(|gp| gp.game_id != *game_id);
            s.untimed.remove(game_id);
            s.next_untimed.retain(|gp| gp.game_id != *game_id);
        });
    }

    pub fn own_games_set(&mut self, games: Vec<GameResponse>) {
        for game in games {
            self.own_games_add(game);
        }
    }

    fn should_show_on_live_tv(&self, game: &GameResponse) -> bool {
        let viewer = self.user.with_untracked(|a| a.as_ref().map(|u| u.id));
        tv_visible_to(
            game.finished,
            game.white_player.uid,
            game.black_player.uid,
            viewer,
        )
    }

    pub fn live_games_add(&mut self, game: GameResponse) {
        self.live_resync_dirty.update_value(|d| {
            d.insert(game.game_id.clone());
        });
        if game.finished {
            self.live.update(|s| {
                s.live_games.remove(&game.game_id);
            });
            return;
        }
        if self.should_show_on_live_tv(&game) {
            self.live.update(|s| {
                s.live_games.insert(game.game_id.to_owned(), game);
            });
        }
    }

    pub fn live_games_remove(&mut self, game_id: &GameId) {
        self.live_resync_dirty.update_value(|d| {
            d.insert(game_id.clone());
        });
        self.live.update(|s| {
            s.live_games.remove(game_id);
        });
    }

    /// Race-safe replace of the TV set. Local entries touched by `live_games_add`
    /// or `live_games_remove` since the last `begin_resync` are preserved even
    /// if absent from the snapshot — those incremental updates ran AFTER the
    /// server's snapshot was collected, so they're newer.
    pub fn live_snapshot_apply(&mut self, games: Vec<GameResponse>) {
        let viewer_uid = self.user.with_untracked(|a| a.as_ref().map(|u| u.id));
        let to_insert: Vec<GameResponse> = games
            .into_iter()
            .filter(|game| {
                tv_visible_to(
                    game.finished,
                    game.white_player.uid,
                    game.black_player.uid,
                    viewer_uid,
                )
            })
            .collect();
        let dirty: HashSet<GameId> = self.live_resync_dirty.with_value(|d| d.clone());
        let snapshot_ids: HashSet<GameId> = to_insert.iter().map(|g| g.game_id.clone()).collect();
        self.live.update(|s| {
            apply_snapshot_hash_map(
                &mut s.live_games,
                &snapshot_ids,
                &dirty,
                to_insert,
                |game| game.game_id.clone(),
            );
        });
        self.live_resync_dirty.update_value(|d| d.clear());
    }

    /// Race-safe replace of the user's urgent games. Same shape as
    /// `live_snapshot_apply`, but spans the three time-mode buckets and the
    /// matching priority heaps inside `OwnGames`.
    pub fn urgent_snapshot_apply(&mut self, games: Vec<GameResponse>) {
        let dirty: HashSet<GameId> = self.own_resync_dirty.with_value(|d| d.clone());
        let snapshot_ids: HashSet<GameId> = games.iter().map(|g| g.game_id.clone()).collect();
        self.own.update(|s| {
            retain_snapshot_hash_map(&mut s.realtime, &snapshot_ids, &dirty);
            retain_snapshot_hash_map(&mut s.untimed, &snapshot_ids, &dirty);
            retain_snapshot_hash_map(&mut s.correspondence, &snapshot_ids, &dirty);
            s.next_realtime = s
                .next_realtime
                .drain()
                .filter(|gp| snapshot_keeps(&gp.game_id, &snapshot_ids, &dirty))
                .collect();
            s.next_untimed = s
                .next_untimed
                .drain()
                .filter(|gp| snapshot_keeps(&gp.game_id, &snapshot_ids, &dirty))
                .collect();
            s.next_correspondence = s
                .next_correspondence
                .drain()
                .filter(|gp| snapshot_keeps(&gp.game_id, &snapshot_ids, &dirty))
                .collect();
        });
        for game in games {
            if dirty.contains(&game.game_id) {
                continue;
            }
            self.own_games_insert(game, false, true);
        }
        // End this resync window after the authoritative snapshot has been
        // merged with locally dirty updates.
        self.own_resync_dirty.update_value(|d| d.clear());
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct GamePriority {
    pub last_interaction: Option<DateTime<Utc>>,
    pub time_left: std::time::Duration,
    pub skipped: usize,
    pub game_id: GameId,
}

impl Ord for GamePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .skipped
            .cmp(&self.skipped)
            .then_with(|| other.time_left.cmp(&self.time_left))
    }
}

impl PartialOrd for GamePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub struct OwnGames {
    pub realtime: HashMap<GameId, GameResponse>,
    pub untimed: HashMap<GameId, GameResponse>,
    pub correspondence: HashMap<GameId, GameResponse>,
    pub next_realtime: BinaryHeap<GamePriority>,
    pub next_untimed: BinaryHeap<GamePriority>,
    pub next_correspondence: BinaryHeap<GamePriority>,
}

impl OwnGames {
    pub fn new() -> Self {
        Self {
            realtime: HashMap::new(),
            untimed: HashMap::new(),
            correspondence: HashMap::new(),
            next_realtime: BinaryHeap::new(),
            next_untimed: BinaryHeap::new(),
            next_correspondence: BinaryHeap::new(),
        }
    }
}

impl Default for OwnGames {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct LiveGames {
    pub live_games: HashMap<GameId, GameResponse>,
}

impl LiveGames {
    pub fn new() -> Self {
        Self {
            live_games: HashMap::new(),
        }
    }
}

impl Default for LiveGames {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_games() {
    let auth_context = expect_context::<AuthContext>();
    provide_context(GamesSignal::new(auth_context.user))
}

/// A game is shown on TV unless it's finished or the viewer is one of the
/// players (their own game is rendered elsewhere). Pulled out as a free
/// function so the predicate can be unit-tested without a Leptos runtime.
fn tv_visible_to(
    finished: bool,
    white_uid: uuid::Uuid,
    black_uid: uuid::Uuid,
    viewer_uid: Option<uuid::Uuid>,
) -> bool {
    if finished {
        return false;
    }
    match viewer_uid {
        Some(uid) => white_uid != uid && black_uid != uid,
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::tv_visible_to;
    use uuid::Uuid;

    #[test]
    fn anonymous_viewer_sees_any_ongoing_game() {
        let white = Uuid::new_v4();
        let black = Uuid::new_v4();
        assert!(tv_visible_to(false, white, black, None));
    }

    #[test]
    fn finished_games_are_hidden_even_from_strangers() {
        let white = Uuid::new_v4();
        let black = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        assert!(!tv_visible_to(true, white, black, None));
        assert!(!tv_visible_to(true, white, black, Some(stranger)));
    }

    #[test]
    fn players_dont_see_their_own_game_on_tv() {
        let white = Uuid::new_v4();
        let black = Uuid::new_v4();
        assert!(!tv_visible_to(false, white, black, Some(white)));
        assert!(!tv_visible_to(false, white, black, Some(black)));
    }

    #[test]
    fn third_party_sees_the_game() {
        let white = Uuid::new_v4();
        let black = Uuid::new_v4();
        let stranger = Uuid::new_v4();
        assert!(tv_visible_to(false, white, black, Some(stranger)));
    }
}
