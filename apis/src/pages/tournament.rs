use crate::{
    common::{markdown_to_html, with_class, ScheduleAction, TournamentAction},
    components::{
        atoms::progress_bar::ProgressBar,
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::{
            game_previews::GamePreviews,
            my_schedules::MySchedules,
            panel::Panel,
            time_row::TimeRow,
            unplayed_game_row::UnplayedGameRow,
            user_identity::UserIdentity,
        },
        organisms::{
            chat::ChatWindow,
            standings::Standings,
            tournament_admin::TournamentAdminControls,
        },
        update_from_event::update_from_input,
    },
    functions::tournaments::{get_complete, UpdateDescription},
    providers::{
        websocket::{ConnectionReadyState, WebsocketContext},
        ApiRequestsProvider,
        AuthContext,
        UpdateNotifier,
    },
    responses::{GameResponse, TournamentResponse},
};
use chrono::Local;
use hudsoni::GameStatus;
use leptos::{html, prelude::*};
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_use::use_resize_observer;
use shared_types::{
    Conclusion,
    GameSpeed,
    PrettyString,
    TimeInfo,
    TournamentGameResult,
    TournamentId,
    TournamentMode,
    TournamentStatus,
};
use std::collections::HashMap;

const DETAILS_STYLE: &str = "w-full min-w-0 h-fit";

#[component]
pub fn Tournament() -> impl IntoView {
    let use_params = use_params_map();
    let update_notification = expect_context::<UpdateNotifier>().tournament_update;
    let tournament_id = move || {
        use_params
            .get_untracked()
            .get("nanoid")
            .map(|s| TournamentId(s.to_string()))
    };
    let current_tournament =
        Action::new(move |_: &()| async move { get_complete(tournament_id().unwrap()).await.ok() });
    Effect::watch(
        update_notification,
        move |needs_update, _, _| {
            if Some(needs_update) == tournament_id().as_ref() {
                current_tournament.dispatch(());
            }
        },
        true,
    );
    Effect::new(move |_| {
        current_tournament.dispatch(());
    });
    view! {
        <PageShell variant=PageShellVariant::Dashboard>
            <div class="flex flex-col gap-6 w-full max-w-6xl">
                <Show
                    when=move || current_tournament.value().get().is_some()
                    fallback=|| {
                        view! {
                            <Panel>
                                <p class="text-sm text-gray-600 dark:text-gray-300">
                                    "Loading tournament..."
                                </p>
                            </Panel>
                        }
                    }
                >
                    <LoadedTournament tournament=current_tournament
                        .value()
                        .get()
                        .flatten()
                        .expect("Current tournament is some") />
                </Show>
            </div>
        </PageShell>
    }
}

//TODO: Bring back fine grained reactivity. All the "signals" we had in here weren't really signals as they all depended on static data
#[component]
fn LoadedTournament(tournament: TournamentResponse) -> impl IntoView {
    let tournament = StoredValue::new(tournament);
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let websocket = expect_context::<WebsocketContext>();
    let account = auth_context.user;
    let user_id = Signal::derive(move || account.with(|a| a.as_ref().map(|a| a.user.uid)));
    let time_info = tournament.with_value(|t| TimeInfo {
        mode: t.time_mode,
        base: t.time_base,
        increment: t.time_increment,
    });
    let tournament_id = StoredValue::new(tournament.with_value(|t| t.tournament_id.clone()));
    Effect::new(move |_| {
        let ready_state = websocket.ready_state.get();
        if tournament.with_value(|t| t.status.clone()) != TournamentStatus::NotStarted
            && ready_state == ConnectionReadyState::Open
        {
            let api = api.get();
            api.schedule_action(ScheduleAction::TournamentPublic(tournament_id.get_value()));
            if user_id().is_some() {
                api.schedule_action(ScheduleAction::TournamentOwn(tournament_id.get_value()));
            }
        }
    });

    let games_hashmap = StoredValue::new({
        let mut games_hashmap = HashMap::new();
        if tournament.with_value(|t| t.status.clone()) != TournamentStatus::NotStarted {
            tournament.with_value(|t| {
                for game in &t.games {
                    games_hashmap.insert(game.game_id.clone(), game.clone());
                }
            });
        }
        games_hashmap
    });

    let number_of_players = tournament.with_value(|t| t.players.len() as i32);
    let user_joined = move || {
        account.with(|a| {
            if let Some(account) = a.as_ref() {
                tournament.with_value(|t| t.players.iter().any(|(id, _)| *id == account.id))
            } else {
                false
            }
        })
    };

    let user_is_organizer_or_admin = Signal::derive(move || {
        account.with(|a| {
            if let Some(account) = a.as_ref() {
                account.user.admin
                    || tournament.with_value(|t| t.organizers.iter().any(|p| p.uid == account.id))
            } else {
                false
            }
        })
    });
    let send_action = move |action: TournamentAction| {
        let api = api.get();
        api.tournament(action);
    };
    let delete = move |_| {
        if user_is_organizer_or_admin() {
            send_action(TournamentAction::Delete(tournament_id.get_value()));
            let navigate = use_navigate();
            navigate("/tournaments", Default::default());
        }
    };
    let finish = move |_| {
        if user_is_organizer_or_admin() {
            send_action(TournamentAction::Finish(tournament_id.get_value()));
        }
    };
    let progress_to_next_round = move |_| {
        if user_is_organizer_or_admin() {
            send_action(TournamentAction::ProgressToNextRound(
                tournament_id.get_value(),
            ));
        }
    };
    let start = move |_| {
        if user_is_organizer_or_admin() {
            send_action(TournamentAction::Start(tournament_id.get_value()));
        }
    };
    let start_disabled = move || tournament.with_value(|t| t.min_seats) > number_of_players;
    let join_disabled = move || {
        if tournament.with_value(|t| t.seats) <= number_of_players {
            return true;
        }
        account.with(|a| {
            if let Some(account) = a.as_ref() {
                let user = &account.user;
                tournament.with_value(|t| {
                    if t.invite_only {
                        if t.invitees.iter().any(|invitee| invitee.uid == user.uid) {
                            return false;
                        }
                        if t.organizers
                            .iter()
                            .any(|organizer| organizer.uid == user.uid)
                        {
                            return false;
                        }
                        return true;
                    }
                    let game_speed = GameSpeed::from_base_increment(t.time_base, t.time_increment);
                    let rating = user.rating_for_speed(&game_speed) as i32;
                    match (t.band_lower, t.band_upper) {
                        (None, None) => false,
                        (None, Some(upper)) => rating >= upper,
                        (Some(lower), None) => rating <= lower,
                        (Some(lower), Some(upper)) => rating <= lower || rating >= upper,
                    }
                })
            } else {
                true
            }
        })
    };

    let starts = tournament.with_value(|tournament| {
        if matches!(tournament.status, TournamentStatus::NotStarted) {
            match tournament.starts_at {
                None => "Start up to organizer".to_string(),
                Some(time) => time
                    .with_timezone(&Local)
                    .format("Starts: %d/%m/%Y %H:%M %Z")
                    .to_string(),
            }
        } else {
            let pretty = tournament.status.pretty_string();
            if let Some(started_at) = tournament.started_at {
                let start = started_at
                    .with_timezone(&Local)
                    .format("started: %d/%m/%Y %H:%M %Z")
                    .to_string();
                format!("{pretty}, {start}")
            } else {
                pretty
            }
        }
    });
    let total_games = tournament.with_value(|t| t.games.len());
    let finished_games = tournament.with_value(|t| t.games.iter().filter(|g| g.finished).count());
    let not_started = tournament.with_value(|t| t.status == TournamentStatus::NotStarted);
    let inprogress = tournament.with_value(|t| t.status == TournamentStatus::InProgress);
    let finished = tournament.with_value(|t| t.status == TournamentStatus::Finished);
    let tournament_is_swiss = tournament.with_value(|t| {
        matches!(
            t.mode.parse::<TournamentMode>().ok(),
            Some(TournamentMode::DoubleSwiss)
        )
    });
    let game_previews = Memo::new(move |_| {
        games_hashmap.with_value(|hashmap| {
            hashmap
                .values()
                .filter_map(|g| match g.game_status {
                    GameStatus::NotStarted => None,
                    _ => Some(g.clone()),
                })
                .collect::<Vec<GameResponse>>()
        })
    });
    let current_description = RwSignal::new(String::new());
    let markdown_desc = move || markdown_to_html(&current_description());
    let editing_description = RwSignal::new(false);
    let previewing_description = RwSignal::new(false);
    let description_text = RwSignal::new(String::new());
    let update_desc_action = ServerAction::<UpdateDescription>::new();

    Effect::new(move |_| {
        current_description.set(tournament.with_value(|t| t.description.clone()));
    });

    Effect::new(move |_| {
        if let Some(Ok(())) = update_desc_action.value().get() {
            current_description.set(description_text.get_untracked());
            editing_description.set(false);
            previewing_description.set(false);
        }
    });
    let unplayed_games = StoredValue::new({
        let mut result = tournament.with_value(|t| {
            t.games
                .iter()
                .filter(|g| g.organizer_can_adjudicate())
                .cloned()
                .collect::<Vec<GameResponse>>()
        });
        result.sort();
        result
    });
    let has_my_schedules = Signal::derive(move || {
        user_id().is_some_and(|user_id| {
            games_hashmap.with_value(|games| {
                games.iter().any(|(_game_id, game)| {
                    game.game_status == GameStatus::NotStarted
                        && (game.white_player.uid == user_id || game.black_player.uid == user_id)
                        && game.conclusion == Conclusion::Unknown
                })
            })
        })
    });
    let has_unplayed_games =
        Signal::derive(move || !unplayed_games.with_value(|games| games.is_empty()));
    let has_game_previews = Signal::derive(move || game_previews.with(|games| !games.is_empty()));
    let has_top_game_sections = Signal::derive(move || has_my_schedules() || has_unplayed_games());
    let top_game_sections_layout = move || {
        if has_my_schedules() && has_unplayed_games() {
            "grid w-full gap-4 lg:grid-cols-2 lg:items-start"
        } else {
            "grid w-full gap-4"
        }
    };
    let tournament_lacks_results = tournament.with_value(|t| {
        t.games
            .iter()
            .any(|e| e.tournament_game_result == TournamentGameResult::Unknown)
    });
    let unstarted_games_count = Signal::derive(move || {
        tournament.with_value(|t| {
            t.games
                .iter()
                .filter(|g| g.game_status == GameStatus::NotStarted)
                .count()
        })
    });
    let adjudicated_games_count = Signal::derive(move || {
        tournament.with_value(|t| {
            t.games
                .iter()
                .filter(|g| g.game_status == GameStatus::Adjudicated)
                .count()
        })
    });
    let confirming_double_forfeit = RwSignal::new(false);
    let confirming_reset_adjudicated = RwSignal::new(false);
    let unstarted_games_label = move || {
        let nr = unstarted_games_count();
        if nr == 1 {
            String::from("1 unstarted game")
        } else {
            format!("{nr} unstarted games")
        }
    };
    let request_double_forfeit = move |_| {
        if user_is_organizer_or_admin() && inprogress {
            confirming_double_forfeit.set(true);
        }
    };
    let cancel_double_forfeit = move |_| confirming_double_forfeit.set(false);
    let double_forfeit_unstarted = move |_| {
        if user_is_organizer_or_admin() && inprogress {
            send_action(TournamentAction::DoubleForfeitUnstartedGames(
                tournament_id.get_value(),
            ));
            confirming_double_forfeit.set(false);
        }
    };
    let reset_adjudicated = move |_| {
        if user_is_organizer_or_admin() && inprogress {
            send_action(TournamentAction::ResetAdjudicatedGames(
                tournament_id.get_value(),
            ));
            confirming_reset_adjudicated.set(false);
        }
    };
    let unplayed_games_string = move || {
        let nr = unplayed_games.with_value(|games| games.len());
        if nr == 1 {
            String::from("1 unplayed game")
        } else {
            format!("{nr} unplayed games")
        }
    };
    let tournament_style =
        "grid w-full gap-6 lg:grid-cols-[repeat(auto-fit,minmax(min(100%,28rem),1fr))] lg:items-start";
    let tournament_info_ref = NodeRef::<html::Div>::new();
    let tournament_info_height = RwSignal::new(None::<f64>);
    use_resize_observer(tournament_info_ref, move |entries, _observer| {
        let rect = entries[0].content_rect();
        tournament_info_height.set(Some(rect.height()));
    });
    view! {
        <PageHeader title=tournament.with_value(|t| t.name.clone()) subtitle=starts.clone() />
        <Panel body_class="space-y-4">
            <Show
                when=editing_description
                fallback=move || {
                    view! {
                        <div
                            class="w-full max-w-none break-words prose dark:prose-invert"
                            inner_html=markdown_desc
                        />
                        <Show when=user_is_organizer_or_admin>
                            <button
                                class="mb-2 ui-button ui-button-secondary ui-button-md"
                                on:click=move |_| {
                                    description_text.set(current_description.get_untracked());
                                    editing_description.set(true);
                                    previewing_description.set(false);
                                }
                            >
                                "Edit Description"
                            </button>
                        </Show>
                    }
                }
            >
                <ActionForm action=update_desc_action attr:class="space-y-3">
                    <input
                        type="hidden"
                        name="tournament_id"
                        value=tournament.with_value(|t| t.tournament_id.0.clone())
                    />
                    <Show
                        when=previewing_description
                        fallback=move || {
                            view! {
                                <textarea
                                    class="ui-field-textarea min-h-36"
                                    name="description"
                                    prop:value=description_text
                                    on:input=update_from_input(description_text)
                                    maxlength="2000"
                                    placeholder="At least 50 characters required"
                                ></textarea>
                            }
                        }
                    >
                        <div
                            class=with_class(
                                "ui-setting-group",
                                "min-h-36 w-full max-w-none break-words prose dark:prose-invert",
                            )
                            inner_html=move || markdown_to_html(&description_text())
                        />
                    </Show>
                    <div class="flex flex-wrap gap-2 items-center">
                        <button
                            type="submit"
                            class="ui-button ui-button-primary ui-button-md"
                            prop:disabled=move || {
                                !(50..=2000).contains(&description_text().len())
                            }
                        >
                            "Update Description"
                        </button>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| {
                                editing_description.set(false);
                                previewing_description.set(false);
                            }
                        >
                            "Cancel"
                        </button>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| previewing_description.update(|b| *b = !*b)
                        >
                            {move || if previewing_description() { "Edit" } else { "Preview" }}
                        </button>
                        <a
                            class="ui-button ui-button-ghost ui-button-md no-link-style"
                            href="https://commonmark.org/help/"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            "Markdown"
                        </a>
                    </div>
                </ActionForm>
            </Show>
        </Panel>
        <div class=tournament_style>
            <div node_ref=tournament_info_ref class="min-w-0">
                <Panel title="Tournament Info" class="min-w-0" body_class="space-y-3 h-fit">
                    <div>
                        <span class="font-bold">"Type: "</span>
                        {tournament
                            .with_value(|t| {
                                t.mode
                                    .parse::<TournamentMode>()
                                    .map(|m| m.pretty_string())
                                    .unwrap_or_default()
                            })}
                    </div>
                    <div class="flex flex-wrap gap-1">
                        <span class="font-bold">"Time control: "</span>
                        <TimeRow time_info=time_info />
                    </div>
                    <div>
                        <span class="font-bold">"Players: "</span>
                        {number_of_players}
                        /
                        {tournament.with_value(|t| t.seats)}
                    </div>
                    <Show when=move || not_started>
                        <div>
                            <span class="font-bold">"Minimum players: "</span>
                            {tournament.with_value(|t| t.min_seats)}
                        </div>
                    </Show>
                    <p class="ui-notice">{starts.clone()}</p>
                    <div class="space-y-2 ui-setting-group">
                        <div class="flex flex-col gap-2">
                            <p class="font-bold">"Organized by"</p>
                            <For
                                each=move || { tournament.with_value(|t| t.organizers.clone()) }

                                key=|users| users.uid
                                let:user
                            >
                                <div>
                                    <UserIdentity
                                        user
                                        class="p-1 h-10"
                                        link_class="truncate max-w-[120px]"
                                    />
                                </div>
                            </For>
                        </div>
                    </div>
                    <ProgressBar current=finished_games.into() total=total_games />
                    <Show when=move || not_started>
                        <div class="flex flex-wrap gap-2">
                            <Show
                                when=user_joined
                                fallback=move || {
                                    view! {
                                        <button
                                            prop:disabled=join_disabled
                                            class="ui-button ui-button-primary ui-button-md"
                                            on:click=move |_| send_action(
                                                TournamentAction::Join(tournament_id.get_value()),
                                            )
                                        >
                                            Join
                                        </button>
                                    }
                                }
                            >

                                <button
                                    class="ui-button ui-button-secondary ui-button-md"
                                    on:click=move |_| send_action(
                                        TournamentAction::Leave(tournament_id.get_value()),
                                    )
                                >
                                    Leave
                                </button>
                            </Show>
                            <Show when=user_is_organizer_or_admin>
                                <button
                                    class="ui-button ui-button-danger ui-button-md"
                                    on:click=delete
                                >
                                    {"Delete"}
                                </button>
                                <button
                                    prop:disabled=start_disabled
                                    class="ui-button ui-button-primary ui-button-md"
                                    on:click=start
                                >
                                    {"Start"}
                                </button>
                            </Show>
                        </div>
                        <TournamentAdminControls
                            user_is_organizer=user_is_organizer_or_admin()
                            tournament
                        />
                    </Show>
                    <Show when=move || user_is_organizer_or_admin() && inprogress>
                        <div class="flex flex-col gap-3 ui-setting-group">
                            <div class="flex flex-wrap gap-2">
                                <button
                                    class="ui-button ui-button-primary ui-button-md"
                                    on:click=finish
                                    prop:disabled=tournament_lacks_results
                                >
                                    {"Finish"}
                                </button>
                            </div>
                            <Show when=move || { unstarted_games_count() > 0 }>
                                <div class="flex flex-col gap-2">
                                    <p class="text-sm font-semibold text-center">
                                        {move || {
                                            format!(
                                                "{} will be double forfeited.",
                                                unstarted_games_label(),
                                            )
                                        }}
                                    </p>
                                    <Show
                                        when=confirming_double_forfeit
                                        fallback=move || {
                                            view! {
                                                <button
                                                    class="ui-button ui-button-danger ui-button-md"
                                                    on:click=request_double_forfeit
                                                >
                                                    "Double forfeit unstarted games"
                                                </button>
                                            }
                                        }
                                    >
                                        <div class="flex flex-col gap-2 items-center">
                                            <div class="text-sm text-center">
                                                {"This will adjudicate every unstarted tournament game as a double forfeit."}
                                            </div>
                                            <div class="flex gap-2">
                                                <button
                                                    class="ui-button ui-button-danger ui-button-md"
                                                    on:click=double_forfeit_unstarted
                                                >
                                                    {move || format!("Confirm {}", unstarted_games_label())}
                                                </button>
                                                <button
                                                    class="ui-button ui-button-secondary ui-button-md"
                                                    on:click=cancel_double_forfeit
                                                >
                                                    {"Cancel"}
                                                </button>
                                            </div>
                                        </div>
                                    </Show>
                                </div>
                            </Show>
                            <Show when=move || { adjudicated_games_count() > 0 }>
                                <div class="flex flex-col gap-2">
                                    <p class="text-sm font-semibold text-center">
                                        {move || {
                                            let count = adjudicated_games_count();
                                            if count == 1 {
                                                String::from("1 adjudicated game will be reset.")
                                            } else {
                                                format!("{count} adjudicated games will be reset.")
                                            }
                                        }}
                                    </p>
                                    <Show
                                        when=confirming_reset_adjudicated
                                        fallback=move || {
                                            view! {
                                                <button
                                                    class="ui-button ui-button-secondary ui-button-md"
                                                    on:click=move |_| confirming_reset_adjudicated.set(true)
                                                >
                                                    "Undo adjudications"
                                                </button>
                                            }
                                        }
                                    >
                                        <div class="flex flex-col gap-2 items-center">
                                            <div class="text-sm text-center">
                                                {"This will set all adjudicated tournament games back to not started."}
                                            </div>
                                            <div class="flex gap-2">
                                                <button
                                                    class="ui-button ui-button-danger ui-button-md"
                                                    on:click=reset_adjudicated
                                                >
                                                    {"Confirm undo"}
                                                </button>
                                                <button
                                                    class="ui-button ui-button-secondary ui-button-md"
                                                    on:click=move |_| confirming_reset_adjudicated.set(false)
                                                >
                                                    {"Cancel"}
                                                </button>
                                            </div>
                                        </div>
                                    </Show>
                                </div>
                            </Show>
                        </div>
                    </Show>
                    // only show if tournament is Swiss
                    <Show when=move || user_is_organizer_or_admin.get() && tournament_is_swiss>
                        <div class="flex flex-wrap gap-2">
                            <Show when=move || inprogress>
                                <button
                                    class="ui-button ui-button-primary ui-button-md"
                                    on:click=progress_to_next_round
                                    // this is also disabled as the Finish button until all existing games have results
                                    prop:disabled=tournament_lacks_results
                                >
                                    {"Progress to next round"}
                                </button>
                            </Show>
                        </div>
                    </Show>
                </Panel>
            </div>
            <Show when=move || !not_started>
                <Standings
                    tournament=Signal::derive(move || tournament.get_value())
                    max_height=tournament_info_height
                />
            </Show>
        </div>
        <div class="flex flex-col gap-4 w-full">
            <Show when=has_top_game_sections>
                <div class=top_game_sections_layout>
                    <Show when=has_my_schedules>
                        <MySchedules
                            games_hashmap=Memo::new(move |_| games_hashmap.get_value())
                            user_id
                        />
                    </Show>
                    <Show when=has_unplayed_games>
                        <div class="min-w-0">
                            <details class=with_class("ui-panel", DETAILS_STYLE)>
                                <summary class="ui-panel-summary">{unplayed_games_string}</summary>
                                <div class="space-y-2 ui-panel-body">
                                    <For
                                        each=move || unplayed_games.get_value()

                                        key=|game| (
                                            game.game_id.clone(),
                                            game.tournament_game_result.clone(),
                                            game.finished,
                                        )
                                        let:game
                                    >

                                        <UnplayedGameRow
                                            game
                                            user_is_organizer=user_is_organizer_or_admin
                                            tournament_finished=finished.into()
                                        />

                                    </For>
                                </div>
                            </details>
                        </div>
                    </Show>
                </div>
            </Show>
            <Show when=has_game_previews>
                <details class=with_class("ui-panel", DETAILS_STYLE)>
                    <summary class="ui-panel-summary">"Finished or ongoing games"</summary>
                    <div class="ui-panel-body">
                        <GamePreviews games=game_previews />
                    </div>
                </details>
            </Show>
        </div>
        <Show when=move || user_is_organizer_or_admin() || user_joined()>
            <div class=with_class(
                "ui-panel",
                "h-72 w-full overflow-hidden p-3 whitespace-normal break-words",
            )>
                <ChatWindow destination=shared_types::SimpleDestination::Tournament(
                    tournament_id.get_value(),
                ) />
            </div>
        </Show>
    }
}
