use crate::providers::Config;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn SoundToggle() -> impl IntoView {
    let config = expect_context::<Config>().0;
    let (_, set_cookie) = Config::get_cookie();
    let icon = move || {
        let icon = if config().prefers_sound {
            icondata::BiVolumeFullRegular
        } else {
            icondata::BiVolumeMuteRegular
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
                                cookie.prefers_sound = !config().prefers_sound;
                            }
                        });
                }
            >

                {icon}
            </button>
        </div>
    }
}
