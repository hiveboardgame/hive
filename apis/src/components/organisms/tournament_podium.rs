use crate::{
    common::{
        display_standing_rows,
        primary_score_presentation,
        standings_count,
        standings_value_text,
        StandingsCount,
    },
    components::organisms::tournament_inspector::TournamentSelection,
    providers::{
        ArenaStateStoreFields,
        EliminationState,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatStore,
    },
    responses::{tournament::rating_clock, TournamentMemberships},
};
use leptos::prelude::*;
use reactive_stores::Store;
use shared_types::{
    tournament::{
        round_robin::PrimaryScore as RoundRobinPrimaryScore,
        standings::Row,
        swiss::{PrimaryScore as DoubleSwissPrimaryScore, System as SwissSystem},
        Format,
        FormatConfig,
    },
    GameSpeed,
};
use std::{collections::BTreeMap, fmt};
use uuid::Uuid;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum PlayerRating {
    Arena(i32),
    Standard(u64),
}

impl fmt::Display for PlayerRating {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arena(rating) => rating.fmt(formatter),
            Self::Standard(rating) => rating.fmt(formatter),
        }
    }
}

#[derive(Clone)]
struct PodiumPlayer {
    id: Uuid,
    name: String,
    rating: Option<PlayerRating>,
    seed: Option<usize>,
    score: Option<String>,
    performance: Option<i32>,
    wins: u32,
    draws: u32,
    losses: u32,
    berserks: u32,
}

#[derive(Clone)]
struct PodiumPlacement {
    rank: u32,
    players: Vec<PodiumPlayer>,
}

fn podium_record(configuration: &FormatConfig, row: &Row) -> (u32, u32, u32) {
    if matches!(configuration, FormatConfig::RoundRobin(config) if config.primary_score == RoundRobinPrimaryScore::MatchPoints)
    {
        return (
            standings_count(row, StandingsCount::RoundRobinMatchWins),
            standings_count(row, StandingsCount::RoundRobinMatchDraws),
            standings_count(row, StandingsCount::RoundRobinMatchLosses),
        );
    }
    let FormatConfig::Swiss(configuration) = configuration else {
        return (row.wins, row.draws, row.losses);
    };
    let SwissSystem::DoubleSwiss(configuration) = &configuration.system else {
        return (row.wins, row.draws, row.losses);
    };
    if configuration.primary_score != DoubleSwissPrimaryScore::MatchPoints {
        return (row.wins, row.draws, row.losses);
    }
    (
        standings_count(row, StandingsCount::SwissMatchWins),
        standings_count(row, StandingsCount::SwissMatchDraws),
        standings_count(row, StandingsCount::SwissMatchLosses),
    )
}

fn standing_podium(
    common: Store<TournamentCommon>,
    format: TournamentFormatStore,
    configuration: &FormatConfig,
) -> Vec<PodiumPlacement> {
    let memberships = common.memberships().get();
    let standings = common.standings().get();
    let speed = rating_clock(configuration).map(GameSpeed::from);
    let presentation = primary_score_presentation(configuration);
    let arena = match format {
        TournamentFormatStore::Arena(state) => Some(state),
        _ => None,
    };
    let mut players = Vec::new();
    for standing in standings
        .snapshot
        .iter()
        .flat_map(|standings| display_standing_rows(standings, &memberships.withdrawn))
    {
        let Some(rank) = standing.rank else {
            continue;
        };
        if rank > 3 {
            continue;
        }
        let row = &standing.row;
        let Some(user) = memberships.players.get(&row.user_id) else {
            continue;
        };
        let player_arena_stats = arena.and_then(|arena| {
            arena
                .player_stats()
                .with_untracked(|stats| stats.contains_key(&row.user_id))
                .then(|| arena.player_stats().at_key(row.user_id).get())
        });
        let (wins, draws, losses) = player_arena_stats.as_ref().map_or_else(
            || podium_record(configuration, row),
            |stats| (stats.wins, stats.draws, stats.losses),
        );
        players.push((
            rank,
            PodiumPlayer {
                id: user.uid,
                name: user.username.clone(),
                rating: player_arena_stats
                    .as_ref()
                    .and_then(|stats| stats.arena_rating)
                    .map(PlayerRating::Arena)
                    .or_else(|| {
                        arena
                            .is_none()
                            .then(|| {
                                speed.as_ref().map(|speed| {
                                    PlayerRating::Standard(user.rating_for_speed(speed))
                                })
                            })
                            .flatten()
                    }),
                seed: memberships
                    .pairing_numbers
                    .get(&row.user_id)
                    .map(|seed| seed + 1),
                score: Some(standings_value_text(row.primary_score, presentation)),
                performance: player_arena_stats
                    .as_ref()
                    .and_then(|stats| stats.performance_rating),
                wins,
                draws,
                losses,
                berserks: player_arena_stats
                    .as_ref()
                    .map_or(0, |stats| stats.berserks),
            },
        ));
    }

    group_podium_players(players)
}

fn group_podium_players(
    players: impl IntoIterator<Item = (u32, PodiumPlayer)>,
) -> Vec<PodiumPlacement> {
    let mut groups = BTreeMap::<u32, Vec<PodiumPlayer>>::new();
    for (rank, player) in players {
        if (1..=3).contains(&rank) {
            groups.entry(rank).or_default().push(player);
        }
    }
    groups
        .into_iter()
        .map(|(rank, players)| PodiumPlacement { rank, players })
        .collect()
}

fn elimination_player(
    memberships: &TournamentMemberships,
    player: Uuid,
    speed: Option<&GameSpeed>,
) -> Option<PodiumPlayer> {
    let user = memberships.players.get(&player)?;
    Some(PodiumPlayer {
        id: user.uid,
        name: user.username.clone(),
        rating: speed.map(|speed| PlayerRating::Standard(user.rating_for_speed(speed))),
        seed: memberships
            .pairing_numbers
            .get(&player)
            .map(|seed| seed + 1),
        score: None,
        performance: None,
        wins: 0,
        draws: 0,
        losses: 0,
        berserks: 0,
    })
}

fn elimination_podium(
    common: Store<TournamentCommon>,
    elimination: Store<EliminationState>,
    configuration: &FormatConfig,
) -> Vec<PodiumPlacement> {
    let memberships = common.memberships().get();
    let speed = rating_clock(configuration).map(GameSpeed::from);
    let player_results = elimination.player_results().get();
    group_podium_players(player_results.iter().filter_map(|result| {
        let rank = result.placement?;
        if memberships.withdrawn.contains(&result.player) {
            return None;
        }
        Some((
            rank,
            elimination_player(&memberships, result.player, speed.as_ref())?,
        ))
    }))
}

fn podium_placements(
    common: Store<TournamentCommon>,
    format: TournamentFormatStore,
) -> Vec<PodiumPlacement> {
    match format {
        TournamentFormatStore::Arena(state) => {
            let configuration = FormatConfig::Arena(state.configuration().get_untracked());
            standing_podium(common, format, &configuration)
        }
        TournamentFormatStore::RoundRobin(state) => {
            let configuration = FormatConfig::RoundRobin(state.configuration().get_untracked());
            standing_podium(common, format, &configuration)
        }
        TournamentFormatStore::Swiss(state) => {
            let configuration = FormatConfig::Swiss(state.configuration().get_untracked());
            standing_podium(common, format, &configuration)
        }
        TournamentFormatStore::Elimination(state) => {
            let configuration = FormatConfig::Elimination(state.configuration().get_untracked());
            elimination_podium(common, state, &configuration)
        }
    }
}

fn placement_label(rank: u32, shared: bool) -> &'static str {
    // TODO: i18n once copy is approved.
    match (rank, shared) {
        (1, false) => "First",
        (1, true) => "Joint first",
        (2, false) => "Second",
        (2, true) => "Joint second",
        (3, false) => "Third",
        (3, true) => "Joint third",
        _ => "Placed",
    }
}

fn player_detail(player: &PodiumPlayer, format: Format) -> String {
    // TODO: i18n once copy is approved.
    if matches!(
        format,
        Format::SingleElimination | Format::DoubleElimination
    ) {
        return player
            .seed
            .map(|seed| format!("Seed {seed}"))
            .unwrap_or_else(|| String::from("Seed —"));
    }
    let record = format!("{}W · {}D · {}L", player.wins, player.draws, player.losses);
    if format == Format::Arena {
        let performance = player
            .performance
            .map(|performance| format!("Perf {performance} · "))
            .unwrap_or_default();
        return if player.berserks == 0 {
            format!("{performance}{record}")
        } else {
            format!("{performance}{record} · {} Berserks", player.berserks)
        };
    }
    record
}

#[component]
pub fn TournamentPodium(
    common: Store<TournamentCommon>,
    format: TournamentFormatStore,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let placements =
        Memo::new_with_compare(move |_| podium_placements(common, format), |_, _| true);
    let format = format.format();
    view! {
        // TODO: i18n once copy is approved.
        <section
            class="p-3 border-b border-black/10 dark:border-white/10"
            aria-label="Tournament podium"
            data-testid="tournament-podium"
        >
            {move || {
                placements
                    .get()
                    .into_iter()
                    .map(|placement| {
                        let count = placement.players.len();
                        let expanded = RwSignal::new(false);
                        let rank = placement.rank;
                        view! {
                            <div
                                class="flex gap-3 items-start py-1.5 border-b last:border-0 border-black/5 dark:border-white/10"
                                data-podium-rank=rank
                            >
                                <span class="tournament-podium-place" data-rank=rank>
                                    {rank}
                                </span>
                                <div class="grid flex-1 gap-2 items-center min-w-0 grid-cols-[4rem_minmax(0,1fr)] min-h-8">
                                    <strong class="text-sm">
                                        {placement_label(rank, count > 1)}
                                    </strong>
                                    <div class="min-w-0">
                                        {(count > 6)
                                            .then(|| {
                                                view! {
                                                    // TODO: i18n once copy is approved.
                                                    <button
                                                        type="button"
                                                        class="text-sm ui-text-link"
                                                        on:click=move |_| expanded.update(|value| *value = !*value)
                                                    >
                                                        {move || {
                                                            if expanded.get() {
                                                                "Hide players".to_string()
                                                            } else {
                                                                format!("Show all {count} players")
                                                            }
                                                        }}
                                                    </button>
                                                }
                                            })}
                                        <div
                                            class="flex flex-wrap gap-2 min-w-0"
                                            class:hidden=move || { count > 6 && !expanded.get() }
                                        >
                                            {placement
                                                .players
                                                .into_iter()
                                                .map(|player| {
                                                    let player_id = player.id;
                                                    let detail = player_detail(&player, format);
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="min-w-0 max-w-full ui-button ui-button-secondary ui-button-sm"
                                                            title=detail
                                                            data-podium-player-id=player_id.to_string()
                                                            on:click=move |_| {
                                                                selection.set(Some(TournamentSelection::Player(player_id)))
                                                            }
                                                        >
                                                            <span class="truncate">{player.name}</span>
                                                            {player
                                                                .score
                                                                .map(|score| {
                                                                    view! {
                                                                        <strong class="tabular-nums shrink-0">" · "{score}</strong>
                                                                    }
                                                                })}
                                                            {player
                                                                .rating
                                                                .map(|rating| {
                                                                    view! {
                                                                        <span class="text-xs italic text-gray-500 dark:text-gray-400 shrink-0">
                                                                            {rating.to_string()}
                                                                        </span>
                                                                    }
                                                                })}
                                                        </button>
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn podium_preserves_official_places_and_every_player_at_those_places() {
        for ranks in [
            vec![1, 1, 3],
            vec![1, 2, 3, 3],
            vec![1, 1, 1, 1],
            vec![1, 2, 3, 4],
        ] {
            let placements = group_podium_players(ranks.iter().enumerate().map(|(index, rank)| {
                (
                    *rank,
                    PodiumPlayer {
                        id: Uuid::from_u128(index as u128),
                        name: index.to_string(),
                        rating: None,
                        seed: None,
                        score: Some("10".to_string()),
                        performance: None,
                        wins: 0,
                        draws: 0,
                        losses: 0,
                        berserks: 0,
                    },
                )
            }));
            let actual = placements
                .iter()
                .flat_map(|placement| {
                    placement
                        .players
                        .iter()
                        .map(move |player| (placement.rank, player.id))
                })
                .collect::<Vec<_>>();
            let expected = ranks
                .into_iter()
                .enumerate()
                .filter(|(_, rank)| *rank <= 3)
                .map(|(index, rank)| (rank, Uuid::from_u128(index as u128)))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }
}
