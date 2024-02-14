use super::auth_context::AuthContext;
use super::navigation_controller::NavigationControllerSignal;
use crate::responses::game::GameResponse;
use chrono::{DateTime, Utc};
use hive_lib::color::Color;
use hive_lib::game_control::GameControl;
use leptos::*;
use shared_types::time_mode::TimeMode;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::HashMap;

#[derive(Clone, Debug, Copy)]
pub struct GamesSignal {
    pub own: RwSignal<OwnGames>,
    pub live: RwSignal<LiveGames>,
}

impl Default for GamesSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl GamesSignal {
    pub fn new() -> Self {
        Self {
            own: create_rw_signal(OwnGames::new()),
            live: create_rw_signal(LiveGames::new()),
        }
    }

    pub fn visit(&mut self, time_mode: TimeMode) -> Option<String> {
        let navigation_controller = expect_context::<NavigationControllerSignal>();
        let auth_context = expect_context::<AuthContext>();
        if let Some(Ok(Some(user))) = untrack(auth_context.user) {
            self.own.update(|s| {
                if let Some(nanoid) = navigation_controller.signal.get_untracked().nanoid {
                    if let Some(game) = s.untimed.get(&nanoid) {
                        if game.current_player_id == user.id {
                            if let Some(gp) =
                                s.next_untimed.clone().iter().find(|gp| gp.nanoid == nanoid)
                            {
                                s.next_untimed.retain(|gp| gp.nanoid != nanoid);
                                if let Ok(time_left) = game.time_left() {
                                    s.next_untimed.push(GamePriority {
                                        last_interaction: gp.last_interaction,
                                        time_left,
                                        skipped: gp.skipped + 1,
                                        nanoid: gp.nanoid.clone(),
                                    });
                                }
                            }
                        }
                    }
                    if let Some(game) = s.realtime.get(&nanoid) {
                        if game.current_player_id == user.id {
                            if let Some(gp) = s
                                .next_realtime
                                .clone()
                                .iter()
                                .find(|gp| gp.nanoid == nanoid)
                            {
                                s.next_realtime.retain(|gp| gp.nanoid != nanoid);
                                if let Ok(time_left) = game.time_left() {
                                    s.next_realtime.push(GamePriority {
                                        last_interaction: gp.last_interaction,
                                        time_left,
                                        skipped: gp.skipped + 1,
                                        nanoid: gp.nanoid.clone(),
                                    });
                                }
                            }
                        }
                    }
                    if let Some(game) = s.correspondence.get(&nanoid) {
                        if game.current_player_id == user.id {
                            if let Some(gp) = s
                                .next_correspondence
                                .clone()
                                .iter()
                                .find(|gp| gp.nanoid == nanoid)
                            {
                                s.next_correspondence.retain(|gp| gp.nanoid != nanoid);
                                if let Ok(time_left) = game.time_left() {
                                    s.next_correspondence.push(GamePriority {
                                        last_interaction: gp.last_interaction,
                                        time_left,
                                        skipped: gp.skipped + 1,
                                        nanoid: gp.nanoid.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            });
            return match time_mode {
                TimeMode::RealTime => self
                    .own
                    .get_untracked()
                    .next_realtime
                    .peek()
                    .map(|gp| gp.nanoid.clone()),
                TimeMode::Correspondence => self
                    .own
                    .get_untracked()
                    .next_correspondence
                    .peek()
                    .map(|gp| gp.nanoid.clone()),
                TimeMode::Untimed => self
                    .own
                    .get_untracked()
                    .next_untimed
                    .peek()
                    .map(|gp| gp.nanoid.clone()),
            };
        };
        None
    }

    pub fn own_games_add(&mut self, game: GameResponse) {
        let auth_context = expect_context::<AuthContext>();
        let mut next_required = false;
        let mut player_color = Color::White;
        if let Some(Ok(Some(user))) = untrack(auth_context.user) {
            if game.current_player_id == user.id {
                next_required = true;
            }
            if game.black_player.uid == user.id {
                player_color = Color::Black;
            }
        }
        if let Some(last) = game.game_control_history.last() {
            match &last.1 {
                GameControl::DrawOffer(color) | GameControl::TakebackRequest(color) => {
                    if color != &player_color {
                        next_required = true;
                    }
                }
                _ => {
                }
            }
        }
        self.own.update(|s| {
            let mut update_required = true;
            if let Some(already_present_game) = match game.time_mode {
                TimeMode::Untimed => s.untimed.get(&game.nanoid),
                TimeMode::Correspondence => s.correspondence.get(&game.nanoid),
                TimeMode::RealTime => s.realtime.get(&game.nanoid),
            } {
                if already_present_game.updated_at == game.updated_at {
                    update_required = false;
                }
            };
            if update_required {
                match game.time_mode {
                    TimeMode::Untimed => {
                        s.untimed.insert(game.nanoid.to_owned(), game.clone());
                        s.next_untimed.retain(|gp| gp.nanoid != game.nanoid);
                        if next_required {
                            if let Ok(time_left) = game.time_left() {
                                s.next_untimed.push(GamePriority {
                                    last_interaction: Some(game.updated_at),
                                    time_left,
                                    skipped: 0,
                                    nanoid: game.nanoid.clone(),
                                });
                            }
                        }
                    }
                    TimeMode::Correspondence => {
                        s.correspondence
                            .insert(game.nanoid.to_owned(), game.clone());
                        s.next_correspondence.retain(|gp| gp.nanoid != game.nanoid);
                        if next_required {
                            if let Ok(time_left) = game.time_left() {
                                s.next_correspondence.push(GamePriority {
                                    last_interaction: game.last_interaction,
                                    time_left,
                                    skipped: 0,
                                    nanoid: game.nanoid.clone(),
                                });
                            }
                        }
                    }
                    TimeMode::RealTime => {
                        s.realtime.insert(game.nanoid.to_owned(), game.clone());
                        s.next_realtime.retain(|gp| gp.nanoid != game.nanoid);
                        if next_required {
                            if let Ok(time_left) = game.time_left() {
                                s.next_realtime.push(GamePriority {
                                    last_interaction: game.last_interaction,
                                    time_left,
                                    skipped: 0,
                                    nanoid: game.nanoid.clone(),
                                });
                            }
                        }
                    }
                };
            }
        });
    }

    pub fn own_games_remove(&mut self, game_id: &str) {
        self.own.update(|s| {
            s.realtime.remove(game_id);
            s.next_realtime.retain(|gp| gp.nanoid != game_id);
            s.correspondence.remove(game_id);
            s.next_correspondence.retain(|gp| gp.nanoid != game_id);
            s.untimed.remove(game_id);
            s.next_untimed.retain(|gp| gp.nanoid != game_id);
        });
    }

    pub fn own_games_set(&mut self, games: Vec<GameResponse>) {
        for game in games {
            self.own_games_add(game);
        }
    }

    pub fn live_games_add(&mut self, game: GameResponse) {
        let auth_context = expect_context::<AuthContext>();
        let mut should_show = true;
        if let Some(Ok(Some(user))) = untrack(auth_context.user) {
            if game.black_player.uid == user.id || game.white_player.uid == user.id {
                should_show = false;
            }
        }
        if game.finished {
            self.live_games_remove(&game.nanoid);
        } else if should_show {
            self.live.update(|s| {
                s.live_games.insert(game.nanoid.to_owned(), game);
            });
        }
    }

    pub fn live_games_remove(&mut self, game_id: &str) {
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
    pub nanoid: String,
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
    pub realtime: HashMap<String, GameResponse>,
    pub untimed: HashMap<String, GameResponse>,
    pub correspondence: HashMap<String, GameResponse>,
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
    pub live_games: HashMap<String, GameResponse>,
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
    provide_context(GamesSignal::new())
}
