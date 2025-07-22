use crate::providers::Config;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn SoundToggle() -> impl IntoView {
    let Config(config, set_cookie) = expect_context();
    let icon = move || {
        let icon = if config().prefers_sound {
            icondata_bi::BiVolumeFullRegular
        } else {
            icondata_bi::BiVolumeMuteRegular
        };
        view! { <Icon icon attr:class="w-4 h-4" /> }
    };
    view! {
        <div class="inline-flex justify-center items-center m-1 rounded">
            <button
                class="flex justify-center items-center px-1 py-2 w-full h-full"
                on:click=move |_| {
                    set_cookie
                        .update(|c| {
                            if let Some(cookie) = c {
                                cookie.prefers_sound = !cookie.prefers_sound;
                            }
                        });
                }
            >

                {icon}
            </button>
        </div>
    }
}
