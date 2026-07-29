use crate::{
    db_error::DbError,
    models::{
        tournament_engine::{Engine, EngineFormat, Field, Replay, POINT_SCALE},
        Game,
        Tournament,
    },
    DbConn,
};
use hive_lib::Color as HiveColor;
use shared_types::{
    PlayerScores,
    PlayerStanding,
    ScoringMode,
    Standings,
    Tiebreaker,
    TournamentGameResult,
    TournamentStatus,
};
use std::{cmp::Ordering, collections::HashMap, str::FromStr};
use tournamint::{swiss::PlayerStandingMetrics, RoundIndex};
use uuid::Uuid;

/// Ranking has to start somewhere, so the primary score always leads whatever
/// order the organizer configured.
fn primary_for(format: EngineFormat) -> Tiebreaker {
    if matches!(
        format,
        EngineFormat::SingleElimination { .. } | EngineFormat::DoubleElimination
    ) {
        return Tiebreaker::RoundsSurvived;
    }
    Tiebreaker::RawPoints
}

fn configured_tiebreakers(tournament: &Tournament, format: EngineFormat) -> Vec<Tiebreaker> {
    let mut order = vec![primary_for(format)];
    for stored in tournament.tiebreaker.iter().flatten() {
        match Tiebreaker::from_str(stored) {
            Ok(tiebreaker) if !order.contains(&tiebreaker) => order.push(tiebreaker),
            Ok(_) => {}
            Err(_) => {
                tracing::warn!(
                    tournament = %tournament.nanoid,
                    tiebreaker = %stored,
                    "ignoring unknown tiebreaker stored on tournament"
                );
            }
        }
    }
    order
}

fn scaled(value: f64) -> f32 {
    (value / POINT_SCALE as f64) as f32
}

/// The engine reports every metric it knows; which of them actually order the
/// table is the tournament's choice.
fn metric_scores(
    metrics: &PlayerStandingMetrics,
    scoring: ScoringMode,
    tiebreakers: &[Tiebreaker],
) -> PlayerScores {
    let primary = match scoring {
        ScoringMode::Match => metrics.match_points as f64,
        ScoringMode::Game => metrics.game_points,
    };
    let mut scores = PlayerScores::new();
    for tiebreaker in tiebreakers {
        let value = match tiebreaker {
            Tiebreaker::RawPoints => Some(scaled(primary)),
            Tiebreaker::MatchPoints => Some(scaled(metrics.match_points as f64)),
            Tiebreaker::GamePoints => Some(scaled(metrics.game_points)),
            Tiebreaker::Buchholz => Some(scaled(metrics.buchholz)),
            Tiebreaker::BuchholzCut1 => Some(scaled(metrics.buchholz_cut_1)),
            Tiebreaker::BuchholzCut2 => Some(scaled(metrics.buchholz_cut_2)),
            Tiebreaker::BuchholzMedian => Some(scaled(metrics.buchholz_median)),
            Tiebreaker::BuchholzBuchholz => Some(scaled(metrics.buchholz_buchholz)),
            Tiebreaker::Koya => Some(scaled(metrics.koya)),
            Tiebreaker::SonnebornBerger => Some(scaled(metrics.sonneborn_berger)),
            Tiebreaker::ProgressiveScore => Some(scaled(metrics.progressive_score)),
            Tiebreaker::Wins => Some(metrics.wins as f32),
            // Filled in from the game rows; the engine has no equivalent.
            Tiebreaker::HeadToHead | Tiebreaker::WinsAsBlack => Some(0.0),
            // Only an arena or a bracket produces these.
            Tiebreaker::RoundsSurvived
            | Tiebreaker::GamesPlayed
            | Tiebreaker::Draws
            | Tiebreaker::Losses
            | Tiebreaker::CurrentStreak
            | Tiebreaker::BestStreak
            | Tiebreaker::Berserks => None,
        };
        if let Some(value) = value {
            scores.insert(*tiebreaker, value);
        }
    }
    scores
}

fn wins_as_black(games: &[Game], player: Uuid) -> f32 {
    games
        .iter()
        .filter(|game| game.finished && game.black_id == player)
        .filter(|game| {
            matches!(
                TournamentGameResult::from_str(&game.tournament_game_result),
                Ok(TournamentGameResult::Winner(HiveColor::Black))
            )
        })
        .count() as f32
}

/// Points one player took off another across every game they played. Only
/// meaningful for a two-way tie, which is the only place it is applied.
fn head_to_head(games: &[Game], one: Uuid, two: Uuid) -> (f32, f32) {
    let mut scores = (0.0, 0.0);
    for game in games.iter().filter(|game| game.finished) {
        let pair = (game.white_id, game.black_id);
        if pair != (one, two) && pair != (two, one) {
            continue;
        }
        match TournamentGameResult::from_str(&game.tournament_game_result) {
            Ok(TournamentGameResult::Draw) => {
                scores.0 += 0.5;
                scores.1 += 0.5;
            }
            Ok(TournamentGameResult::Winner(color)) => {
                let winner = match color {
                    HiveColor::White => game.white_id,
                    HiveColor::Black => game.black_id,
                };
                if winner == one {
                    scores.0 += 1.0;
                } else {
                    scores.1 += 1.0;
                }
            }
            _ => {}
        }
    }
    scores
}

fn games_played(games: &[Game], player: Uuid) -> i32 {
    games
        .iter()
        .filter(|game| game.finished && (game.white_id == player || game.black_id == player))
        .count() as i32
}

/// Groups players who are equal on every tiebreaker, best first. Ties inside a
/// group stay in seed order so the output never depends on hash iteration.
fn rank(
    mut entries: Vec<(Uuid, i32, PlayerScores)>,
    tiebreakers: &[Tiebreaker],
    games: &[Game],
    seed_of: &HashMap<Uuid, i32>,
) -> Vec<Vec<PlayerStanding>> {
    let key = |scores: &PlayerScores| -> Vec<f32> {
        tiebreakers
            .iter()
            .filter(|tiebreaker| **tiebreaker != Tiebreaker::HeadToHead)
            .map(|tiebreaker| scores.get(tiebreaker).copied().unwrap_or(0.0))
            .collect()
    };

    entries.sort_by(|(left_id, _, left), (right_id, _, right)| {
        key(right)
            .partial_cmp(&key(left))
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                seed_of
                    .get(left_id)
                    .unwrap_or(&0)
                    .cmp(seed_of.get(right_id).unwrap_or(&0))
            })
    });

    let mut groups: Vec<Vec<(Uuid, i32, PlayerScores)>> = Vec::new();
    for entry in entries {
        match groups.last() {
            Some(group) if key(&group[0].2) == key(&entry.2) => groups
                .last_mut()
                .expect("just matched a last group")
                .push(entry),
            _ => groups.push(vec![entry]),
        }
    }

    // Direct encounter splits a pair, but says nothing useful about three or
    // more players, so it is only applied to groups of exactly two.
    if tiebreakers.contains(&Tiebreaker::HeadToHead) {
        for group in groups.iter_mut().filter(|group| group.len() == 2) {
            let (one, two) = (group[0].0, group[1].0);
            let (first, second) = head_to_head(games, one, two);
            group[0].2.insert(Tiebreaker::HeadToHead, first);
            group[1].2.insert(Tiebreaker::HeadToHead, second);
            if first < second {
                group.swap(0, 1);
            }
        }
        let split: Vec<Vec<(Uuid, i32, PlayerScores)>> = groups
            .into_iter()
            .flat_map(|group| {
                if group.len() == 2 {
                    let first = group[0].2.get(&Tiebreaker::HeadToHead).copied();
                    let second = group[1].2.get(&Tiebreaker::HeadToHead).copied();
                    if first != second {
                        return group.into_iter().map(|entry| vec![entry]).collect();
                    }
                }
                vec![group]
            })
            .collect();
        groups = split;
    }

    let mut position = 1;
    groups
        .into_iter()
        .map(|group| {
            let at = position;
            position += group.len() as u32;
            group
                .into_iter()
                .map(|(player, played, scores)| PlayerStanding {
                    player,
                    position: at,
                    games_played: played,
                    scores,
                })
                .collect()
        })
        .collect()
}

impl Tournament {
    /// The standings as they stand: recomputed from the games every time, so
    /// they are correct mid-event as well as at the end.
    pub async fn standings(&self, conn: &mut DbConn<'_>) -> Result<Standings, DbError> {
        if self.status == TournamentStatus::NotStarted.to_string() {
            return Ok(Standings::default());
        }

        if self.mode()?.is_arena() {
            return self.arena_standings(conn).await;
        }

        let replay = self.replay(conn).await?;
        let games = &replay.games;
        let tiebreakers = configured_tiebreakers(self, replay.format);
        let seed_of: HashMap<Uuid, i32> = replay
            .field
            .players
            .iter()
            .map(|player| (player.user_id, player.seed))
            .collect();

        let groups = match &replay.engine {
            Engine::Scored(engine) => {
                let scoring = self.scoring_mode();
                let metrics = engine.standing_metrics();
                let entries = replay
                    .field
                    .players
                    .iter()
                    .map(|player| {
                        let metric = metrics
                            .iter()
                            .find(|metric| metric.player_id.index() == player.seed as usize)
                            .ok_or_else(|| DbError::InvalidAction {
                                info: format!("no standings for seed {}", player.seed),
                            })?;
                        let mut scores = metric_scores(metric, scoring, &tiebreakers);
                        if scores.contains_key(&Tiebreaker::WinsAsBlack) {
                            scores.insert(
                                Tiebreaker::WinsAsBlack,
                                wins_as_black(&games, player.user_id),
                            );
                        }
                        Ok((player.user_id, games_played(&games, player.user_id), scores))
                    })
                    .collect::<Result<Vec<_>, DbError>>()?;
                rank(entries, &tiebreakers, &games, &seed_of)
            }
            Engine::Single(bracket) => bracket_groups(
                &replay,
                &games,
                bracket.champion(),
                bracket.runner_up(),
                bracket.third_place(),
                |player| bracket.eliminated_in_round(player),
            )?,
            Engine::Double(bracket) => bracket_groups(
                &replay,
                &games,
                bracket.champion(),
                bracket.runner_up(),
                bracket.third_place(),
                |player| bracket.eliminated_in_round(player),
            )?,
        };

        Ok(Standings {
            tiebreakers,
            groups,
        })
    }
}

/// A bracket has no points — how far you got *is* the result. The podium is
/// exact, and everyone else ties with the others knocked out in their round.
fn bracket_groups(
    replay: &Replay,
    games: &[Game],
    champion: Option<tournamint::PlayerId>,
    runner_up: Option<tournamint::PlayerId>,
    third_place: Option<tournamint::PlayerId>,
    eliminated_in_round: impl Fn(tournamint::PlayerId) -> Option<RoundIndex>,
) -> Result<Vec<Vec<PlayerStanding>>, DbError> {
    let field: &Field = &replay.field;
    let podium: Vec<Uuid> = [champion, runner_up, third_place]
        .into_iter()
        .flatten()
        .map(|player| field.user_id(player))
        .collect::<Result<Vec<_>, DbError>>()?;

    // The podium is exact; everyone else ties with whoever went out in the
    // same round. `eliminated_in_round` cannot be compared against the podium
    // directly — a bronze-match loser's entry is overwritten with the final
    // round — so the podium is resolved first and excluded here.
    let mut tiers: Vec<Vec<Uuid>> = podium.iter().map(|player| vec![*player]).collect();

    let mut remaining: Vec<(i32, Uuid)> = Vec::new();
    for player in &field.players {
        if podium.contains(&player.user_id) {
            continue;
        }
        // No elimination round means nobody has knocked them out: they are
        // still in the bracket, so they have outlasted everyone who has gone
        // out. Reading that as round zero would rank the players still playing
        // below every player already eliminated.
        let round = eliminated_in_round(field.player_id(player.user_id)?)
            .map_or(i32::MAX, |round| round.value() as i32);
        remaining.push((round, player.user_id));
    }
    remaining.sort_by(|(left_round, left), (right_round, right)| {
        right_round.cmp(left_round).then_with(|| left.cmp(right))
    });

    let mut index = 0;
    while index < remaining.len() {
        let round = remaining[index].0;
        let tier: Vec<Uuid> = remaining[index..]
            .iter()
            .take_while(|(other, _)| *other == round)
            .map(|(_, player)| *player)
            .collect();
        index += tier.len();
        tiers.push(tier);
    }

    // Rounds, not finishing tiers. Reading it off the tier index looks right
    // until a third-place match is enabled: the bronze pair adds a tier without
    // adding a round, and every player above them gains a phantom round — the
    // champion of a three-round bracket reporting four.
    //
    // The last round anybody was knocked out in is the final, so it is the
    // tournament's round count. A player knocked out in round `r` won the
    // `r - 1` rounds before it, and whoever is still in has survived them all.
    let final_round = field
        .players
        .iter()
        .filter_map(|player| {
            let id = field.player_id(player.user_id).ok()?;
            eliminated_in_round(id).map(|round| round.value() as i32)
        })
        .max()
        .unwrap_or(0);

    // The bronze match is played *in* the final round, so both its players
    // carry the final round even though the semi-final is what actually put
    // them out of the bracket. Only the runner-up genuinely lost in the final;
    // anyone else holding that round is a bronze player who went out one round
    // earlier. Reading it raw ranks fourth place above third.
    let runner_up = runner_up.map(|player| field.user_id(player)).transpose()?;
    let rounds_survived = |player: Uuid| -> Result<f32, DbError> {
        let id = field.player_id(player)?;
        let Some(round) = eliminated_in_round(id) else {
            return Ok(final_round as f32);
        };
        let round = round.value() as i32;
        let knocked_out = if round == final_round && Some(player) != runner_up {
            final_round - 1
        } else {
            round
        };
        Ok((knocked_out - 1) as f32)
    };

    let mut position = 1;
    tiers
        .into_iter()
        .map(|players| {
            let at = position;
            position += players.len() as u32;
            players
                .into_iter()
                .map(|player| {
                    Ok(PlayerStanding {
                        player,
                        position: at,
                        games_played: games_played(games, player),
                        scores: PlayerScores::from([(
                            Tiebreaker::RoundsSurvived,
                            rounds_survived(player)?,
                        )]),
                    })
                })
                .collect::<Result<Vec<_>, DbError>>()
        })
        .collect()
}
