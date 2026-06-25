use crate::{components::molecules::hover_ratings::HoverRating, i18n::*, responses::UserResponse};
use leptos::{html, prelude::*};
use leptos_icons::*;

#[component]
pub fn ProfileLink(
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional, default = "w-full")] wrapper_tw_classes: &'static str,
    #[prop(optional)] user_is_hoverable: MaybeProp<UserResponse>,
    deleted: bool,
    username: String,
    patreon: bool,
    bot: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let name_classes = StoredValue::new(format!(
        "inline-flex min-w-0 max-w-full items-center text-xs {extend_tw_classes}"
    ));
    let wrapper_classes = StoredValue::new(format!("relative {wrapper_tw_classes}"));
    let username = StoredValue::new(username);
    let profile_link = StoredValue::new(format!("/@/{}", username.get_value()));
    let hover_show = RwSignal::new(false);
    let patreon = RwSignal::new(patreon);
    let bot = RwSignal::new(bot);
    let link_ref = NodeRef::<html::A>::new();
    view! {
        <div class=wrapper_classes.get_value()>
            <Show
                when=move || !deleted
                fallback=move || {
                    view! {
                        <span class="z-20 font-bold no-link-style">
                            <span class=name_classes
                                .get_value()>{t!(i18n, profile.deleted_user)}</span>
                        </span>
                    }
                }
            >
                <a
                    class="inline-flex z-20 min-w-0 max-w-full font-bold w-fit no-link-style hover:text-pillbug-teal"
                    node_ref=link_ref
                    on:mouseover=move |_| {
                        if user_is_hoverable.with(|u| u.is_some()) {
                            hover_show.set(true);
                        }
                    }

                    on:mouseleave=move |_| hover_show.set(false)
                    href=move || profile_link.get_value()
                >
                    <span class=name_classes
                        .get_value()>
                        {username.get_value()} <Show when=patreon>
                            <Icon icon=icondata_lu::LuCrown attr:class="size-2" />
                        </Show> <Show when=bot>
                            <span class="ml-1 text-[80%]">BOT</span>
                        </Show>
                    </span>
                </a>
                <Show when=move || user_is_hoverable.with(|u| u.is_some()) && hover_show()>
                    <HoverRating
                        user=user_is_hoverable().expect("Showing because it's some")
                        anchor_ref=link_ref
                    />
                </Show>
            </Show>
        </div>
    }
}
