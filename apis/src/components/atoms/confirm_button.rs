use leptos::logging::log;
use leptos::*;
use leptos_icons::{ChIcon::ChCross, Icon};

#[component]
pub fn ConfirmButton(icon: Icon, action: Callback<()>) -> impl IntoView {
    let is_clicked = RwSignal::new(false);
    let onclick_confirm = move |_| {
        if is_clicked() {
            log!("Confirming click ");
            action(());
            is_clicked.update(|v| *v = false);
        } else {
            log!("first click");
            is_clicked.update(|v| *v = true);
        }
    };
    let cancel = move |_| is_clicked.update(|v| *v = false);
    let active_color = move || {
        if is_clicked() {
            "bg-red-700 hover:bg-red-500"
        } else {
            ""
        }
    };
    view! {
        <div class="relative">
            <button
                on:click=onclick_confirm
                class=move || {
                    format!(
                        "aspect-square max-h-fit max-w-fit hover:bg-green-500 rounded-sm relative {}",
                        active_color(),
                    )
                }
            >

                <Icon icon=icon class="h-[2vw] w-[2vw]"/>

            </button>
            <Show when=is_clicked>
                <button
                    on:click=cancel
                    class="aspect-square max-h-fit max-w-fit bg-red-700 hover:bg-green-500 rounded-sm absolute drop-shadow-md"
                >

                    <Icon icon=Icon::from(ChCross) class="h-[2vw] w-[2vw]"/>
                </button>
            </Show>
        </div>
    }
}
