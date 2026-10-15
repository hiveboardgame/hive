mod direct_encounter;

use self::direct_encounter::{direct_encounter_values, DirectEncounterCell, DirectEncounterValue};
use crate::{
    common::{
        display_standing_rows,
        primary_score_presentation,
        round_robin_criterion_presentation,
        standings_value_text,
        swiss_criterion_presentation,
        use_fitted_pagination,
        ScorePresentation,
    },
    components::{
        molecules::{
            pagination_controls::PaginationControls,
            panel::Panel,
            tournament_standings_controls::TournamentStandingsControls,
        },
        organisms::tournament_explanations::{
            round_robin_criterion_explanation,
            round_robin_criterion_name,
            swiss_criterion_explanation,
            swiss_criterion_name,
            StandingsCriteriaRules,
        },
    },
    i18n::*,
    providers::{
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatState,
        TournamentState,
    },
};
use leptos::{html, prelude::*};
use leptos_i18n::I18nContext;
use shared_types::{
    tournament::{
        round_robin::Criterion as RoundRobinCriterion,
        standings::Row,
        swiss::Criterion as SwissCriterion,
        FormatConfig,
    },
    GameSpeed,
};

const CELL: &str = "px-1 py-2 text-center text-sm whitespace-nowrap sm:px-2";
const HEAD: &str = "px-1 py-2 text-[0.65rem] font-bold uppercase whitespace-nowrap sm:px-2";

#[derive(Clone, Copy)]
struct DetailedStandingsColumn {
    index: usize,
    name: Signal<String>,
    explanation: Signal<String>,
    presentation: ScorePresentation,
    direct_encounter: bool,
}

#[derive(Clone)]
struct DetailedStandingsModel {
    primary_name: Signal<String>,
    primary_presentation: ScorePresentation,
    columns: Vec<DetailedStandingsColumn>,
    rules: FormatConfig,
}

#[derive(Clone, PartialEq)]
struct DetailedStandingsRow {
    placement: Option<u32>,
    username: String,
    withdrawn: bool,
    rating: String,
    total: String,
    values: Vec<(String, Option<DirectEncounterValue>)>,
}

fn detailed_standings_model(
    configuration: &FormatConfig,
    i18n: I18nContext<Locale, I18nKeys>,
) -> Option<DetailedStandingsModel> {
    let primary_presentation = primary_score_presentation(configuration);
    match configuration {
        FormatConfig::RoundRobin(configuration) => {
            let primary_configuration = configuration.clone();
            let primary_name = Signal::derive(move || {
                round_robin_criterion_name(
                    i18n,
                    &primary_configuration,
                    RoundRobinCriterion::PrimaryScore,
                )
            });
            let columns = configuration
                .rules
                .standings
                .criteria
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, criterion)| *criterion != RoundRobinCriterion::PrimaryScore)
                .map(|(index, criterion)| {
                    let name_configuration = configuration.clone();
                    let explanation_configuration = configuration.clone();
                    DetailedStandingsColumn {
                        index,
                        name: Signal::derive(move || {
                            round_robin_criterion_name(i18n, &name_configuration, criterion)
                        }),
                        explanation: Signal::derive(move || {
                            round_robin_criterion_explanation(
                                i18n,
                                &explanation_configuration,
                                criterion,
                            )
                        }),
                        presentation: round_robin_criterion_presentation(configuration, criterion),
                        direct_encounter: matches!(
                            criterion,
                            RoundRobinCriterion::DirectEncounter(_)
                        ),
                    }
                })
                .collect();
            Some(DetailedStandingsModel {
                primary_name,
                primary_presentation,
                columns,
                rules: FormatConfig::RoundRobin(configuration.clone()),
            })
        }
        FormatConfig::Swiss(configuration) => {
            let primary_configuration = configuration.clone();
            let primary_name = Signal::derive(move || {
                swiss_criterion_name(i18n, &primary_configuration, SwissCriterion::PrimaryScore)
            });
            let columns = configuration
                .standings
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, criterion)| *criterion != SwissCriterion::PrimaryScore)
                .map(|(index, criterion)| {
                    let name_configuration = configuration.clone();
                    let explanation_configuration = configuration.clone();
                    DetailedStandingsColumn {
                        index,
                        name: Signal::derive(move || {
                            swiss_criterion_name(i18n, &name_configuration, criterion)
                        }),
                        explanation: Signal::derive(move || {
                            swiss_criterion_explanation(i18n, &explanation_configuration, criterion)
                        }),
                        presentation: swiss_criterion_presentation(configuration, criterion),
                        direct_encounter: matches!(criterion, SwissCriterion::DirectEncounter(_)),
                    }
                })
                .collect();
            Some(DetailedStandingsModel {
                primary_name,
                primary_presentation,
                columns,
                rules: FormatConfig::Swiss(configuration.clone()),
            })
        }
        FormatConfig::Elimination(_) | FormatConfig::Arena(_) => None,
    }
}

fn value_at(row: &Row, column: DetailedStandingsColumn) -> String {
    row.values
        .get(column.index)
        .copied()
        .map(|value| standings_value_text(value, column.presentation))
        .unwrap_or_else(|| String::from("—"))
}

#[component]
pub fn TournamentDetailedStandings(tournament: TournamentState) -> impl IntoView {
    let i18n = use_i18n();
    let configuration = match tournament.format {
        TournamentFormatState::RoundRobin(state) => {
            FormatConfig::RoundRobin(state.configuration().get_untracked())
        }
        TournamentFormatState::Swiss(state) => {
            FormatConfig::Swiss(state.configuration().get_untracked())
        }
        TournamentFormatState::Arena(_) | TournamentFormatState::Elimination(_) => {
            return ().into_any();
        }
    };
    let Some(model) = detailed_standings_model(&configuration, i18n) else {
        return ().into_any();
    };
    let search = RwSignal::new(String::new());
    let suggestions_id = StoredValue::new(format!(
        "standing-suggestions-{}",
        tournament.tournament_id().0
    ));
    let entrant_names = Signal::derive(move || {
        tournament
            .common
            .memberships()
            .get()
            .players
            .values()
            .map(|player| player.username.clone())
            .collect()
    });
    let container = NodeRef::<html::Div>::new();
    let row_columns = model.columns.clone();
    let primary_presentation = model.primary_presentation;
    let filtered_rows = Memo::new(move |_| {
        let needle = search.get().trim().to_lowercase();
        let memberships = tournament.common.memberships().get();
        let standings = tournament.common.standings().get();
        let encounters = row_columns
            .iter()
            .map(|column| {
                standings
                    .snapshot
                    .as_ref()
                    .filter(|_| column.direct_encounter)
                    .map(|snapshot| direct_encounter_values(snapshot, column.index))
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        let speed = match &configuration {
            FormatConfig::RoundRobin(configuration) => Some(configuration.clock),
            FormatConfig::Swiss(configuration) => Some(configuration.clock),
            FormatConfig::Elimination(_) | FormatConfig::Arena(_) => None,
        }
        .map(GameSpeed::from);
        standings
            .snapshot
            .iter()
            .flat_map(|snapshot| display_standing_rows(snapshot, &memberships.withdrawn))
            .filter_map(|standing| {
                let user = memberships.players.get(&standing.row.user_id)?;
                if !needle.is_empty() && !user.username.to_lowercase().contains(needle.as_str()) {
                    return None;
                }
                let rating = speed
                    .as_ref()
                    .and_then(|speed| user.ratings.get(speed))
                    .map(|rating| rating.rating.to_string())
                    .unwrap_or_else(|| String::from("—"));
                Some(DetailedStandingsRow {
                    placement: standing.rank,
                    username: user.username.clone(),
                    withdrawn: standing.withdrawn,
                    rating,
                    total: standings_value_text(standing.row.primary_score, primary_presentation),
                    values: row_columns
                        .iter()
                        .copied()
                        .zip(&encounters)
                        .map(|(column, encounters)| {
                            (
                                value_at(&standing.row, column),
                                encounters.get(&standing.row.user_id).cloned(),
                            )
                        })
                        .collect(),
                })
            })
            .collect::<Vec<_>>()
    });
    let matching_count = Signal::derive(move || filtered_rows.with(Vec::len));
    let pagination = use_fitted_pagination(container, matching_count, 25);
    let page = pagination.page;
    let page_size = pagination.page_size;
    let on_page_change = Callback::new(move |next| page.set(next));
    let on_query_change = Callback::new(move |query| {
        search.set(query);
        page.set(1);
    });
    let rows = move || {
        let start = page.get().saturating_sub(1).saturating_mul(page_size.get());
        filtered_rows.with(|rows| {
            rows.iter()
                .skip(start)
                .take(page_size.get())
                .cloned()
                .map(|row| {
                    let DetailedStandingsRow {
                        placement,
                        username,
                        withdrawn,
                        rating,
                        total,
                        values,
                    } = row;
                    view! {
                        <tr data-page-row class="ui-dense-table-row">
                            <td class="sticky left-0 z-10 py-2 px-1 w-14 text-sm text-center whitespace-nowrap min-w-14 bg-even-light dark:bg-surface-panel">
                                {placement
                                    .map(|placement| placement.to_string())
                                    .unwrap_or_else(|| String::from("—"))}
                            </td>
                            <td class="sticky left-14 z-10 py-2 px-1 sm:px-2 min-w-28 max-w-40 bg-even-light dark:bg-surface-panel">
                                <div class=if withdrawn {
                                    "flex gap-1 items-center min-w-0 line-through"
                                } else {
                                    "flex gap-1 items-center min-w-0"
                                }>
                                    <span class="font-semibold text-left truncate">{username}</span>
                                </div>
                            </td>
                            <td class=CELL>{rating}</td>
                            <td class=CELL>
                                <strong>{total}</strong>
                            </td>
                            {values
                                .into_iter()
                                .map(|(text, encounter)| match encounter {
                                    Some(value) => {
                                        view! { <DirectEncounterCell value /> }.into_any()
                                    }
                                    None => view! { <td class=CELL>{text}</td> }.into_any(),
                                })
                                .collect_view()}
                        </tr>
                    }
                })
                .collect_view()
        })
    };

    let primary_name = model.primary_name;
    let columns = model.columns;
    let column_count = columns.len() + 4;
    view! {
        <Panel
            title=move || tournament.common.lifecycle().get().name
            class="min-w-0"
            body_class="p-0"
        >
            <div node_ref=container data-testid="tournament-standings">
                <div class="flex items-center py-2 px-2 min-w-0 border-b sm:px-3 border-black/10 dark:border-white/10">
                    <TournamentStandingsControls
                        page=page.into()
                        total=matching_count
                        page_size
                        query=search.into()
                        on_query_change
                        on_page_change
                        entrant_names
                        suggestions_id=suggestions_id.get_value()
                        search_only=true
                    />
                </div>
                <div class="overflow-x-auto">
                    <table class="w-full table-auto h-fit min-w-[22rem] [&_th+th]:border-l [&_td+td]:border-l [&_th]:border-black/10 [&_td]:border-black/10 dark:[&_th]:border-white/10 dark:[&_td]:border-white/10">
                        <thead>
                            <tr>
                                <th class="sticky left-0 z-20 p-2 w-14 text-sm min-w-14 bg-even-light dark:bg-surface-panel">
                                    {t!(i18n, tournaments.finished_standings.position)}
                                </th>
                                <th class="sticky left-14 z-20 p-2 text-sm bg-even-light dark:bg-surface-panel">
                                    {t!(i18n, common.player)}
                                </th>

                                <th class=HEAD>{t!(i18n, profile.rating)}</th>
                                <th class=HEAD>{move || primary_name.get()}</th>
                                {columns
                                    .into_iter()
                                    .map(|column| {
                                        view! {
                                            <th class=HEAD title=move || column.explanation.get()>
                                                {move || column.name.get()}
                                            </th>
                                        }
                                    })
                                    .collect_view()}
                            </tr>
                        </thead>
                        <tbody data-page-items>
                            <Show when=move || {
                                matching_count.get() == 0 && !search.get().trim().is_empty()
                            }>
                                <tr>
                                    <td colspan=column_count class="p-4 text-sm text-center">

                                        <span>{t!(i18n, common.no_players_match_your_search)}</span>
                                        " "

                                        <button
                                            type="button"
                                            class="ui-text-link"
                                            on:click=move |_| on_query_change.run(String::new())
                                        >
                                            {move || {
                                                t_string!(i18n, common.clear_search).to_string()
                                            }}
                                        </button>
                                    </td>
                                </tr>
                            </Show>
                            {rows}
                        </tbody>
                    </table>
                </div>
                <div
                    class="flex justify-end pt-3"
                    class:hidden=move || matching_count.get() <= page_size.get()
                >
                    <PaginationControls
                        page=page.into()
                        total=matching_count
                        page_size
                        on_page_change
                        hide_single_page=true
                    />
                </div>
            </div>
            <details class="mx-3 mt-3 mb-3 ui-setting-group">

                <summary class="text-sm font-semibold cursor-pointer">
                    {t!(i18n, tournaments.ui.scoring_and_tiebreak_details)}
                </summary>
                <StandingsCriteriaRules configuration=model.rules />
            </details>
        </Panel>
    }
    .into_any()
}
