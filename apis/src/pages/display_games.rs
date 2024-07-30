use html::Div;
use leptos::*;
use crate::{
    functions::users::get::get_finished_games_in_batches, pages::profile_view::ProfileGamesContext,
    components::molecules::game_row::GameRow,
    components::organisms::display_profile::DisplayProfile,
};
use super::profile_view::ProfileGamesView;
use leptos_router::A;
use leptos_use::{use_infinite_scroll_with_options, UseInfiniteScrollOptions};

#[component]
pub fn DisplayGames(tab_view: ProfileGamesView) -> impl IntoView {
    let ctx = expect_context::<ProfileGamesContext>();
    let el = create_node_ref::<Div>();
    let active = move |view: ProfileGamesView| {
        let button_style = String::from("hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded");
        if tab_view == view {
            button_style + " bg-pillbug-teal"
        } else {
            button_style + " bg-button-dawn dark:bg-button-twilight"
        }
    };
    let username = store_value(ctx.user.username.clone());
    let _ = use_infinite_scroll_with_options(
        el,
        move |_| async move {
            if tab_view == ProfileGamesView::Finished && ctx.more_finished.get() {
                let games = get_finished_games_in_batches(
                    username(),
                    ctx.finished_last_timestamp.get(),
                    ctx.finished_last_id.get(),
                    5,
                )
                .await;
                if let Ok((g, are_more)) = games {
                    ctx.finished_last_timestamp
                        .update(|v| *v = g.last().map(|gr| gr.updated_at));
                    ctx.finished_last_id
                        .update(|v| *v = g.last().map(|gr| gr.uuid));
                    ctx.more_finished.set(are_more);
                    ctx.finished.update(|v| v.extend(g.clone()));
                }
            }
        },
        UseInfiniteScrollOptions::default().distance(10.0),
    );
    view! {
        <div class="flex flex-col w-full">
            <DisplayProfile user=store_value(ctx.user.clone())/>
            <div class="flex gap-1 ml-3">
                <Show when=move || !ctx.unstarted.get().is_empty()>
                    <A
                        href=format!("/@/{}/unstarted", username())
                        class=move || active(ProfileGamesView::Unstarted)
                    >
                        "Unstarted Tournament Games"
                    </A>
                </Show>
                <Show when=move || !ctx.playing.get().is_empty()>
                    <A
                        href=format!("/@/{}/playing", username())
                        class=move || active(ProfileGamesView::Playing)
                    >
                        "Playing "
                    </A>
                </Show>
                <Show when=move || !ctx.finished.get().is_empty()>
                    <A
                        href=format!("/@/{}/finished", username())
                        class=move || active(ProfileGamesView::Finished)
                    >
                        "Finished Games "
                    </A>
                </Show>
            </div>
            <div node_ref=el class="flex flex-col overflow-x-hidden items-center h-[72vh]">
                <For
                    each=move || match tab_view {
                        ProfileGamesView::Finished => ctx.finished.get(),
                        ProfileGamesView::Playing => ctx.playing.get(),
                        ProfileGamesView::Unstarted => ctx.unstarted.get(),
                    }

                    key=|game| (game.uuid)
                    let:game
                >
                    <GameRow game=store_value(game)/>
                </For>
            </div>
        </div>
    }
}
