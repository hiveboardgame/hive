use crate::providers::NotificationContext;
use crate::responses::TournamentResponse;
use leptos::*;
use leptos_icons::*;

#[component]
pub fn TournamentStartedNotification(tournament: StoredValue<TournamentResponse>) -> impl IntoView {
    let div_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";

    let dismiss = move |_| {
        let notifications = expect_context::<NotificationContext>();
        notifications.tournament_started.update(|t| {
            t.remove(&tournament().tournament_id);
        });
    };

    view! {
        <div class="flex items-center text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <div class="flex relative">
                <div class=div_class>
                    <div>{tournament().name} " started"</div>
                </div>
                <a
                    class="absolute top-0 left-0 z-10 w-full h-full"
                    href=format!("/tournament/{}", tournament().tournament_id)
                ></a>
            </div>
            <div class=div_class>
                <button
                    title="Dismiss"
                    on:click=dismiss
                    class="z-20 p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95"
                >
                    <Icon icon=icondata::IoCloseSharp class="w-6 h-6" />
                </button>
            </div>
        </div>
    }
}
