use super::AuthContext;
use crate::responses::AccountResponse;
use crate::responses::GameResponse;
use chrono::{DateTime, Utc};
use hive_lib::{Color, GameControl};
use leptos::prelude::*;
use shared_types::GameId;
use shared_types::TimeMode;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;

#[derive(Clone, Debug, Copy)]
pub struct GamesSignal {
    pub own: RwSignal<OwnGames>,
    pub live: RwSignal<LiveGames>,
    user: Signal<Option<AccountResponse>>,
}

impl GamesSignal {
    pub fn new(user: Signal<Option<AccountResponse>>) -> Self {
        Self {
            own: RwSignal::new(OwnGames::new()),
            live: RwSignal::new(LiveGames::new()),
            user,
        }
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
        let mut next_required = false;
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
                GameControl::DrawOffer(color) | GameControl::TakebackRequest(color) => {
                    if color != &player_color {
                        next_required = true;
                    }
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

    pub fn live_games_add(&mut self, game: GameResponse) {
        let mut should_show = true;
        self.user.with_untracked(|a| {
            if let Some(user) = a {
                if game.black_player.uid == user.id || game.white_player.uid == user.id {
                    should_show = false;
                }
            }
        });
        if game.finished {
            self.live_games_remove(&game.game_id);
        } else if should_show {
            self.live.update(|s| {
                s.live_games.insert(game.game_id.to_owned(), game);
            });
        }
    }

    pub fn live_games_remove(&mut self, game_id: &GameId) {
        self.live.update(|s| {
            s.live_games.remove(game_id);
        });
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
