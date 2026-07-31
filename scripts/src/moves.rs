use anyhow::{Context, Result};
use db_lib::{models::Game, DbConn};
use hive_lib::{Bug, GameStatus, GameType, History, Piece, Position, State};
use rand::{prelude::IndexedRandom, rngs::SmallRng, RngExt, SeedableRng};
use std::{collections::HashMap, str::FromStr};

/// Enough moves for the board to be worth opening, without playing anybody out —
/// the game is then resigned or drawn rather than played to a real finish.
const MOVES_PER_GAME: usize = 20;

/// Queen has to be down by the fourth turn each side, so it is held back before
/// this to keep the openings varied.
const QUEEN_HELD_BACK_UNTIL: usize = 2;

const MOVE_PROBABILITY: f64 = 0.3;

/// Plays a few legal moves into a game, so it has a real history behind it.
///
/// Best-effort: a game that cannot be started or advanced is left alone rather
/// than failing the whole seeding run, since the result is adjudicated either
/// way and an empty board is a cosmetic loss.
pub async fn play_some(game: &Game, conn: &mut DbConn<'_>) -> Result<()> {
    let started = match game.start(conn).await {
        Ok(started) => started,
        Err(error) => {
            tracing::debug!(game = %game.nanoid, %error, "left a game unplayed");
            return Ok(());
        }
    };

    let history = History::new_from_str(&started.history)
        .with_context(|| format!("could not parse the history of {}", started.nanoid))?;
    let mut state = State::new_from_history(&history)
        .with_context(|| format!("could not rebuild the state of {}", started.nanoid))?;
    state.game_type = GameType::from_str(&started.game_type)
        .with_context(|| format!("could not parse the type of {}", started.nanoid))?;

    let mut rng = SmallRng::seed_from_u64(started.id.as_u128() as u64);
    for _ in 0..MOVES_PER_GAME {
        if matches!(state.game_status, GameStatus::Finished(_)) || !play_one(&mut state, &mut rng) {
            break;
        }
    }

    if state.turn > 0 {
        started
            .update_gamestate(&state, 0.0, conn)
            .await
            .with_context(|| format!("could not store the moves of {}", started.nanoid))?;
    }
    Ok(())
}

/// One random legal move or spawn. False when there is nothing left to play.
fn play_one(state: &mut State, rng: &mut SmallRng) -> bool {
    let color = state.turn_color;
    let spawns: Vec<Position> = state.board.spawnable_positions(color).collect();
    let mut reserve = state.reserve(color);
    if state.turn < QUEEN_HELD_BACK_UNTIL {
        reserve.remove(&Bug::Queen);
    }

    let queen_played = state.board.queen_played(color);
    let moves = if queen_played {
        state.board.moves(color)
    } else {
        HashMap::new()
    };

    if moves.is_empty() && (spawns.is_empty() || reserve.is_empty()) {
        return false;
    }

    if queen_played && !moves.is_empty() && rng.random_bool(MOVE_PROBABILITY) {
        let entries: Vec<_> = moves.iter().collect();
        if let Some(((piece, _), targets)) = entries.choose(rng) {
            if let Some(target) = targets.choose(rng) {
                if state.play_turn_from_position(*piece, *target).is_ok() {
                    return true;
                }
            }
        }
    }

    let in_reserve: Vec<String> = reserve.into_values().flatten().collect();
    let Some(candidate) = in_reserve.choose(rng) else {
        return false;
    };
    let Ok(piece) = candidate.parse::<Piece>() else {
        return false;
    };
    let Some(position) = spawns.choose(rng) else {
        return false;
    };
    state.play_turn_from_position(piece, *position).is_ok()
}
