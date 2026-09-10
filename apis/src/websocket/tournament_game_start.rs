use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use db_lib::models::Game;
use shared_types::GameId;
use std::{collections::HashMap, sync::RwLock};
use uuid::Uuid;

#[derive(Debug)]
pub struct TournamentGameStart {
    pub games_date: RwLock<HashMap<GameId, (Uuid, DateTime<Utc>)>>,
}

impl TournamentGameStart {
    pub fn new() -> Self {
        Self {
            games_date: RwLock::new(HashMap::new()),
        }
    }

    pub fn should_start(&self, game: &Game, user_id: Uuid) -> Result<bool> {
        if let Ok(mut games_date) = self.games_date.try_write() {
            let now = Utc::now();
            // The 35-second window above means anything older is dead state.
            // Sweep on insert (cheap O(N) since N is concurrent active starts,
            // not lifetime tournament games). Without this, every tournament
            // game ever started leaked one entry forever.
            let cutoff = now - chrono::Duration::seconds(60);
            games_date.retain(|_, (_, ts)| *ts > cutoff);

            if let Some((uuid, then)) = games_date.get_mut(&GameId(game.nanoid.clone())) {
                let since = now.signed_duration_since(then).abs().num_seconds();
                if *uuid == user_id {
                    games_date.insert(GameId(game.nanoid.clone()), (user_id, now));
                    return Ok(false);
                }
                if since < 35 {
                    return Ok(true);
                }
            }
            games_date.insert(GameId(game.nanoid.clone()), (user_id, now));
            Ok(false)
        } else {
            println!("Could not aquire write lock for TournamentGameStart");
            Err(anyhow!(
                "Could not aquire write lock for TournamentGameStart"
            ))
        }
    }

    pub fn forget(&self, game_id: &GameId) {
        if let Ok(mut games_date) = self.games_date.write() {
            games_date.remove(game_id);
        }
    }
}

impl Default for TournamentGameStart {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forgetting_a_stale_ready_proposal_removes_process_state() {
        let starts = TournamentGameStart::new();
        let game_id = GameId(String::from("stale-ready"));
        starts
            .games_date
            .write()
            .expect("ready state lock")
            .insert(game_id.clone(), (Uuid::new_v4(), Utc::now()));

        starts.forget(&game_id);

        assert!(!starts
            .games_date
            .read()
            .expect("ready state lock")
            .contains_key(&game_id));
    }
}
