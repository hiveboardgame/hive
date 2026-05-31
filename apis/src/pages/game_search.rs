use crate::{
    functions::games::get::get_batch_from_options,
    pages::archive::{form::ArchiveSearchForm, list::ArchiveGameList},
};
use leptos::{html, prelude::*, task::spawn_local};
use leptos_router::{
    hooks::{use_navigate, use_query_map},
    location::State,
    NavigateOptions,
};
use shared_types::{GameProgress, GameSpeed, GamesQueryOptions};
use std::sync::Arc;

/// Sane defaults shown in the archive form before the user searches.
fn default_search_options() -> GamesQueryOptions {
    GamesQueryOptions {
        speeds: GameSpeed::all_games()
            .into_iter()
            .filter(|speed| *speed != GameSpeed::Untimed)
            .collect(),
        rated: Some(true),
        expansions: Some(true),
        exclude_bots: true,
        fixed_colors: false,
        ..GamesQueryOptions::default()
    }
}

#[derive(Debug, Clone)]
struct GameSearchViewError(Vec<String>);

impl std::fmt::Display for GameSearchViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.join("; "))
    }
}

impl std::error::Error for GameSearchViewError {}

#[component]
pub fn GameSearch() -> impl IntoView {
    let draft_options = RwSignal::new(default_search_options());

    let games = RwSignal::new(Vec::new());
    let total = RwSignal::new(None::<i64>);
    let errors = RwSignal::new(Vec::<String>::new());
    let applied_options = RwSignal::new(default_search_options());
    let has_searched = RwSignal::new(false);
    // Monotonic id so an in-flight fetch can tell whether a newer one has
    // superseded it; lets newer searches/pages replace older ones instead of
    // being dropped, and ignores out-of-order responses.
    let request_seq = StoredValue::new(0_u64);

    let scroll_ref = NodeRef::<html::Div>::new();
    let is_loading = RwSignal::new(false);

    let fetch_page: Arc<dyn Fn(GamesQueryOptions) + Send + Sync> =
        Arc::new(move |opts: GamesQueryOptions| {
            request_seq.update_value(|n| *n += 1);
            let seq = request_seq.get_value();
            is_loading.set(true);
            spawn_local(async move {
                let result = get_batch_from_options(opts).await;
                // Drop stale responses: a newer request has superseded this one.
                if request_seq.get_value() != seq {
                    return;
                }
                is_loading.set(false);
                match result {
                    Ok(batch) => {
                        total.set(batch.total);
                        errors.update(|e| {
                            if !e.is_empty() {
                                e.clear();
                            }
                        });
                        games.set(batch.games);
                    }
                    Err(err_msg) => {
                        errors.set(vec![err_msg.to_string()]);
                    }
                }
            });
        });

    let navigate = use_navigate();
    let queries = use_query_map();
    {
        let fetch_page = Arc::clone(&fetch_page);

        Effect::watch(
            queries,
            move |_, _, _| {
                let query_string = queries
                    .get_untracked()
                    .into_iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join("&");
                if query_string.is_empty() {
                    // Invalidate any in-flight fetch so a stale response can't
                    // repopulate the cleared page (the seq check at the top of
                    // fetch_page drops it).
                    request_seq.update_value(|n| *n += 1);
                    games.set(Vec::new());
                    total.set(None);
                    errors.set(Vec::new());
                    has_searched.set(false);
                    is_loading.set(false);
                    draft_options.set(default_search_options());
                    applied_options.set(default_search_options());
                    return;
                }

                errors.set(Vec::new());
                match GamesQueryOptions::parse_query(&query_string) {
                    Ok(mut opts) => {
                        opts.batch_token = None;
                        opts.game_progress = GameProgress::Finished;
                        match opts.validate_all() {
                            Ok(valid) => {
                                draft_options.set(valid.clone());
                                applied_options.set(valid.clone());
                                has_searched.set(true);
                                fetch_page(valid);
                            }
                            Err(errs) => {
                                // Invalidate any in-flight fetch so a stale
                                // response can't clear the error or repopulate
                                // results for this invalid search.
                                request_seq.update_value(|n| *n += 1);
                                is_loading.set(false);
                                errors.set(errs.into_iter().map(|e| e.to_string()).collect());
                                games.set(Vec::new());
                                total.set(None);
                                has_searched.set(true);
                            }
                        }
                    }
                    Err(errs) => {
                        // Invalidate any in-flight fetch so a stale response
                        // can't clear the error or repopulate results for this
                        // invalid search.
                        request_seq.update_value(|n| *n += 1);
                        is_loading.set(false);
                        errors.set(errs.into_iter().map(|e| e.to_string()).collect());
                        games.set(Vec::new());
                        total.set(None);
                        has_searched.set(true);
                    }
                }
            },
            true,
        );
    }

    let start_search = {
        let navigate = navigate.clone();
        move |_| {
            let nav_options = NavigateOptions {
                resolve: true,
                replace: true,
                scroll: false,
                state: State::new(None),
            };
            errors.set(Vec::new());
            let mut opts = draft_options.get_untracked();
            opts.batch_token = None;
            opts.page = 1;
            opts.game_progress = GameProgress::Finished;
            match opts.validate_all() {
                Ok(valid) => {
                    applied_options.set(valid.clone());
                    has_searched.set(true);
                    navigate(&format!("/archive{valid}",), nav_options);
                }
                Err(errs) => {
                    errors.set(errs.into_iter().map(|e| e.to_string()).collect());
                    games.set(Vec::new());
                    total.set(None);
                    has_searched.set(false);
                }
            }
        }
    };

    let on_search_cb = Callback::new({
        let start_search = start_search;
        move |_| start_search(())
    });

    let on_page_change_cb = Callback::new({
        let navigate = navigate.clone();
        let applied = applied_options;
        move |new_page: usize| {
            let mut opts = applied.get_untracked();
            opts.page = new_page;
            if let Ok(valid) = opts.validate_all() {
                applied.set(valid.clone());
                navigate(
                    &format!("/archive{valid}",),
                    NavigateOptions {
                        resolve: true,
                        replace: false,
                        scroll: true,
                        state: State::new(None),
                    },
                );
            }
        }
    });

    view! {
        <div
            node_ref=scroll_ref
            class="flex overflow-y-auto flex-col pt-20 w-full min-h-screen max-h-screen bg-light dark:bg-gray-950"
        >
            <ArchiveSearchForm draft_options=draft_options on_search=on_search_cb />

            <div class="flex-1 px-4 pb-8 mx-auto w-full max-w-5xl sm:px-6">
                <div class="space-y-4">
                    <ErrorBoundary fallback=move |errors_signal| {
                        let messages = Signal::derive(move || {
                            errors_signal
                                .get()
                                .into_iter()
                                .map(|(_, err)| err.to_string())
                                .collect::<Vec<_>>()
                        });
                        view! {
                            <div class="flex flex-col flex-1 min-h-0">
                                <div class="p-4 space-y-1 text-sm text-red-700 bg-red-50 rounded-xl border-2 border-red-200 shadow-sm dark:text-red-300 dark:border-red-900/50 dark:bg-red-950/30">
                                    <For each=move || messages.get() key=|msg| msg.clone() let:msg>
                                        <p>{msg}</p>
                                    </For>
                                </div>
                            </div>
                        }
                    }>
                        {move || {
                            if errors.with(|e| e.is_empty()) {
                                Ok(
                                    view! {
                                        <ArchiveGameList
                                            games=games
                                            is_loading=is_loading.into()
                                            has_searched=has_searched.into()
                                            total=total
                                            page=Signal::derive(move || {
                                                applied_options.with(|o| o.page)
                                            })
                                            batch_size=Signal::derive(move || {
                                                applied_options.with(|o| o.batch_size)
                                            })
                                            on_page_change=on_page_change_cb
                                            draft_options=draft_options
                                            on_search=on_search_cb
                                        />
                                    },
                                )
                            } else {
                                Err(GameSearchViewError(errors.get()))
                            }
                        }}
                    </ErrorBoundary>
                </div>
            </div>
        </div>
    }
}
