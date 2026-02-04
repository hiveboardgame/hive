use crate::pages::archive::{form::ArchiveSearchForm, list::ArchiveGameList};
use crate::functions::games::get::GetFinishedBatchFromOptions;
use leptos::{html, prelude::*};
use leptos_router::{
    hooks::{use_navigate, use_query_map},
    location::State,
    NavigateOptions,
};
use leptos_use::{
    use_element_bounding, use_infinite_scroll_with_options, UseInfiniteScrollOptions,
};
use shared_types::{BatchToken, FinishedGamesQueryOptions, GameProgress};
use std::str::FromStr;
use std::sync::Arc;

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
    let draft_options = RwSignal::new(FinishedGamesQueryOptions::default());

    let games = RwSignal::new(Vec::new());
    let next_batch = RwSignal::new(None::<BatchToken>);
    let has_more = StoredValue::new(true);
    let total = RwSignal::new(None::<i64>);
    let errors = RwSignal::new(Vec::<String>::new());
    let applied_options = RwSignal::new(FinishedGamesQueryOptions::default());
    let next_batch_action = ServerAction::<GetFinishedBatchFromOptions>::new();
    let is_first_batch = StoredValue::new(true);
    let has_searched = RwSignal::new(false);

    let scroll_ref = NodeRef::<html::Div>::new();
    let bounding = use_element_bounding(scroll_ref);
    let infinite_scroll_batch_size = Signal::derive(move || {
        let width = bounding.width.get();
        if width < 640.0 {
            3
        } else if width < 1024.0 {
            4
        } else {
            6
        }
    });

    let is_loading = next_batch_action.pending();

    let dispatch_batch: Arc<dyn Fn(usize, bool) + Send + Sync> = {
        let options = applied_options;
        let action = next_batch_action;
        Arc::new(move |batch_size: usize, reset_lists: bool| {
            if action.pending().get_untracked() {
                return;
            }
            let mut opts = options.get_untracked();
            opts.batch_size = batch_size;
            opts.batch_token = if reset_lists {
                None
            } else {
                next_batch.get_untracked()
            };
            is_first_batch.set_value(reset_lists);

            if reset_lists {
                games.set(Vec::new());
                has_more.set_value(true);
                next_batch.set(None);
                total.set(None);
                errors.set(Vec::new());
            }

            options.set(opts.clone());
            action.dispatch(GetFinishedBatchFromOptions { options: opts });
        })
    };

    Effect::watch(
        next_batch_action.version(),
        move |_, _, _| {
            if let Some(result) = next_batch_action.value().get_untracked() {
                match result {
                    Ok(batch) => {
                        has_more.set_value(batch.next_batch.is_some());
                        total.set(Some(batch.total));
                        next_batch.set(batch.next_batch);
                        errors.update(|e| {
                            if !e.is_empty() {
                                e.clear();
                            }
                        });
                        games.update(|state| {
                            if is_first_batch.get_value() {
                                *state = batch.games;
                            } else {
                                state.extend(batch.games);
                            }
                        });
                    }
                    Err(err_msg) => {
                        errors.set(vec![err_msg.to_string()]);
                        has_more.set_value(false);
                    }
                }
            }
        },
        true,
    );

    let fetch_more = Arc::new({
        let dispatch_batch = Arc::clone(&dispatch_batch);
        move || {
            dispatch_batch(infinite_scroll_batch_size.get_untracked(), false);
        }
    });

    let navigate = use_navigate();
    let queries = use_query_map();
    {
        let dispatch_batch = Arc::clone(&dispatch_batch);

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
                    return;
                }

                errors.set(Vec::new());
                match FinishedGamesQueryOptions::from_str(&query_string) {
                    Ok(mut opts) => {
                        opts.batch_size = infinite_scroll_batch_size.get_untracked();
                        opts.batch_token = None;
                        match opts.validate_all() {
                            Ok(valid) => {
                                draft_options.set(valid.clone());
                                applied_options.set(valid.clone());
                                has_searched.set(true);
                                dispatch_batch(valid.batch_size, true);
                            }
                            Err(errs) => {
                                errors.set(errs.into_iter().map(|e| e.to_string()).collect());
                                games.set(Vec::new());
                                total.set(None);
                                has_more.set_value(false);
                                has_searched.set(true);
                            }
                        }
                    }
                    Err(e) => {
                        let msgs = match e {
                            shared_types::FinishedGamesQueryParseError::ValidationFailedList(
                                errs,
                            ) => errs.into_iter().map(|e| e.to_string()).collect(),
                            _ => vec![e.to_string()],
                        };
                        errors.set(msgs);
                        games.set(Vec::new());
                        total.set(None);
                        has_more.set_value(false);
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
            opts.batch_size = infinite_scroll_batch_size.get_untracked();
            opts.batch_token = None;
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
                    has_more.set_value(false);
                    has_searched.set(false);
                }
            }
        }
    };

    let value = fetch_more.clone();
    let _ = use_infinite_scroll_with_options(
        scroll_ref,
        move |_| {
            let fetch_more = Arc::clone(&value);
            async move {
                if !has_searched() || !has_more.get_value() || is_loading.get() {
                    return;
                }
                fetch_more();
            }
        },
        UseInfiniteScrollOptions::default()
            .distance(10.0)
            .interval(300.0),
    );

    view! {
        <div
            node_ref=scroll_ref
            class="flex flex-col min-h-screen max-h-screen w-full bg-light dark:bg-gray-950 pt-20 overflow-y-auto"
        >
            <ArchiveSearchForm draft_options=draft_options on_search=Callback::new(move |_| start_search(())) />

            <div class="px-4 pb-6">
                <div class="space-y-2">
                    <ErrorBoundary fallback=move |errors_signal| {
                        let messages = Signal::derive(move || {
                            errors_signal
                                .get()
                                .into_iter()
                                .map(|(_, err)| err.to_string())
                                .collect::<Vec<_>>()
                        });
                        view! {
                            <div class="flex-1 min-h-0 flex flex-col">
                                <div class="p-2 text-sm text-red-600 dark:text-red-400 space-y-1">
                                    <For each=move || messages.get() key=|msg| msg.clone() let:msg>
                                        <p>{msg}</p>
                                    </For>
                                </div>
                            </div>
                        }
                    }>
                        {move || {
                            if errors.with(|e| e.is_empty()) {
                                Ok(view! {
                                    <ArchiveGameList
                                        games=games
                        is_loading=is_loading.into()
                            has_searched=has_searched.into()
                                        total=total
                                    />
                                })
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
