use crate::{
    components::{
        layouts::base_layout::{COMMON_LINK_STYLE, DROPDOWN_BUTTON_STYLE, DROPDOWN_MENU_STYLE},
        molecules::hamburger::Hamburger,
    },
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
            button_style=DROPDOWN_BUTTON_STYLE
            dropdown_style=DROPDOWN_MENU_STYLE
            id="locale_dropdown"
            content=move || i18n.get_locale().to_string()
        >
            <For each=Locale::get_all key=move |locale| locale.to_string() let:locale>
                <a class=COMMON_LINK_STYLE on:click=move |_| onclick_close(*locale)>
                    {locale.to_string()}
                </a>
            </For>
        </Hamburger>
    }
}
