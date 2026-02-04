use crate::pages::archive::{form::ArchiveSearchForm, list::ArchiveGameList};
use crate::functions::games::get::GetFinishedBatchFromOptions;
use leptos::{html, prelude::*};
use leptos_router::{
    hooks::{use_navigate, use_query_map},
    location::State,
    NavigateOptions,
};
use shared_types::{BatchToken, FinishedGamesQueryOptions, GameProgress};
use std::str::FromStr;
use std::sync::Arc;

const ARCHIVE_PAGE_SIZE: usize = 50;

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
    let total = RwSignal::new(None::<i64>);
    let errors = RwSignal::new(Vec::<String>::new());
    let applied_options = RwSignal::new(FinishedGamesQueryOptions::default());
    let next_batch_action = ServerAction::<GetFinishedBatchFromOptions>::new();
    let has_searched = RwSignal::new(false);

    let scroll_ref = NodeRef::<html::Div>::new();
    let is_loading = next_batch_action.pending();

    let fetch_page: Arc<dyn Fn(FinishedGamesQueryOptions) + Send + Sync> = {
        let action = next_batch_action;
        Arc::new(move |opts: FinishedGamesQueryOptions| {
            if action.pending().get_untracked() {
                return;
            }
            action.dispatch(GetFinishedBatchFromOptions { options: opts });
        })
    };

    Effect::watch(
        next_batch_action.version(),
        move |_, _, _| {
            if let Some(result) = next_batch_action.value().get_untracked() {
                match result {
                    Ok(batch) => {
                        total.set(Some(batch.total));
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
            }
        },
        true,
    );

    let navigate = use_navigate();
    let queries = use_query_map();
    {
        let fetch_page = Arc::clone(&fetch_page);
        let applied_options = applied_options;

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
                        opts.batch_size = ARCHIVE_PAGE_SIZE;
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
                                errors.set(errs.into_iter().map(|e| e.to_string()).collect());
                                games.set(Vec::new());
                                total.set(None);
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
            opts.batch_size = ARCHIVE_PAGE_SIZE;
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
                                let navigate = navigate.clone();
                                let on_page_change = Callback::new(move |new_page: usize| {
                                    let mut opts = applied_options.get_untracked();
                                    opts.page = new_page;
                                    if let Ok(valid) = opts.validate_all() {
                                        applied_options.set(valid.clone());
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
                                });
                                Ok(view! {
                                    <ArchiveGameList
                                        games=games
                                        is_loading=is_loading.into()
                                        has_searched=has_searched.into()
                                        total=total
                                        page=Signal::derive(move || applied_options.with(|o| o.page))
                                        batch_size=ARCHIVE_PAGE_SIZE
                                        on_page_change=on_page_change
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
