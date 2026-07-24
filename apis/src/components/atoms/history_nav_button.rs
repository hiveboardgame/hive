use leptos::prelude::*;

#[component]
pub fn HistoryNavButton<F>(
    disabled: F,
    on_press: Callback<()>,
    #[prop(optional)] on_pointerdown: Option<Callback<()>>,
    children: Children,
) -> impl IntoView
where
    F: Fn() -> bool + Send + 'static,
{
    let press = move |_| on_press.run(());
    let mark_pointerdown = move |_| {
        if let Some(on_pointerdown) = on_pointerdown {
            on_pointerdown.run(());
        }
    };

    view! {
        <button
            class="ui-board-nav-button"
            prop:disabled=disabled
            on:pointerdown=mark_pointerdown
            on:click=press
        >
            {children()}
        </button>
    }
}
