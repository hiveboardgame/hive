use crate::i18n::*;
use leptos::prelude::*;
const ALLOWED_LOCALES: [&str; 6] = ["en", "es", "ca", "it", "fr", "pt"];

#[component]
pub fn RulesSummary() -> impl IntoView {
    let i18n = use_i18n();
    let is_allowed = Signal::derive(move || ALLOWED_LOCALES.contains(&i18n.get_locale().as_str()));
    view! {
        <div class="pt-20">
            <div class="px-4 mx-auto max-w-4xl sm:px-6 lg:px-8">
                <Show
                    when=is_allowed
                    fallback=|| {
                        view! { <p>"Rules Summary not available in this language (yet)"</p> }
                    }
                >

                    <img
                        src=Signal::derive(move || {
                            let locale = i18n.get_locale().to_string();
                            format!("/assets/rules_summary/{locale}.png")
                        })

                        alt="Rules Summary"
                        class="w-full"
                    />
                </Show>
            </div>
        </div>
    }
}
