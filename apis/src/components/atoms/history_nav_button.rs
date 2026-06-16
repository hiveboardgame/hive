use leptos::{leptos_dom::helpers::debounce, prelude::*};

#[component]
pub fn HistoryNavButton<F>(disabled: F, on_press: Callback<()>, children: Children) -> impl IntoView
where
    F: Fn() -> bool + Send + 'static,
{
    let nav_buttons_style = "flex place-items-center justify-center hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 m-1 h-7 rounded-md border-cyan-500 dark:border-button-twilight border-2 drop-shadow-lg disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent";
    let debounced_action = debounce(std::time::Duration::from_millis(10), move |_| {
        on_press.run(())
    });

    view! {
        <button class=nav_buttons_style prop:disabled=disabled on:click=debounced_action>
            {children()}
        </button>
    }
}
