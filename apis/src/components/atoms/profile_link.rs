use crate::components::molecules::hover_ratings::HoverRating;
use crate::responses::UserResponse;
use leptos::html;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn ProfileLink(
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] user_is_hoverable: MaybeProp<UserResponse>,
    #[prop(optional)] use_default_style: Option<bool>,
    username: String,
    patreon: bool,
    bot: bool,
) -> impl IntoView {
    let profile_link = format!("/@/{username}");
    let hover_show = RwSignal::new(false);
    let patreon = RwSignal::new(patreon);
    let bot = RwSignal::new(bot);
    let link_ref = NodeRef::<html::A>::new();
    const DEFAULT_STYLE: &str = "flex text-xs";
    let ds = if use_default_style.unwrap_or(true) {
        DEFAULT_STYLE
    } else {
        ""
    };
    view! {
        <div class="relative w-full">
            <a
                class="z-20 font-bold duration-300 no-link-style hover:text-pillbug-teal"
                node_ref=link_ref
                on:mouseover=move |_| {
                    if user_is_hoverable.with(|u| u.is_some()) {
                        hover_show.set(true);
                    }
                }

                on:mouseleave=move |_| hover_show.set(false)
                href=profile_link
            >
                <div class=format!(
                    "{} {}",
                    ds,
                    extend_tw_classes,
                )>
                    {username} <Show when=patreon>
                        <Icon icon=icondata_lu::LuCrown attr:class="size-2" />
                    </Show> <Show when=bot>
                        <Icon icon=icondata_mdi::MdiRobotHappy attr:class="size-3" />
                    </Show>
                </div>
            </a>
            <Show when=move || user_is_hoverable.with(|u| u.is_some()) && hover_show()>
                <HoverRating
                    user=user_is_hoverable().expect("Showing because it's some")
                    anchor_ref=link_ref
                />
            </Show>
        </div>
    }
}
