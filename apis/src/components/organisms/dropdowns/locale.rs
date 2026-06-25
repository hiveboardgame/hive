use crate::{
    components::molecules::hamburger::Hamburger,
    functions::accounts::edit::edit_lang,
    i18n::*,
    providers::AuthContext,
};
use leptos::{prelude::*, task::spawn_local};

#[component]
pub fn LocaleDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |locale: Locale| {
        use_i18n().set_locale(locale);
        hamburger_show.update(|b| *b = false);
        let logged_in = expect_context::<AuthContext>()
            .user
            .with_untracked(|u| u.is_some());
        if logged_in {
            let code = locale.to_string();
            spawn_local(async move {
                let _ = edit_lang(code).await;
            });
        }
    };
    let i18n = use_i18n();
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-dropdown-button"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-left ui-header-dropdown-menu"
            id="locale_dropdown"
            content=move || i18n.get_locale().to_string()
        >
            <For each=Locale::get_all key=move |locale| locale.to_string() let:locale>
                <a class="ui-dropdown-link" on:click=move |_| onclick_close(*locale)>
                    {locale.to_string()}
                </a>
            </For>
        </Hamburger>
    }
}
