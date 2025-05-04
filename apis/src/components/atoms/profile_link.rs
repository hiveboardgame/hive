use crate::components::molecules::hover_ratings::HoverRating;
use crate::responses::UserResponse;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn ProfileLink(
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] user_is_hoverable: MaybeProp<UserResponse>,
    username: String,
    patreon: bool,
) -> impl IntoView {
    let profile_link = format!("/@/{}", username);
    let hover_show = RwSignal::new(false);
    let patreon = RwSignal::new(patreon);
    view! {
        <div class="relative w-full">
            <a
                class="z-20 font-bold duration-300 no-link-style hover:text-pillbug-teal"
                on:mouseover=move |_| {
                    if user_is_hoverable().is_some() {
                        hover_show.set(true);
                    }
                }

                on:mouseleave=move |_| hover_show.set(false)
                href=profile_link
            >
                <div class=format!(
                    "flex {}",
                    extend_tw_classes,
                )>
                    {username} <Show when=patreon>
                        <Icon icon=icondata::LuCrown attr:class="w-2 h-2" />
                    </Show>
                </div>
            </a>
            <Show when=move || user_is_hoverable().is_some() && hover_show()>
                <HoverRating user=user_is_hoverable().expect("Showing because it's some") />
            </Show>
        </div>
    }
}
