use crate::providers::NotificationContext;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;

#[component]
pub fn TournamentStatusNotification(
    tournament_id: TournamentId,
    tournament_name: String,
    finished: bool,
) -> impl IntoView {
    let notifications = expect_context::<NotificationContext>();
    let tournament_id = StoredValue::new(tournament_id);
    let div_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let notification_text = format!(
        "Tournament: {} {}",
        tournament_name,
        if !finished { "started" } else { "finished" }
    );
    let dismiss = move |_| {
        if !finished {
            notifications.tournament_started.update(|t| {
                t.remove(&tournament_id.get_value());
            });
        } else {
            notifications.tournament_finished.update(|t| {
                t.remove(&tournament_id.get_value());
            });
        }
    };

    view! {
        <div class="flex items-center justify-between text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light w-full p-2">
            <div class="flex relative flex-grow">
                <div class=div_class>
                    <div>{notification_text}</div>
                </div>
                <a
                    class="absolute top-0 left-0 z-10 w-full h-full"
                    href=format!("/tournament/{}", &tournament_id.get_value())
                ></a>
            </div>
            <div>
                <button
                    title="Dismiss"
                    on:click=dismiss
                    class="z-20 p-1 text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95"
                >
                    <Icon icon=icondata::IoCloseSharp attr:class="w-6 h-6" />
                </button>
            </div>
        </div>
    }
}
