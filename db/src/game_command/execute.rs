use crate::{
    db_error::DbError,
    game_command::{Command, Outcome},
    models::{
        DeadlineSettlement,
        FlagGrace,
        Game,
        ScheduleOffer,
        Tournament,
        TournamentSlot,
        User,
    },
    tournaments::settle_arena_game,
    DbConn,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use diesel_async::AsyncConnection;
use hive_lib::{Color, GameControl, GameResult, GameStatus, State, Turn};
use shared_types::{tournament::FormatConfig, Conclusion, GameStart, RealtimeClock};
use std::str::FromStr;
use uuid::Uuid;

pub const MAX_MOVE_COMPENSATION_SECS: f64 = 1.0;

// Past this, no move still on its way can be compensated back under the deadline.
pub fn real_time_flag_grace() -> ChronoDuration {
    ChronoDuration::milliseconds((MAX_MOVE_COMPENSATION_SECS * 1000.0) as i64)
}

fn accepted_compensation(compensation: f64) -> Result<(), DbError> {
    if compensation.is_finite() && (0.0..=MAX_MOVE_COMPENSATION_SECS).contains(&compensation) {
        return Ok(());
    }
    Err(DbError::InvalidInput {
        info: String::from("Move compensation is outside its accepted range"),
        error: format!(
            "compensation must be finite and between 0 and {MAX_MOVE_COMPENSATION_SECS} seconds"
        ),
    })
}

// The player on move waits out only what their own move earned. Anyone else, and the deadline
// sweep, must allow for a move of theirs still on its way.
fn flag_grace_for(command: &Command, game: &Game) -> FlagGrace {
    let on_move = game.player_id(game.color_on_move());
    match command {
        Command::Move {
            user_id,
            compensation,
            arrived_at,
            ..
        } if *user_id == on_move => FlagGrace::Earned {
            grace: ChronoDuration::nanoseconds((compensation * 1_000_000_000.0) as i64)
                .min(game.flag_grace()),
            own_arrival: *arrived_at,
        },
        Command::Control { user_id, .. }
        | Command::ReadyIntent { user_id }
        | Command::Berserk { user_id }
            if *user_id == on_move =>
        {
            FlagGrace::ended()
        }
        Command::CheckTime { user_id, turn } if *user_id == on_move && *turn == game.turn => {
            FlagGrace::ended()
        }
        _ => FlagGrace::MovesInFlight,
    }
}

// The mover is charged up to the move's arrival; the opponent's clock starts at the commit, so
// time the server spends in between is charged to nobody.
fn move_arrival(arrived_at: Option<DateTime<Utc>>, effective_at: DateTime<Utc>) -> DateTime<Utc> {
    arrived_at.map_or(effective_at, |arrived| arrived.min(effective_at))
}

pub async fn execute(
    game_id: Uuid,
    command: Command,
    conn: &mut DbConn<'_>,
) -> Result<Outcome, DbError> {
    execute_with_clock(game_id, command, Utc::now, conn).await
}

#[cfg(test)]
pub(crate) async fn execute_at(
    game_id: Uuid,
    command: Command,
    effective_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<Outcome, DbError> {
    execute_with_clock(game_id, command, move || effective_at, conn).await
}

async fn execute_with_clock<F>(
    game_id: Uuid,
    command: Command,
    effective_at: F,
    conn: &mut DbConn<'_>,
) -> Result<Outcome, DbError>
where
    F: FnOnce() -> DateTime<Utc> + Send,
{
    if let Command::StartReady { user_id } = &command {
        return execute_ready_start(game_id, *user_id, effective_at, conn).await;
    }
    conn.transaction::<_, DbError, _>(async move |tc| {
        let game = Game::find_by_uuid_for_update(&game_id, tc).await?;
        let binding = GameBinding::for_game(&game)?;
        let effective_at = effective_at();
        let checked_at = match &command {
            Command::Move {
                compensation,
                arrived_at,
                ..
            } => {
                accepted_compensation(*compensation)?;
                move_arrival(*arrived_at, effective_at)
            }
            _ => effective_at,
        };
        let grace = flag_grace_for(&command, &game);
        let settlement = game.settle_deadline_allowing(checked_at, grace, tc).await?;
        let (checked, newly_terminal, terminal_at) = match settlement {
            // Played now, a move past its own deadline would leave no time on the clock, while
            // another move of theirs could still be in time.
            DeadlineSettlement::Awaiting(_)
                if matches!(command, Command::Move { .. })
                    && matches!(grace, FlagGrace::Earned { .. }) =>
            {
                return Err(DbError::InvalidAction {
                    info: String::from("Another move of yours in this game is still being played"),
                });
            }
            DeadlineSettlement::Active(game) | DeadlineSettlement::Awaiting(game) => {
                (game, false, None)
            }
            DeadlineSettlement::Terminal {
                game, terminal_at, ..
            } => (game, true, Some(terminal_at)),
        };
        if let Some(terminal_at) = terminal_at {
            settle_terminal_tournament_game(&checked, &binding, terminal_at, effective_at, tc)
                .await?;
        }

        if matches!(
            &command,
            Command::SettleDeadline | Command::CheckTime { .. }
        ) {
            return Ok(Outcome::Applied {
                game: checked,
                newly_terminal,
                schedule_offer_updates: Vec::new(),
            });
        }

        if checked.finished {
            return if newly_terminal {
                Ok(Outcome::TimedOut {
                    game: checked,
                    rejected: DbError::GameIsOver,
                    schedule_offer_updates: Vec::new(),
                })
            } else {
                Err(DbError::GameIsOver)
            };
        }

        let actor_id = command_actor(&command);
        User::ensure_active_ids(&[actor_id], tc).await?;
        let applied = apply_command(checked, &binding, command, effective_at, tc).await?;

        match applied {
            AppliedCommand::Removed(previous) => Ok(Outcome::Removed { previous }),
            AppliedCommand::Game(game) => {
                if game.finished {
                    settle_terminal_tournament_game(
                        &game,
                        &binding,
                        effective_at,
                        effective_at,
                        tc,
                    )
                    .await?;
                }
                Ok(Outcome::Applied {
                    newly_terminal: game.finished,
                    game,
                    schedule_offer_updates: Vec::new(),
                })
            }
        }
    })
    .await
}

async fn execute_ready_start<F>(
    game_id: Uuid,
    user_id: Uuid,
    effective_at: F,
    conn: &mut DbConn<'_>,
) -> Result<Outcome, DbError>
where
    F: FnOnce() -> DateTime<Utc> + Send,
{
    let binding_ids = load_game_binding_ids(game_id, conn).await?;
    let (Some(tournament_id), Some(slot_id)) =
        (binding_ids.tournament_id, binding_ids.tournament_slot_id)
    else {
        return Err(DbError::InvalidAction {
            info: String::from("Only a fixed-field Ready game uses the handshake start"),
        });
    };
    conn.transaction::<_, DbError, _>(async move |tc| {
        let slot = TournamentSlot::find_for_update(tournament_id, slot_id, tc).await?;
        let game = Game::find_by_uuid_for_update(&game_id, tc).await?;
        if game.tournament_id != binding_ids.tournament_id
            || game.tournament_slot_id != binding_ids.tournament_slot_id
        {
            return Err(DbError::InvalidPersistedTournament {
                reason: String::from("Game tournament binding changed during Ready start"),
            });
        }
        if slot.resolution.is_some() {
            return Err(DbError::InvalidAction {
                info: String::from("A resolved tournament Slot cannot start a Game"),
            });
        }
        let effective_at = effective_at();
        let game = match game.settle_deadline(effective_at, tc).await? {
            DeadlineSettlement::Active(game) | DeadlineSettlement::Awaiting(game) => game,
            DeadlineSettlement::Terminal { game, .. } => {
                return Ok(Outcome::TimedOut {
                    game,
                    rejected: DbError::GameIsOver,
                    schedule_offer_updates: Vec::new(),
                });
            }
        };
        if game.finished {
            return Err(DbError::GameIsOver);
        }
        User::ensure_active_ids(&[user_id], tc).await?;
        ensure_player(&game, user_id)?;
        if game.game_start != GameStart::Ready.to_string()
            || game.turn != 0
            || game.game_status != GameStatus::NotStarted.to_string()
        {
            return Err(DbError::InvalidAction {
                info: String::from("Cannot start this Ready game"),
            });
        }
        let game = game.start(effective_at, tc).await?;
        let schedule_updates =
            ScheduleOffer::close_pending_for_slot(tournament_id, slot_id, effective_at, tc).await?;
        Ok(Outcome::Applied {
            game,
            newly_terminal: false,
            schedule_offer_updates: schedule_updates,
        })
    })
    .await
}

fn command_actor(command: &Command) -> Uuid {
    match command {
        Command::Move { user_id, .. }
        | Command::Control { user_id, .. }
        | Command::StartReady { user_id }
        | Command::ReadyIntent { user_id }
        | Command::Berserk { user_id } => *user_id,
        Command::SettleDeadline | Command::CheckTime { .. } => {
            unreachable!("deadline checks return before actor lookup")
        }
    }
}

#[derive(Clone, Copy)]
struct GameBindingIds {
    tournament_id: Option<Uuid>,
    tournament_slot_id: Option<Uuid>,
}

async fn load_game_binding_ids(
    game_id: Uuid,
    conn: &mut DbConn<'_>,
) -> Result<GameBindingIds, DbError> {
    let game = Game::find_by_uuid(&game_id, conn).await?;
    Ok(GameBindingIds {
        tournament_id: game.tournament_id,
        tournament_slot_id: game.tournament_slot_id,
    })
}

enum GameBinding {
    Ordinary,
    Arena {
        tournament_id: Uuid,
        clock: RealtimeClock,
    },
    FixedField,
}

impl GameBinding {
    fn for_game(game: &Game) -> Result<Self, DbError> {
        match (
            game.tournament_id,
            game.tournament_slot_id,
            game.arena_ordinal,
        ) {
            (None, None, None) => Ok(Self::Ordinary),
            (Some(_), Some(_), None) => Ok(Self::FixedField),
            (Some(tournament_id), None, Some(_)) => Ok(Self::Arena {
                tournament_id,
                clock: arena_clock(game)?,
            }),
            _ => Err(DbError::InvalidPersistedTournament {
                reason: String::from("Game has an invalid tournament ownership shape"),
            }),
        }
    }
}

fn arena_clock(game: &Game) -> Result<RealtimeClock, DbError> {
    game.realtime_clock()?
        .ok_or_else(|| DbError::InvalidPersistedTournament {
            reason: String::from("Arena Game does not have a realtime clock"),
        })
}

async fn settle_terminal_tournament_game(
    game: &Game,
    binding: &GameBinding,
    terminal_at: DateTime<Utc>,
    observed_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<(), DbError> {
    match binding {
        GameBinding::Ordinary | GameBinding::FixedField => Ok(()),
        GameBinding::Arena { tournament_id, .. } => {
            settle_arena_game(*tournament_id, game, terminal_at, observed_at, conn).await?;
            Ok(())
        }
    }
}

enum AppliedCommand {
    Game(Game),
    Removed(Game),
}

async fn apply_command(
    game: Game,
    binding: &GameBinding,
    command: Command,
    effective_at: DateTime<Utc>,
    conn: &mut DbConn<'_>,
) -> Result<AppliedCommand, DbError> {
    match (binding, &command) {
        (
            GameBinding::Arena { .. } | GameBinding::FixedField,
            Command::Control {
                control:
                    GameControl::TakebackRequest(_)
                    | GameControl::TakebackAccept(_)
                    | GameControl::TakebackReject(_),
                ..
            },
        ) => {
            return Err(DbError::InvalidAction {
                info: String::from("Tournament games do not allow takebacks"),
            })
        }
        (
            GameBinding::Arena { .. } | GameBinding::FixedField,
            Command::Control {
                control: GameControl::Abort(_),
                ..
            },
        ) => {
            return Err(DbError::InvalidAction {
                info: String::from("Tournament games cannot be aborted"),
            })
        }
        (GameBinding::Arena { .. }, Command::ReadyIntent { .. }) => {
            return Err(DbError::InvalidAction {
                info: String::from("Arena games do not use the Ready handshake"),
            })
        }
        (GameBinding::Ordinary, Command::ReadyIntent { .. }) => {
            return Err(DbError::InvalidAction {
                info: String::from("Only a fixed-field Ready game uses the handshake start"),
            })
        }
        _ => {}
    }

    let game = match command {
        Command::Move {
            user_id,
            turn,
            compensation,
            expected_turn,
            arrived_at,
        } => {
            let ordinary_opening = matches!(binding, GameBinding::Ordinary)
                && game.game_start == GameStart::Moves.to_string()
                && game.game_status == GameStatus::NotStarted.to_string();
            if !ordinary_opening && game.game_status != GameStatus::InProgress.to_string() {
                return Err(DbError::InvalidAction {
                    info: String::from("This game cannot accept a move yet"),
                });
            }
            if let Some(expected_turn) = expected_turn {
                if usize::try_from(game.turn).ok() != Some(expected_turn) {
                    return Err(DbError::InvalidAction {
                        info: format!(
                            "This move answers turn {expected_turn}, but the game is at turn {}",
                            game.turn
                        ),
                    });
                }
            }
            let mut state = game_state(&game)?;
            let expected_actor = match state.turn_color {
                Color::White => game.white_id,
                Color::Black => game.black_id,
            };
            if user_id != expected_actor {
                return Err(DbError::InvalidAction {
                    info: String::from("It is not this player's turn"),
                });
            }
            let Turn::Move(piece, position) = turn else {
                return Err(DbError::InvalidAction {
                    info: String::from("A client cannot submit a synthetic shutout turn"),
                });
            };
            state
                .play_turn_from_position(piece, position)
                .map_err(|error| DbError::InvalidInput {
                    info: String::from("Invalid game move"),
                    error: error.to_string(),
                })?;
            game.update_gamestate(
                &state,
                compensation,
                move_arrival(arrived_at, effective_at),
                effective_at,
                conn,
            )
            .await?
        }
        Command::Control { user_id, control } => {
            validate_control(&game, binding, user_id, control)?;
            match control {
                GameControl::Abort(_) => {
                    game.delete(conn).await?;
                    return Ok(AppliedCommand::Removed(game));
                }
                GameControl::Resign(color) => {
                    game.finish_game_control(
                        control,
                        GameResult::Winner(color.opposite_color()),
                        Conclusion::Resigned,
                        effective_at,
                        conn,
                    )
                    .await?
                }
                GameControl::DrawAccept(_) => {
                    game.finish_game_control(
                        control,
                        GameResult::Draw,
                        Conclusion::Draw,
                        effective_at,
                        conn,
                    )
                    .await?
                }
                GameControl::TakebackAccept(_) => {
                    game.accept_takeback(control, effective_at, conn).await?
                }
                GameControl::DrawOffer(_)
                | GameControl::DrawReject(_)
                | GameControl::TakebackRequest(_)
                | GameControl::TakebackReject(_) => {
                    game.write_game_control(control, effective_at, conn).await?
                }
            }
        }
        Command::StartReady { .. } => unreachable!("StartReady returned before dispatch"),
        Command::ReadyIntent { user_id } => {
            ensure_player(&game, user_id)?;
            if game.game_start != GameStart::Ready.to_string()
                || game.turn != 0
                || game.game_status != GameStatus::NotStarted.to_string()
            {
                return Err(DbError::InvalidAction {
                    info: String::from("Cannot ready this game"),
                });
            }
            game
        }
        Command::Berserk { user_id } => {
            let GameBinding::Arena {
                tournament_id,
                clock,
            } = binding
            else {
                return Err(DbError::InvalidAction {
                    info: String::from("Berserk is available only in Arena games"),
                });
            };
            let color = ensure_player(&game, user_id)?;
            let tournament = Tournament::find(*tournament_id, conn).await?;
            let FormatConfig::Arena(configuration) = &tournament.configuration().format else {
                return Err(DbError::InvalidPersistedTournament {
                    reason: String::from("Arena Game belongs to another tournament format"),
                });
            };
            if configuration.rules.scoring.berserk.is_none() {
                return Err(DbError::InvalidAction {
                    info: String::from("Berserk is disabled for this Arena"),
                });
            }
            game.declare_berserk(color, *clock, effective_at, conn)
                .await?
        }
        Command::SettleDeadline | Command::CheckTime { .. } => {
            unreachable!("deadline checks return before dispatch")
        }
    };
    Ok(AppliedCommand::Game(game))
}

fn game_state(game: &Game) -> Result<State, DbError> {
    let mut state = State::new_from_str(&game.history, &game.game_type).map_err(|error| {
        DbError::InternalError {
            reason: format!("game history does not replay: {error}"),
        }
    })?;
    state.tournament = game.tournament_queen_rule;
    Ok(state)
}

fn ensure_player(game: &Game, actor: Uuid) -> Result<Color, DbError> {
    game.user_color(actor).ok_or(DbError::Unauthorized)
}

fn validate_control(
    game: &Game,
    binding: &GameBinding,
    actor: Uuid,
    control: GameControl,
) -> Result<(), DbError> {
    let color = ensure_player(game, actor)?;
    if color != control.color() {
        return Err(DbError::InvalidAction {
            info: String::from("Game control has the wrong color"),
        });
    }
    if matches!(binding, GameBinding::FixedField)
        && game.game_start == GameStart::Ready.to_string()
        && game.game_status == GameStatus::NotStarted.to_string()
    {
        return Err(DbError::InvalidAction {
            info: String::from("An unbegun Ready tournament game cannot accept game controls"),
        });
    }
    let start = GameStart::from_str(&game.game_start).map_err(|error| DbError::InternalError {
        reason: error.to_string(),
    })?;
    let status =
        GameStatus::from_str(&game.game_status).map_err(|error| DbError::InternalError {
            reason: error.to_string(),
        })?;
    let resumed_move_opening = start.is_resumed_move_opening(
        matches!(binding, GameBinding::Ordinary),
        status,
        usize::try_from(game.turn).map_err(|error| DbError::InternalError {
            reason: error.to_string(),
        })?,
    );
    match (binding, control) {
        (GameBinding::Ordinary, GameControl::Abort(_)) if resumed_move_opening => {
            return Err(DbError::InvalidAction {
                info: String::from("A started game cannot be aborted"),
            })
        }
        (GameBinding::Ordinary, GameControl::TakebackRequest(_))
            if resumed_move_opening && game.turn == 0 =>
        {
            return Err(DbError::InvalidAction {
                info: String::from("Takeback failed, no moves to pop"),
            })
        }
        (GameBinding::Ordinary, _) if resumed_move_opening => {}
        (GameBinding::Arena { .. } | GameBinding::FixedField, GameControl::Resign(_)) => {}
        (_, control) if !control.allowed_on_turn(game.turn) => {
            return Err(DbError::InvalidAction {
                info: String::from("Game control is not allowed on this turn"),
            })
        }
        _ => {}
    }
    validate_control_history(game, control)
}

fn validate_control_history(game: &Game, control: GameControl) -> Result<(), DbError> {
    let previous = game.last_game_control()?;
    if previous == Some(control) {
        return Err(DbError::InvalidAction {
            info: String::from("The same game control is already present"),
        });
    }
    let expected = match control {
        GameControl::TakebackAccept(_) | GameControl::TakebackReject(_) => Some(
            GameControl::TakebackRequest(control.color().opposite_color()),
        ),
        GameControl::DrawAccept(_) | GameControl::DrawReject(_) => {
            Some(GameControl::DrawOffer(control.color().opposite_color()))
        }
        GameControl::Abort(_)
        | GameControl::Resign(_)
        | GameControl::DrawOffer(_)
        | GameControl::TakebackRequest(_) => None,
    };
    if expected.is_some() && previous != expected {
        return Err(DbError::InvalidAction {
            info: String::from("Game control does not match the pending request"),
        });
    }
    Ok(())
}
