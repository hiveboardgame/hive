use super::auth_context::AuthContext;
use super::navigation_controller::NavigationControllerSignal;
use crate::responses::game::GameResponse;
use chrono::{Duration, Utc};
use hive_lib::{color::Color, game_control::GameControl};
use leptos::logging::log;
use leptos::*;
use shared_types::time_mode::TimeMode;
use std::collections::HashMap;
use std::str::FromStr;

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

    pub fn update_next_games(&mut self) {
        if self.own.get_untracked().games.is_empty() {
            self.own.update(|s| {
                s.next_games.clear();
            });
            return;
        }
        let auth_context = expect_context::<AuthContext>();
        if let Some(Ok(Some(user))) = untrack(auth_context.user) {
            self.own.update(|s| {
                let mut games: Vec<GameResponse> = s.games.values().cloned().collect();
                games.sort_by(|a, b| a.updated_at.cmp(&b.updated_at));
                let mut games_with_time = games
                    .iter()
                    .filter_map(|game| {
                        let not_player_color = if game.black_player.uid == user.id {
                            Color::White
                        } else {
                            Color::Black
                        };
                        let gc = game.game_control_history.last().map(|(_turn, gc)| gc);
                        let unanswered_gc = match gc {
                            Some(GameControl::DrawOffer(color))
                            | Some(GameControl::TakebackRequest(color)) => {
                                *color == not_player_color
                            }
                            _ => false,
                        };
                        if !game.finished && (game.current_player_id == user.id || unanswered_gc) {
                            let time_left = if let Some(last_interaction) = game.last_interaction {
                                let left = match TimeMode::from_str(&game.time_mode) {
                                    Ok(TimeMode::RealTime) | Ok(TimeMode::Correspondence) => {
                                        if game.turn % 2 == 0 {
                                            Duration::from_std(game.white_time_left.unwrap())
                                                .unwrap()
                                        } else {
                                            Duration::from_std(game.black_time_left.unwrap())
                                                .unwrap()
                                        }
                                    }
                                    _ => Duration::days(10_000),
                                };
                                let future = last_interaction.checked_add_signed(left).unwrap();
                                let now = Utc::now();
                                if now > future {
                                    std::time::Duration::from_nanos(0)
                                } else {
                                    future.signed_duration_since(now).to_std().unwrap()
                                }
                            } else if let Some(base) = game.white_time_left {
                                base
                            } else {
                                std::time::Duration::from_secs(u64::MAX)
                            };
                            Some((time_left, game.nanoid.to_owned()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<(std::time::Duration, String)>>();
                games_with_time.sort_by(|a, b| a.0.cmp(&b.0));
                s.next_games = games_with_time.iter().map(|g| g.1.to_owned()).collect();
            });
        }
    }

    pub fn visit_game(&mut self) -> Option<String> {
        let mut next_game = None;
        let navigation_controller = expect_context::<NavigationControllerSignal>();
        self.own.update(|s| {
            let mut games = s.next_games.clone();
            log!("Games in visit: {:?}", games);
            if let Some(nanoid) = navigation_controller.signal.get_untracked().nanoid {
                games.retain(|g| *g != nanoid);
                games.push(nanoid.clone());
            }
            next_game = games.first().cloned();
            s.next_games = games;
            log!("Games after visit: {:?}", s.next_games);
        });
        next_game
    }

    pub fn own_games_add(&mut self, game: GameResponse) {
        let mut update_required = true;
        self.own.update_untracked(|s| {
            if let Some(already_present_game) = s.games.get(&game.nanoid) {
                if already_present_game.updated_at == game.updated_at {
                    update_required = false
                }
            };
            if update_required {
                s.games.insert(game.nanoid.to_owned(), game);
            }
        });
        if update_required {
            self.update_next_games();
        }
    }

    pub fn own_games_remove(&mut self, game_id: &str) {
        self.own.update_untracked(|s| {
            s.games.remove(game_id);
        });
        self.update_next_games();
    }

    pub fn remove_from_next_games(&mut self, game_id: &str) {
        self.own.update_untracked(|s| {
            s.next_games.retain(|g| g != game_id);
        });
        self.update_next_games();
    }

    pub fn own_games_set(&mut self, games: Vec<GameResponse>) {
        for game in games {
            self.own.update_untracked(|s| {
                s.games.insert(game.nanoid.to_owned(), game);
            });
        }
        self.update_next_games();
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

#[derive(Clone, Debug)]
pub struct OwnGames {
    pub games: HashMap<String, GameResponse>,
    pub next_games: Vec<String>,
}

impl OwnGames {
    pub fn new() -> Self {
        Self {
            next_games: Vec::new(),
            games: HashMap::new(),
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
