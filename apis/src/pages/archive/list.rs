use crate::components::molecules::game_row::GameRow;
use crate::i18n::*;
use crate::responses::GameResponse;
use leptos::prelude::*;
use leptos_router::hooks::use_query_map;

#[component]
pub fn ArchiveGameList(
    games: RwSignal<Vec<GameResponse>>,
    is_loading: Signal<bool>,
    has_searched: Signal<bool>,
    total: RwSignal<Option<i64>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let queries = use_query_map();

    view! {
        <div class="space-y-2">
            <Show when=move || total.get().is_some()>
                <div class="max-w-5xl mx-auto px-4 flex gap-1">
                    <p class="text-sm text-gray-700 dark:text-gray-300">
                        {move || {
                            let loaded = games.with(|g| g.len());
                            total
                                .get()
                                .map(|t| {
                                    t_string!(i18n, archive.games_loaded_count, loaded = loaded, total = t)
                                })
                                .unwrap_or_default()
                        }}
                    </p>
                    <a href=move || format!("/archive{}", queries.get().to_query_string())>
                        {t!(i18n, archive.permalink)}
                    </a>
                </div>
            </Show>
            <div class="space-y-2">
                <Show when=move || has_searched.get()>
                    <div class="flex flex-col">
                        <div class="min-h-0 rounded-lg sm:grid sm:grid-cols-2 sm:content-start lg:grid-cols-3">
                            <For
                                each=move || games.get()
                                key=|game| game.game_id.clone()
                                let:game
                            >
                                <GameRow game />
                            </For>
                        </div>
                        <Show when=move || games.with(|g| g.is_empty()) && !is_loading.get()>
                            <p class="mt-4 text-sm text-gray-600 dark:text-gray-400">
                                {t!(i18n, archive.no_games_found)}
                            </p>
                        </Show>
                        <Show when=is_loading>
                            <p class="mt-4 text-sm text-gray-600 dark:text-gray-400">
                                {t!(i18n, archive.loading_games)}
                            </p>
                        </Show>
                    </div>
                </Show>
            </div>
        </div>
    }
}
