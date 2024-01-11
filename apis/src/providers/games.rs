use super::auth_context::AuthContext;
use crate::responses::game::GameResponse;
use hive_lib::{color::Color, game_control::GameControl};
use leptos::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Copy)]
pub struct GamesSignal {
    pub signal: RwSignal<GamesState>,
}

impl Default for GamesSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl GamesSignal {
    pub fn new() -> Self {
        Self {
            signal: create_rw_signal(GamesState::new()),
        }
    }

    fn update_next_games(&mut self) {
        let auth_context = expect_context::<AuthContext>();
        if let Some(Ok(Some(user))) = untrack(auth_context.user) {
            self.signal.update(|s| {
                s.next_games = s
                    .games
                    .iter()
                    .filter_map(|(nanoid, game)| {
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
                            Some(nanoid.to_owned())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<String>>()
            });
        };
    }

    pub fn visit_game(&mut self, game_id: String) {
        self.signal.update(|s| {
            let mut games = s.next_games.clone();
            games.retain(|g| *g != game_id);
            games.push(game_id);
            s.next_games = games;
        });
    }

    pub fn games_add(&mut self, game: GameResponse) {
        self.signal.update_untracked(|s| {
            s.games.insert(game.nanoid.to_owned(), game);
        });
        self.update_next_games();
    }

    pub fn games_remove(&mut self, game_id: &str) {
        self.signal.update_untracked(|s| {
            s.games.remove(game_id);
        });
        self.update_next_games();
    }

    pub fn games_set(&mut self, games: Vec<GameResponse>) {
        for game in games {
            self.signal.update_untracked(|s| {
                s.games.insert(game.nanoid.to_owned(), game);
            });
        }
        self.update_next_games();
    }
}

#[derive(Clone, Debug)]
pub struct GamesState {
    pub games: HashMap<String, GameResponse>,
    pub next_games: Vec<String>,
}

impl GamesState {
    pub fn new() -> Self {
        Self {
            next_games: Vec::new(),
            games: HashMap::new(),
        }
    }
}

impl Default for GamesState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn provide_games() {
    provide_context(GamesSignal::new())
}
