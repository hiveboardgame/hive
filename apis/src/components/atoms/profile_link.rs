use crate::components::molecules::hover_ratings::HoverRating;
use crate::responses::UserResponse;
use leptos::prelude::*;
use leptos::html;
use leptos_icons::*;

#[component]
pub fn ProfileLink(
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] user_is_hoverable: MaybeProp<UserResponse>,
    username: String,
    patreon: bool,
    bot: bool,
) -> impl IntoView {
    let profile_link = format!("/@/{username}");
    let hover_show = RwSignal::new(false);
    let patreon = RwSignal::new(patreon);
    let bot = RwSignal::new(bot);
    let link_ref = NodeRef::<html::A>::new();
    view! {
        <div class="relative w-full">
            <a
                class="z-20 font-bold duration-300 no-link-style hover:text-pillbug-teal"
                node_ref=link_ref
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
                    </Show> <Show when=bot>
                        <Icon icon=icondata::MdiRobotHappy attr:class="w-3 h-3" />
                    </Show>
                </div>
            </a>
            <Show when=move || user_is_hoverable().is_some() && hover_show()>
                <HoverRating
                    user=user_is_hoverable().expect("Showing because it's some")
                    anchor_ref=link_ref
                />
            </Show>
        </div>
    }
}
