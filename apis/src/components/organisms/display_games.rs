use crate::{
    components::molecules::game_row::GameRow,
    providers::{
        calculate_initial_batch_size,
        initial_profile_filters_for_tab,
        load_games,
        GamesSearchContext,
    },
};
use leptos::prelude::*;
use leptos_router::{hooks::use_params, params::Params};
use leptos_use::{
    use_element_bounding,
    use_infinite_scroll_with_options,
    watch_throttled_with_options,
    UseInfiniteScrollOptions,
    WatchThrottledOptions,
};
use shared_types::GameProgress;

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

#[component]
pub fn DisplayGames(tab_view: GameProgress) -> impl IntoView {
    let ctx = expect_context::<GamesSearchContext>();
    let params = use_params::<UsernameParams>();
    let username = Signal::derive(move || {
        params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default())
    });
    let el = ctx.games_container_ref;
    Effect::watch(
        move || (),
        move |_, _, _| {
            request_animation_frame(move || {
                request_animation_frame(move || {
                    let effective_filters = initial_profile_filters_for_tab(
                        tab_view,
                        ctx.get_filter_cookie.get_untracked(),
                    );

                    ctx.filters.set(effective_filters.clone());
                    ctx.pending.set(effective_filters.clone());
                    ctx.has_more.set_value(true);
                    ctx.is_first_batch.set_value(true);
                    ctx.next_batch_token.set(None);
                    ctx.games.set(vec![]);

                    load_games(
                        effective_filters,
                        tab_view,
                        username.get_untracked(),
                        None,
                        ctx.next_batch,
                        ctx.initial_batch_size.get_untracked(),
                    );
                });
            });
        },
        true,
    );

    let bounding = use_element_bounding(el);
    let _ = watch_throttled_with_options(
        bounding.height,
        move |new_height, old_height, _| {
            let new_height = *new_height;
            let old_height = old_height.copied().unwrap_or(new_height);

            if new_height > old_height + 50.0
                && ctx.has_more.get_value()
                && !ctx.next_batch.pending().get_untracked()
            {
                let current_games_count = ctx.games.with_untracked(|games| games.len());
                let container_width = bounding.width.get_untracked();
                let needed_games = calculate_initial_batch_size(new_height, container_width);

                if current_games_count < needed_games {
                    let filters = ctx.filters.get_untracked();
                    let username = username.get_untracked();
                    let batch_token = ctx.next_batch_token.get_untracked();
                    ctx.is_first_batch.set_value(false);
                    load_games(
                        filters,
                        tab_view,
                        username,
                        batch_token,
                        ctx.next_batch,
                        ctx.infinite_scroll_batch_size.get_untracked(),
                    );
                }
            }
        },
        200.0,
        WatchThrottledOptions::default()
            .immediate(false)
            .trailing(true),
    );

    let _ = use_infinite_scroll_with_options(
        el,
        move |_| {
            let filters = ctx.filters.get();
            let username = username();
            let batch_token = ctx.next_batch_token.get();
            ctx.is_first_batch.set_value(batch_token.is_none());
            async move {
                if !ctx.has_more.get_value() || ctx.next_batch.pending().get() {
                    return;
                }
                load_games(
                    filters,
                    tab_view,
                    username,
                    batch_token,
                    ctx.next_batch,
                    ctx.infinite_scroll_batch_size.get(),
                );
            }
        },
        UseInfiniteScrollOptions::default()
            .distance(10.0)
            .interval(300.0),
    );
    view! {
        <div
            node_ref=el
            class="overflow-y-auto overflow-x-hidden h-full sm:grid sm:grid-cols-2 sm:content-start lg:grid-cols-3"
        >
            {move || {
                ctx.games.get().into_iter().map(|game| view! { <GameRow game /> }).collect_view()
            }}
        </div>
    }
}
