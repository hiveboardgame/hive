#![allow(dead_code)]

use db_lib::{
    models::{Game, NewTournament, NewUser, ProgressOutcome, Tournament, TournamentBye, User},
    schema::{ratings, tournaments_users},
    DbConn,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use hive_lib::Color;
use shared_types::{
    GameSpeed,
    PlayerScores,
    ScoringMode,
    Standings,
    StartMode,
    Tiebreaker,
    TimeMode,
    TournamentDetails,
    TournamentGameResult,
    TournamentMode,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Every tournament fixture is real time, 60+0 — which is `GameSpeed::Bullet`,
/// the speed seeding reads ratings from. It also has to be real time for
/// `adjudicate_tournament_result` to accept a result: correspondence games are
/// created already in progress.
pub const TIME_BASE: i32 = 60;

pub struct Fixture {
    pub organizer: User,
    /// In creation order, which is also descending rating, which is also seed
    /// order — `players[0]` is seed 0.
    pub players: Vec<User>,
    pub tournament: Tournament,
}

impl Fixture {
    pub fn seed_of(&self, player: Uuid) -> usize {
        self.players
            .iter()
            .position(|user| user.id == player)
            .expect("player belongs to this tournament")
    }

    pub fn name_of(&self, player: Uuid) -> &str {
        &self.players[self.seed_of(player)].username
    }

    /// Seeds in finishing order, best first, grouped by tie.
    pub fn seed_groups(&self, standings: &Standings) -> Vec<Vec<usize>> {
        standings
            .ordered_groups()
            .iter()
            .map(|group| {
                let mut seeds: Vec<usize> =
                    group.iter().map(|player| self.seed_of(*player)).collect();
                seeds.sort_unstable();
                seeds
            })
            .collect()
    }
}

pub async fn create_user(username: &str, rating: f64, conn: &mut DbConn<'_>) -> User {
    let new_user = NewUser::new(username, "password", &format!("{username}@example.com"))
        .expect("build user fixture");
    let user = User::create(new_user, conn).await.expect("insert user");

    // Users are created at a flat 1500, which would leave seeding to fall back
    // on uuid order — random, so no test could predict a seed.
    diesel::update(
        ratings::table.filter(
            ratings::user_uid
                .eq(user.id)
                .and(ratings::speed.eq(GameSpeed::Bullet.to_string())),
        ),
    )
    .set(ratings::rating.eq(rating))
    .execute(conn)
    .await
    .expect("set fixture rating");

    user
}

pub struct TournamentOpts {
    pub player_count: usize,
    pub rounds: i32,
    pub tiebreakers: Vec<Tiebreaker>,
    pub scoring: ScoringMode,
    pub third_place_match: bool,
    pub fully_automated: bool,
    pub arena_duration_seconds: Option<i32>,
    /// Defaults to the starting field size. An arena wants room to spare, so
    /// late joiners have a seat to take.
    pub seats: Option<i32>,
    pub points: shared_types::PointSystemDetails,
    pub invite_only: bool,
}

impl Default for TournamentOpts {
    fn default() -> Self {
        Self {
            player_count: 4,
            rounds: 0,
            tiebreakers: vec![Tiebreaker::RawPoints],
            scoring: ScoringMode::Game,
            third_place_match: false,
            fully_automated: false,
            arena_duration_seconds: None,
            seats: None,
            points: shared_types::PointSystemDetails::default(),
            invite_only: false,
        }
    }
}

/// The details a fixture would build, without creating anything — so a test
/// can check that invalid details are refused.
pub fn details_for(mode: TournamentMode, opts: TournamentOpts) -> TournamentDetails {
    TournamentDetails {
        name: format!("{mode} {}", nanoid::nanoid!(6).to_lowercase()),
        description: String::new(),
        scoring: opts.scoring,
        tiebreakers: opts.tiebreakers.into_iter().map(Some).collect(),
        invitees: Vec::new(),
        seats: opts.seats.unwrap_or(opts.player_count as i32),
        min_seats: opts.player_count as i32,
        rounds: opts.rounds,
        invite_only: opts.invite_only,
        mode: mode.to_string(),
        time_mode: TimeMode::RealTime,
        time_base: Some(TIME_BASE),
        time_increment: Some(0),
        band_upper: None,
        band_lower: None,
        start_mode: StartMode::Manual,
        starts_at: None,
        round_duration: None,
        series: None,
        fully_automated: opts.fully_automated,
        third_place_match: opts.third_place_match,
        arena_duration_seconds: opts.arena_duration_seconds,
        points: opts.points,
    }
}

/// Creates the organizer and the field, joins everyone, and leaves the
/// tournament ready to start. Ratings descend by 10 from 2000 so seed order is
/// exactly creation order.
pub async fn create_tournament(
    mode: TournamentMode,
    opts: TournamentOpts,
    conn: &mut DbConn<'_>,
) -> Fixture {
    let suffix = nanoid::nanoid!(6).to_lowercase();
    let organizer = create_user(&format!("org_{suffix}"), 2500.0, conn).await;

    let mut players = Vec::with_capacity(opts.player_count);
    for index in 0..opts.player_count {
        players.push(
            create_user(
                &format!("p{index}_{suffix}"),
                2000.0 - (index as f64) * 10.0,
                conn,
            )
            .await,
        );
    }

    let details = details_for(mode, opts);
    let new_tournament = NewTournament::new(details).expect("valid tournament details");
    let tournament = Tournament::create(organizer.id, &new_tournament, conn)
        .await
        .expect("insert tournament");

    for player in &players {
        tournament
            .join(&player.id, conn)
            .await
            .expect("join tournament");
    }

    Fixture {
        organizer,
        players,
        tournament,
    }
}

pub async fn start(fixture: &mut Fixture, conn: &mut DbConn<'_>) -> Vec<Game> {
    let (tournament, games, _) = fixture
        .tournament
        .start_by_organizer(&fixture.organizer.id, conn)
        .await
        .expect("start tournament");
    fixture.tournament = tournament;
    games
}

pub async fn games_of(fixture: &Fixture, conn: &mut DbConn<'_>) -> Vec<Game> {
    fixture
        .tournament
        .games(conn)
        .await
        .expect("load tournament games")
}

pub async fn byes_of(fixture: &Fixture, conn: &mut DbConn<'_>) -> Vec<TournamentBye> {
    TournamentBye::for_tournament(fixture.tournament.id, conn)
        .await
        .expect("load tournament byes")
}

pub async fn standings_of(fixture: &Fixture, conn: &mut DbConn<'_>) -> Standings {
    fixture
        .tournament
        .standings(conn)
        .await
        .expect("compute standings")
}

/// The scripted result used by most tests: the better-seeded player always
/// wins, which makes the whole final table hand-computable.
pub fn lower_seed_wins(fixture: &Fixture) -> impl Fn(&Game) -> TournamentGameResult + '_ {
    move |game: &Game| {
        if fixture.seed_of(game.white_id) < fixture.seed_of(game.black_id) {
            TournamentGameResult::Winner(Color::White)
        } else {
            TournamentGameResult::Winner(Color::Black)
        }
    }
}

/// Adjudicates every unfinished game, returning how many were played.
pub async fn play_unfinished(
    fixture: &Fixture,
    result: impl Fn(&Game) -> TournamentGameResult,
    conn: &mut DbConn<'_>,
) -> usize {
    let games = games_of(fixture, conn).await;
    let mut played = 0;
    for game in games.iter().filter(|game| !game.finished) {
        game.adjudicate_tournament_result(&fixture.organizer.id, &result(game), conn)
            .await
            .expect("adjudicate tournament game");
        played += 1;
    }
    played
}

/// Plays every game and advances until the tournament is ready to finish,
/// returning how many times it advanced a round.
pub async fn run_to_completion(
    fixture: &Fixture,
    result: impl Fn(&Game) -> TournamentGameResult,
    conn: &mut DbConn<'_>,
) -> usize {
    let mut advances = 0;
    for _ in 0..256 {
        play_unfinished(fixture, &result, conn).await;
        match fixture
            .tournament
            .progress(conn)
            .await
            .expect("progress tournament")
        {
            ProgressOutcome::ReadyToFinish => return advances,
            ProgressOutcome::Advanced(games) => {
                assert!(!games.is_empty(), "advancing must create games");
                advances += 1;
            }
            ProgressOutcome::Replays(games) => {
                assert!(!games.is_empty(), "a replay must create games");
            }
            ProgressOutcome::Waiting => {
                panic!("every game was just played, so nothing should be waiting")
            }
        }
    }
    panic!("tournament did not finish within 256 rounds");
}

pub async fn seeds(fixture: &Fixture, conn: &mut DbConn<'_>) -> Vec<(Uuid, i32)> {
    tournaments_users::table
        .filter(tournaments_users::tournament_id.eq(fixture.tournament.id))
        .filter(tournaments_users::seed.is_not_null())
        .select((tournaments_users::user_id, tournaments_users::seed))
        .order(tournaments_users::seed.asc())
        .get_results::<(Uuid, Option<i32>)>(conn)
        .await
        .expect("load seeds")
        .into_iter()
        .map(|(user, seed)| (user, seed.expect("filtered to non-null")))
        .collect()
}

pub fn unordered(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

pub type PairCounts = HashMap<(Uuid, Uuid), usize>;

/// How many times each unordered pair met, and how often the first of the pair
/// had white.
pub fn meetings(games: &[Game]) -> (PairCounts, PairCounts) {
    let mut met: HashMap<(Uuid, Uuid), usize> = HashMap::new();
    let mut whites: HashMap<(Uuid, Uuid), usize> = HashMap::new();
    for game in games {
        let pair = unordered(game.white_id, game.black_id);
        *met.entry(pair).or_default() += 1;
        if game.white_id == pair.0 {
            *whites.entry(pair).or_default() += 1;
        }
    }
    (met, whites)
}

pub fn score(standings: &Standings, player: Uuid, tiebreaker: Tiebreaker) -> f32 {
    standings
        .score(player, tiebreaker)
        .unwrap_or_else(|| panic!("{tiebreaker} is missing from the standings"))
}

/// Every player appears exactly once, groups are ordered by non-increasing
/// primary score, and positions are competition ranking.
pub fn assert_well_formed(standings: &Standings, expected_players: usize) {
    let total: usize = standings.groups.iter().map(Vec::len).sum();
    assert_eq!(
        total, expected_players,
        "every player must appear in exactly one tie group"
    );

    let mut seen = std::collections::HashSet::new();
    for group in &standings.groups {
        assert!(!group.is_empty(), "no empty tie groups");
        for standing in group {
            assert!(
                seen.insert(standing.player),
                "a player appeared in two tie groups"
            );
        }
    }

    // Not `.first()`: diesel's prelude is in scope and its `FirstDsl` wins.
    let primary = standings.tiebreakers[0];
    let mut previous = f32::INFINITY;
    let mut expected_position = 1;
    for group in &standings.groups {
        let scores: Vec<f32> = group
            .iter()
            .map(|standing| standing.scores.get(&primary).copied().unwrap_or(0.0))
            .collect();
        let first = scores[0];
        assert!(
            scores.iter().all(|value| *value == first),
            "everyone in a tie group shares the primary score"
        );
        assert!(
            first <= previous,
            "tie groups must be ordered by non-increasing {primary}"
        );
        previous = first;

        for standing in group {
            assert_eq!(
                standing.position, expected_position,
                "tied players share the group's position"
            );
        }
        expected_position += group.len() as u32;
    }
}

/// With `win` worth 1 and a draw worth half each, every game hands out exactly
/// one point, so the field's total is fixed by the number of games played.
pub fn assert_points_conserved(standings: &Standings, decisive_games: usize) {
    let total: f32 = standings
        .players()
        .map(|standing| {
            standing
                .scores
                .get(&Tiebreaker::RawPoints)
                .copied()
                .unwrap_or(0.0)
        })
        .sum();
    assert!(
        (total - decisive_games as f32).abs() < 1e-4,
        "total points {total} should equal the {decisive_games} games played"
    );
}

pub fn scores_of(standings: &Standings, player: Uuid) -> PlayerScores {
    standings
        .players()
        .find(|standing| standing.player == player)
        .map(|standing| standing.scores.clone())
        .expect("player is in the standings")
}

pub async fn finish(fixture: &Fixture, conn: &mut DbConn<'_>) -> Tournament {
    fixture
        .tournament
        .finish(&fixture.organizer.id, conn)
        .await
        .expect("finish tournament")
}

pub fn assert_outcome_is_ready(outcome: &ProgressOutcome) {
    assert!(
        matches!(outcome, ProgressOutcome::ReadyToFinish),
        "expected the tournament to be ready to finish, got {outcome:?}"
    );
}
