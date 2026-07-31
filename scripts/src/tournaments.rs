use crate::{
    common::{ensure_pool, setup_database, ORGANIZER, PASSWORD},
    moves,
};
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use db_lib::{
    models::{Game, NewTournament, ProgressOutcome, Tournament, User},
    schema::games,
    DbConn,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::{Color, GameControl};
use rand::{rngs::SmallRng, RngExt, SeedableRng};
use serde::Serialize;
use shared_types::{
    PointSystemDetails,
    ScoringMode,
    StartMode,
    Tiebreaker,
    TimeMode,
    TournamentDetails,
    TournamentGameResult,
    TournamentMode,
};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Every fixture is real time 60+0, which is `GameSpeed::Bullet` — the speed
/// seeding reads ratings from, and a mode `adjudicate_tournament_result` will
/// accept a result for. Correspondence games are created already in progress.
const TIME_BASE: i32 = 60;

/// How long a seeded arena runs for. Long enough that it is still live while
/// somebody browses the site after seeding.
const ARENA_SECONDS: i32 = 3 * 3600;

/// How far apart arena finishes are stamped, in milliseconds.
///
/// An arena replays from a timeline of instants at millisecond resolution, so
/// several finishes adjudicated back to back can share one instant and collide
/// with a pairing — a hard error that kills the arena permanently rather than
/// producing a wrong number. Milliseconds rather than seconds because a finish
/// must still land *before* the next tick's pairing, and `progress()` stamps
/// those at real `now`.
const ARENA_FINISH_SPACING_MS: i64 = 100;

/// Waited out between arena ticks so the next pairing's `created_at` is strictly
/// later than every finish stamped for the previous one. Cheap insurance against
/// an inverted timeline, which replay cannot recover from.
const ARENA_TICK_GAP_MS: u64 = 1_500;

/// What a seeded tournament is left looking like.
#[derive(Clone, Copy, PartialEq)]
pub enum Stage {
    NotStarted,
    InProgress,
    Finished,
}

impl Stage {
    fn label(self) -> &'static str {
        match self {
            Self::NotStarted => "upcoming",
            Self::InProgress => "live",
            Self::Finished => "done",
        }
    }
}

/// How a `NotStarted` tournament is left waiting, so the upcoming-tournament UI
/// gets exercised rather than just one flavour of it.
#[derive(Clone, Copy, PartialEq)]
enum Waiting {
    /// Joined and idle, waiting on the organizer.
    Manual,
    /// Scheduled, so the countdown renders.
    Scheduled,
    /// Nobody joined; the field is invited and has not answered.
    InviteOnly,
}

struct Plan {
    mode: TournamentMode,
    players: usize,
    rounds: i32,
    /// How many rounds to play before stopping, for `InProgress`.
    rounds_before_pausing: usize,
    /// Distinguishes a scenario from the plain tournament of the same mode, in
    /// both the name and the description.
    label: Option<&'static str>,
    /// `(after this many rounds, withdraw this many of the weakest players)`.
    ///
    /// Mid-event departures are their own scoring path: the engine forfeits the
    /// leaver's remaining games, and an even field turning odd starts handing out
    /// byes — neither of which any other fixture reaches.
    withdraw: Option<(usize, usize)>,
    /// After this many rounds, grant one player a bye they asked for.
    ///
    /// A requested bye scores differently from the one the engine allocates when
    /// the field is odd, and `grant_zero_point_bye` has no UI caller at all, so
    /// nothing else exercises it.
    zero_point_bye: Option<usize>,
}

impl Plan {
    fn new(mode: TournamentMode, players: usize, rounds: i32) -> Self {
        Self {
            mode,
            players,
            rounds,
            rounds_before_pausing: 2,
            label: None,
            withdraw: None,
            zero_point_bye: None,
        }
    }
}

/// Field sizes are per mode rather than a flat sixteen: `replay()` is O(games)
/// per read and runs once per `progress()`, so a big field in a repeating format
/// costs quadratic time for no extra coverage.
fn plans() -> Vec<Plan> {
    use TournamentMode::*;
    vec![
        Plan::new(SingleRoundRobin, 8, 0),
        Plan::new(DoubleRoundRobin, 6, 0),
        Plan::new(DutchSwiss, 16, 5),
        Plan::new(BursteinSwiss, 16, 5),
        Plan {
            rounds_before_pausing: 1,
            ..Plan::new(DoubleSwiss, 8, 3)
        },
        Plan::new(SingleElimination, 16, 0),
        Plan::new(DoubleElimination, 8, 0),
    ]
}

/// Tournaments built to reach a specific state rather than to cover a mode.
///
/// Each of these exercises something the plain per-mode fixtures never do, so
/// they carry an explicit stage instead of the usual three.
fn scenarios() -> Vec<(Plan, Stage)> {
    use TournamentMode::*;
    let odd_field = || Plan {
        // Odd, so every round the engine has to hand somebody a bye.
        label: Some("odd field"),
        ..Plan::new(DutchSwiss, 13, 5)
    };
    let withdrawals = || Plan {
        label: Some("withdrawals"),
        // Sixteen down to thirteen: an even field becomes odd, so the rounds
        // after the departures also start allocating byes.
        //
        // The trigger has to come *before* the pause or the live copy would stop
        // without ever reaching it, leaving two tournaments named "withdrawals"
        // where only one had any.
        withdraw: Some((1, 3)),
        rounds_before_pausing: 3,
        ..Plan::new(BursteinSwiss, 16, 5)
    };
    let requested_bye = || Plan {
        label: Some("requested bye"),
        zero_point_bye: Some(1),
        ..Plan::new(DutchSwiss, 12, 4)
    };

    vec![
        // Four players meeting six times each is 36 games in a 4x4 grid — the
        // densest cross-table the formats can produce, and small enough that the
        // quadratic replay cost stays trivial.
        (
            Plan {
                label: Some("cross-table"),
                ..Plan::new(SextupleRoundRobin, 4, 0)
            },
            Stage::Finished,
        ),
        // Not a power of two, so the first round is full of byes and the diagram
        // has to cope with an unbalanced tree.
        (
            Plan {
                label: Some("big field"),
                ..Plan::new(SingleElimination, 55, 0)
            },
            Stage::Finished,
        ),
        (odd_field(), Stage::InProgress),
        (odd_field(), Stage::Finished),
        (withdrawals(), Stage::InProgress),
        (withdrawals(), Stage::Finished),
        (requested_bye(), Stage::InProgress),
        // A round robin keeps every pairing fixed, so a leaver forfeits a known
        // set of games — a different shape of hole from a Swiss, where the
        // pairings that would have existed simply never happen.
        (
            Plan {
                label: Some("withdrawals"),
                // Two rounds in, so the leavers have a played record behind them
                // and a forfeited one after.
                withdraw: Some((2, 2)),
                ..Plan::new(SingleRoundRobin, 7, 0)
            },
            Stage::Finished,
        ),
    ]
}

/// One seeded tournament, as the end-to-end tests consume it.
///
/// Names and nanoids are freshly random on every run, so a browser test cannot
/// hardcode a URL. Writing them out as fixtures is what makes the suite
/// deterministic without giving up the uniqueness `tournaments.name` requires.
#[derive(Serialize)]
pub struct Seeded {
    pub name: String,
    pub nanoid: String,
    pub mode: String,
    pub stage: &'static str,
}

pub async fn run(
    database_url: Option<String>,
    play_moves: bool,
    manifest: Option<PathBuf>,
) -> Result<()> {
    let mut conn = setup_database(database_url).await?;
    let (organizer, players) = ensure_pool(&mut conn).await?;
    tracing::info!(players = players.len(), "seed pool ready");

    let mut seeded = Vec::new();
    // Fixed, so two runs produce the same results and anything odd in the UI can
    // be reproduced.
    let mut rng = SmallRng::seed_from_u64(0x4849_5645_5345_4544);

    for plan in plans() {
        for (index, stage) in [Stage::Finished, Stage::InProgress, Stage::NotStarted]
            .into_iter()
            .enumerate()
        {
            let waiting = match index {
                0 => Waiting::Manual,
                1 => Waiting::Scheduled,
                _ => Waiting::InviteOnly,
            };
            let record = seed_one(
                &plan, stage, waiting, &organizer, &players, play_moves, &mut rng, &mut conn,
            )
            .await
            .with_context(|| format!("seeding a {} {}", plan.mode, stage.label()))?;
            seeded.push(record);
        }
    }

    for (plan, stage) in scenarios() {
        let record = seed_one(
            &plan,
            stage,
            Waiting::Manual,
            &organizer,
            &players,
            play_moves,
            &mut rng,
            &mut conn,
        )
        .await
        .with_context(|| {
            format!(
                "seeding a {} {} ({})",
                plan.mode,
                stage.label(),
                plan.label.unwrap_or("scenario")
            )
        })?;
        seeded.push(record);
    }

    for stage in [Stage::InProgress, Stage::Finished] {
        let record = seed_arena(&organizer, &players, stage, play_moves, &mut rng, &mut conn)
            .await
            .with_context(|| format!("seeding an arena ({})", stage.label()))?;
        seeded.push(record);
    }

    if let Some(path) = manifest {
        write_manifest(&seeded, &path)?;
    }
    report(&seeded);
    Ok(())
}

fn write_manifest(seeded: &[Seeded], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(seeded).context("could not encode the manifest")?;
    std::fs::write(path, json).with_context(|| format!("could not write {}", path.display()))?;
    tracing::info!(path = %path.display(), "wrote the fixture manifest");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn seed_one(
    plan: &Plan,
    stage: Stage,
    waiting: Waiting,
    organizer: &User,
    pool: &[User],
    play_moves: bool,
    rng: &mut SmallRng,
    conn: &mut DbConn<'_>,
) -> Result<Seeded> {
    let field = &pool[..plan.players];
    let invite_only = stage == Stage::NotStarted && waiting == Waiting::InviteOnly;
    let starts_at = (stage == Stage::NotStarted && waiting == Waiting::Scheduled)
        .then(|| Utc::now() + Duration::hours(6));

    let label = plan
        .label
        .map(|label| format!(" ({label})"))
        .unwrap_or_default();
    let details = TournamentDetails {
        name: format!(
            "{}{label} {} {}",
            plan.mode,
            stage.label(),
            nanoid::nanoid!(5)
        ),
        description: format!(
            "Seeded {}{label} with {} players, left {}. Organizer is {ORGANIZER}.",
            plan.mode,
            plan.players,
            stage.label()
        ),
        scoring: ScoringMode::Game,
        tiebreakers: tiebreakers_for(plan.mode),
        invitees: Vec::new(),
        seats: plan.players as i32,
        min_seats: plan.players as i32,
        rounds: plan.rounds,
        invite_only,
        mode: plan.mode.to_string(),
        time_mode: TimeMode::RealTime,
        time_base: Some(TIME_BASE),
        time_increment: Some(0),
        band_upper: None,
        band_lower: None,
        start_mode: if starts_at.is_some() {
            StartMode::Date
        } else {
            StartMode::Manual
        },
        starts_at,
        round_duration: None,
        series: None,
        fully_automated: false,
        third_place_match: plan.mode == TournamentMode::SingleElimination,
        arena_duration_seconds: None,
        points: PointSystemDetails::default(),
    };

    let new_tournament = NewTournament::new(details).context("invalid tournament details")?;
    let mut tournament = Tournament::create(organizer.id, &new_tournament, conn)
        .await
        .context("could not insert the tournament")?;

    // An invite-only tournament is left with the field invited and unanswered,
    // which is the state the organizer panel's pending list is for.
    if invite_only {
        for player in field {
            tournament = tournament
                .create_invitation(&organizer.id, &player.id, conn)
                .await
                .context("could not invite a player")?;
        }
    } else {
        for player in field {
            tournament = tournament
                .join(&player.id, conn)
                .await
                .context("could not join a player")?;
        }
    }

    if stage == Stage::NotStarted {
        return Ok(record(&tournament, stage));
    }

    let (started, _, _) = tournament
        .start_by_organizer(&organizer.id, conn)
        .await
        .context("could not start the tournament")?;
    tournament = started;

    let decide = decider(plan.mode, field, rng.random());
    play(
        &tournament,
        organizer,
        field,
        plan,
        stage,
        &decide,
        play_moves,
        conn,
    )
    .await?;

    if stage == Stage::Finished {
        // A withdrawal forfeits the leaver's remaining games, so by here every
        // game has a result even though fewer were actually played.
        tournament = tournament
            .finish(&organizer.id, conn)
            .await
            .context("could not finish the tournament")?;
    }

    Ok(record(&tournament, stage))
}

/// Arena ranks itself, and elimination ranks by rounds survived, so neither
/// takes a configurable tiebreaker — `NewTournament::new` rejects a non-empty
/// list for them.
fn tiebreakers_for(mode: TournamentMode) -> Vec<Option<Tiebreaker>> {
    if mode.is_arena() || mode.is_elimination() {
        return Vec::new();
    }
    vec![
        Some(Tiebreaker::RawPoints),
        Some(Tiebreaker::Buchholz),
        Some(Tiebreaker::HeadToHead),
    ]
}

/// How a game's result is chosen.
///
/// Two-game-match modes get a scripted better-seed-wins rule: a 1-1 match is
/// *unresolved* rather than drawn and triggers a replay, so random per-game
/// results split 1-1 often enough to spin. Single-game modes take random
/// results, draws included, which is what makes the standings interesting.
fn decider<'a>(
    mode: TournamentMode,
    field: &'a [User],
    entropy: u64,
) -> Box<dyn Fn(&Game) -> TournamentGameResult + 'a> {
    let seed_of = move |player: Uuid| {
        field
            .iter()
            .position(|user| user.id == player)
            .unwrap_or(usize::MAX)
    };

    if mode.is_two_game_match() {
        return Box::new(move |game: &Game| {
            if seed_of(game.white_id) < seed_of(game.black_id) {
                TournamentGameResult::Winner(Color::White)
            } else {
                TournamentGameResult::Winner(Color::Black)
            }
        });
    }

    Box::new(move |game: &Game| {
        // Derived from the game's own id so a result is stable across replays of
        // the same seeding run.
        let mut rng = SmallRng::seed_from_u64(entropy ^ game.id.as_u128() as u64);
        match rng.random_range(0..10) {
            0 | 1 => TournamentGameResult::Draw,
            n if n % 2 == 0 => TournamentGameResult::Winner(Color::White),
            _ => TournamentGameResult::Winner(Color::Black),
        }
    })
}

async fn has_unfinished(tournament: &Tournament, conn: &mut DbConn<'_>) -> Result<bool> {
    Ok(tournament
        .games(conn)
        .await
        .context("could not load tournament games")?
        .iter()
        .any(|game| !game.finished))
}

/// Decides every unfinished game up to and including `round`.
///
/// The round bound matters for a round robin, which creates its whole schedule
/// when it starts: playing every unfinished game would finish the tournament on
/// the first pass, so nothing that happens "after round two" could ever be
/// staged. Swiss and elimination only have the current round outstanding, so the
/// bound is a no-op there.
async fn adjudicate_unfinished(
    tournament: &Tournament,
    organizer: &User,
    decide: &dyn Fn(&Game) -> TournamentGameResult,
    round: i32,
    play_moves: bool,
    conn: &mut DbConn<'_>,
) -> Result<usize> {
    let games = tournament
        .games(conn)
        .await
        .context("could not load tournament games")?;

    let mut played = 0;
    for game in games
        .iter()
        .filter(|game| !game.finished && game.round.is_none_or(|at| at <= round))
    {
        finish_game(game, organizer, &decide(game), play_moves, conn).await?;
        played += 1;
    }
    Ok(played)
}

/// Ends a game the way players actually end one.
///
/// Adjudication writes `Conclusion::Committee`, which is what an organizer
/// overriding a result looks like — fine for a forfeit, misleading for the whole
/// field. A decisive game is resigned by the loser and a drawn one is agreed, so
/// the conclusions read like a real tournament. Adjudication stays as the
/// fallback, since a game that never started cannot be resigned.
async fn finish_game(
    game: &Game,
    organizer: &User,
    result: &TournamentGameResult,
    play_moves: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    // Resigning and agreeing a draw both need a started game.
    let started = if play_moves {
        moves::play_some(game, conn).await?;
        true
    } else {
        game.start(conn).await.is_ok()
    };

    if started {
        match result {
            TournamentGameResult::Winner(winner) => {
                let resign = GameControl::Resign(winner.opposite_color());
                if game.resign(&resign, conn).await.is_ok() {
                    return Ok(());
                }
            }
            TournamentGameResult::Draw => {
                // A draw is agreed, not declared: `accept_draw` checks the last
                // control was the opponent's offer, so one has to be made first.
                //
                // Re-read before offering. `write_game_control` matches on the
                // row's `turn`, `history` and control history, so the copy loaded
                // before the moves were played no longer matches anything and the
                // write silently fails — which is why every draw used to end up
                // adjudicated. `resign` re-reads internally, so it never noticed.
                let fresh = Game::find_by_uuid(&game.id, conn)
                    .await
                    .context("could not re-read a game before agreeing a draw")?;
                let offer = GameControl::DrawOffer(Color::White);
                if fresh.write_game_control(&offer, conn).await.is_ok()
                    && fresh
                        .accept_draw(&GameControl::DrawAccept(Color::Black), conn)
                        .await
                        .is_ok()
                {
                    return Ok(());
                }
            }
            // A double forfeit has no player action behind it.
            _ => {}
        }
    }

    game.adjudicate_tournament_result(&organizer.id, result, conn)
        .await
        .context("could not adjudicate a game")?;
    Ok(())
}

/// Plays the tournament out, applying the plan's mid-event events between rounds.
///
/// One loop for both stages so a withdrawal lands at the same point whether the
/// tournament is then paused or run to the end — otherwise the paused and
/// finished copies of a scenario would not be the same tournament at different
/// times, which is the whole point of seeding both.
async fn play(
    tournament: &Tournament,
    organizer: &User,
    field: &[User],
    plan: &Plan,
    stage: Stage,
    decide: &dyn Fn(&Game) -> TournamentGameResult,
    play_moves: bool,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    let stop_after = (stage == Stage::InProgress).then_some(plan.rounds_before_pausing);

    for round in 0..512 {
        // Before the round is played, not after: the engine forfeits a leaver's
        // outstanding games, and adjudicating first would leave it nothing to
        // forfeit — which is exactly the path worth staging.
        if let Some((after, count)) = plan.withdraw {
            if round == after {
                // From the bottom of the field, so the standings above the
                // departures stay readable.
                for player in field.iter().rev().take(count) {
                    match tournament
                        .withdraw_player(&player.id, &organizer.id, conn)
                        .await
                    {
                        Ok(forfeited) => tracing::info!(
                            player = %player.username,
                            round,
                            forfeited,
                            "withdrew a player",
                        ),
                        Err(error) => tracing::warn!(%error, "could not withdraw a player"),
                    }
                }
            }
        }

        adjudicate_unfinished(
            tournament,
            organizer,
            decide,
            round as i32 + 1,
            play_moves,
            conn,
        )
        .await?;

        if plan.zero_point_bye == Some(round) {
            // The weakest player is the one plausibly asking to sit out.
            if let Some(player) = field.last() {
                match tournament
                    .grant_zero_point_bye(&player.id, &organizer.id, conn)
                    .await
                {
                    Ok(_) => {
                        tracing::info!(player = %player.username, round, "granted a requested bye")
                    }
                    Err(error) => tracing::warn!(%error, "could not grant a requested bye"),
                }
            }
        }

        match tournament.progress(conn).await {
            Ok(ProgressOutcome::ReadyToFinish) => return Ok(()),
            Ok(ProgressOutcome::Advanced(_) | ProgressOutcome::Replays(_)) => {}
            Ok(ProgressOutcome::Waiting) => {
                // Not necessarily a stall. A round robin has its whole schedule
                // from the start, so `progress` has nothing to add until the last
                // round is in — and a withdrawal can leave a round with nothing
                // left to pair. Only an idle tick with no games left is wrong.
                if stop_after.is_some() {
                    return Ok(());
                }
                if !has_unfinished(tournament, conn).await? {
                    anyhow::bail!("nothing to do and nothing left to play")
                }
            }
            Err(error) => {
                tracing::warn!(%error, "stopped advancing this tournament early");
                return Ok(());
            }
        }

        if stop_after.is_some_and(|limit| round + 1 >= limit) {
            return Ok(());
        }
    }
    anyhow::bail!("tournament did not finish within 512 rounds")
}

async fn seed_arena(
    organizer: &User,
    pool: &[User],
    stage: Stage,
    play_moves: bool,
    rng: &mut SmallRng,
    conn: &mut DbConn<'_>,
) -> Result<Seeded> {
    // Fewer players than seats, so there is room to walk in — which is the whole
    // point of the format and of the front-page card.
    let field = &pool[..12];

    let details = TournamentDetails {
        name: format!("Arena {} {}", stage.label(), nanoid::nanoid!(5)),
        description: format!(
            "Seeded arena left {}. Organizer is {ORGANIZER}.",
            stage.label()
        ),
        scoring: ScoringMode::Game,
        tiebreakers: Vec::new(),
        invitees: Vec::new(),
        seats: 24,
        min_seats: 1,
        rounds: 0,
        invite_only: false,
        mode: TournamentMode::Arena.to_string(),
        time_mode: TimeMode::RealTime,
        time_base: Some(TIME_BASE),
        time_increment: Some(0),
        band_upper: None,
        band_lower: None,
        start_mode: StartMode::Manual,
        starts_at: None,
        round_duration: None,
        series: None,
        fully_automated: false,
        third_place_match: false,
        arena_duration_seconds: Some(ARENA_SECONDS),
        points: PointSystemDetails::default(),
    };

    let new_tournament = NewTournament::new(details).context("invalid arena details")?;
    let mut tournament = Tournament::create(organizer.id, &new_tournament, conn)
        .await
        .context("could not insert the arena")?;

    for player in field {
        tournament = tournament
            .join(&player.id, conn)
            .await
            .context("could not join an arena player")?;
    }

    let (started, _, _) = tournament
        .start_by_organizer(&organizer.id, conn)
        .await
        .context("could not start the arena")?;
    tournament = started;

    let decide = decider(TournamentMode::Arena, field, rng.random());
    let ticks = if stage == Stage::Finished { 8 } else { 4 };

    // An arena is empty until its first tick pairs the waiting pool, so this
    // finishes whatever the previous tick created and then ticks again.
    for _ in 0..ticks {
        let finished = finish_open_games(&tournament, organizer, &decide, play_moves, conn).await?;
        if finished > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(ARENA_TICK_GAP_MS)).await;
        }
        if let Err(error) = tournament.progress(conn).await {
            tracing::warn!(%error, "stopped ticking the arena early");
            break;
        }
    }

    if stage == Stage::Finished {
        finish_open_games(&tournament, organizer, &decide, play_moves, conn).await?;
        tournament = tournament
            .finish(&organizer.id, conn)
            .await
            .context("could not finish the arena")?;
    }

    Ok(record(&tournament, stage))
}

/// Decides every open game in the arena, spacing their finishes apart.
async fn finish_open_games(
    tournament: &Tournament,
    organizer: &User,
    decide: &dyn Fn(&Game) -> TournamentGameResult,
    play_moves: bool,
    conn: &mut DbConn<'_>,
) -> Result<usize> {
    let games = tournament.games(conn).await.context("load arena games")?;
    let mut finished = 0;

    for game in games.iter().filter(|game| !game.finished) {
        if play_moves {
            moves::play_some(game, conn).await?;
        }
        let offset = ARENA_FINISH_SPACING_MS * (finished as i64 + 1);
        finish_arena_game(game, organizer, &decide(game), offset, conn).await?;
        finished += 1;
    }
    Ok(finished)
}

/// Adjudicates an arena game and then restamps when it finished.
///
/// `Event::Finish` is timed from `finished_at`, which adjudication sets to now —
/// so without this, games decided back to back share one millisecond and a
/// finish can collide with a pairing. Replay treats that as fatal and the arena
/// is unrecoverable, so the spacing is load-bearing rather than cosmetic.
///
/// Measured from the game's own `created_at` so a finish can never precede the
/// pairing that produced it. `updated_at` moves in step to keep the row coherent.
async fn finish_arena_game(
    game: &Game,
    organizer: &User,
    result: &TournamentGameResult,
    millis_in: i64,
    conn: &mut DbConn<'_>,
) -> Result<()> {
    game.adjudicate_tournament_result(&organizer.id, result, conn)
        .await
        .context("could not adjudicate an arena game")?;

    let at = game.created_at + Duration::milliseconds(millis_in);
    diesel::update(games::table.find(game.id))
        .set((games::finished_at.eq(at), games::updated_at.eq(at)))
        .execute(conn)
        .await
        .context("could not stamp an arena finish")?;
    Ok(())
}

fn record(tournament: &Tournament, stage: Stage) -> Seeded {
    tracing::info!(name = %tournament.name, stage = stage.label(), "seeded");
    Seeded {
        name: tournament.name.clone(),
        nanoid: tournament.nanoid.clone(),
        mode: tournament.mode.clone(),
        stage: stage.label(),
    }
}

fn report(seeded: &[Seeded]) {
    println!("\nSeeded {} tournaments.\n", seeded.len());
    println!("Log in as {ORGANIZER} for the organizer view, or tt-01 for a player's.");
    println!("Password for every seeded account: {PASSWORD}\n");
    for entry in seeded {
        println!(
            "  {:<18} {:<9} /tournament/{:<14} {}",
            entry.mode, entry.stage, entry.nanoid, entry.name
        );
    }
    println!();
}
