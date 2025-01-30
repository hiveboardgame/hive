use leptos::prelude::*;

#[component]
pub fn GameType(game_type: String) -> impl IntoView {
    view! {
        <div class="flex justify-center">
            {if game_type == "Base" {
                view! { "—" }.into_any()
            } else {
                view! {
                    <img
                        width="100%"
                        height="100%"
                        src="/assets/plm.svg"
                        alt="plm"
                        class="w-14 lg:w-20"
                    />
                }
                    .into_any()
            }}

        </div>
    }
}
