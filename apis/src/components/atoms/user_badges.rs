use crate::i18n::*;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn UserBadges(patreon: bool, bot: bool, chat_muted: bool) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Show when=move || patreon>
            <Icon icon=icondata_lu::LuCrown attr:class="size-2 shrink-0 self-start" />
        </Show>
        <Show when=move || bot>
            <span class="ml-1 text-[80%]">{t!(i18n, common.bot)}</span>
        </Show>
        <Show when=move || chat_muted>
            <span
                class="inline-flex relative z-20 items-center ml-1 text-red-600 dark:text-red-400 shrink-0"
                title=move || t_string!(i18n, common.muted_by_admin).to_string()
                aria-label=move || t_string!(i18n, common.muted_by_admin).to_string()
                role="img"
            >
                <Icon
                    icon=icondata_lu::LuMessageSquareOff
                    attr:class="size-3"
                    attr:aria-hidden="true"
                />
            </span>
        </Show>
    }
}
