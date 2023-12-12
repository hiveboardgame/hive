use leptos::*;
use leptos_icons::{ChIcon::ChCross, Icon};

#[component]
pub fn AcceptDenyGc(icon: Icon, red: bool, action: Callback<()>) -> impl IntoView {
    let active_color = move || {
        if red {
            "bg-red-700 hover:bg-red-500 absolute"
        } else {
            "mr-1 bg-green-700 hover:bg-green-500 relative"
        }
    };
    let on_click = move |_| {
        action(());
    };
    view! {
        <button
            on:click=on_click
            class=move || {
                format!("aspect-square hover:bg-green-500 rounded-sm relative {}", active_color())
            }
        >

            <Icon icon=icon class="h-[2vw] w-[2vw]"/>
        </button>
    }
}

#[component]
pub fn ConfirmButton(icon: Icon, action: Callback<()>) -> impl IntoView {
    let is_clicked = RwSignal::new(false);
    let onclick_confirm = move |_| {
        if is_clicked() {
            action(());
            is_clicked.update(|v| *v = false);
        } else {
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
                        "aspect-square hover:bg-green-500 rounded-sm relative {}",
                        active_color(),
                    )
                }
            >

                <Icon icon=icon class="h-[2vw] w-[2vw]"/>

            </button>
            <Show when=is_clicked>
                <button
                    on:click=cancel
                    class="ml-1 aspect-square bg-red-700 hover:bg-green-500 rounded-sm absolute"
                >

                    <Icon icon=Icon::from(ChCross) class="h-[2vw] w-[2vw]"/>
                </button>
            </Show>
        </div>
    }
}
