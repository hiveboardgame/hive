use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use db_lib::{models::Game};
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

    pub fn should_start(
        &self,
        game: &Game,
        user_id: Uuid,
    ) -> Result<bool> {
        if game.black_id != user_id && game.white_id != user_id {
            return Err(anyhow!("Not your game to start"));
        }
        if let Ok(mut games_date) = self.games_date.try_write() {
            if let Some((uuid, then)) = games_date.get_mut(&GameId(game.nanoid.clone())) {
                let since = Utc::now().signed_duration_since(then).abs().num_seconds();
                if *uuid == user_id {
                    games_date.insert(GameId(game.nanoid.clone()), (user_id, Utc::now()));
                    return Ok(false);
                }
                if since < 30 {
                    return Ok(true);
                }
            }
            games_date.insert(GameId(game.nanoid.clone()), (user_id, Utc::now()));
            return Ok(false);
        } else {
            println!("Could not aquire write lock for TournamentGameStart");
            return Err(anyhow!(
                "Could not aquire write lock for TournamentGameStart"
            ));
        }
    }
}

impl Default for TournamentGameStart {
    fn default() -> Self {
        Self::new()
    }
}
