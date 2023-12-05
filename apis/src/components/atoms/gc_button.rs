use leptos::*;
use leptos_icons::Icon;

// TODO: @ion feel free to refactor this away
#[component]
pub fn GcButton(icon: Icon, red: bool, action: Callback<()>) -> impl IntoView {
    let active_color = move || {
        if red {
            "bg-red-700 hover:bg-red-500"
        } else {
            "bg-green-700 hover:bg-green-500"
        }
    };
    let on_click = move |_| {
        action(());
    };
    view! {
        <div class="relative">
            <button
                on:click=on_click
                class=move || {
                    format!(
                        "aspect-square max-h-fit max-w-fit hover:bg-green-500 rounded-sm relative {}",
                        active_color(),
                    )
                }
            >
                <Icon icon=icon class="h-[2vw] w-[2vw]"/>
            </button>
        </div>
    }
}
