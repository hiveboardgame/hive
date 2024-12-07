use crate::providers::NotificationContext;
use crate::responses::TournamentResponse;
use leptos::*;
use leptos_icons::*;
use shared_types::TournamentStatus;

#[component]
pub fn TournamentStatusNotification(tournament: StoredValue<TournamentResponse>) -> impl IntoView {
    let div_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let tournament = tournament();
    let started = tournament.status == TournamentStatus::InProgress;
    let tournament_id = StoredValue::new(tournament.tournament_id);
    let notification_text = move || {
        format!(
            "Tournament: {} {}",
            tournament.name,
            if started { "started" } else { "finished" }
        )
    };
    let dismiss = move |_| {
        let notifications = expect_context::<NotificationContext>();
        if started {
            notifications.tournament_started.update(|t| {
                t.remove(&tournament_id());
            });
        } else {
            notifications.tournament_finished.update(|t| {
                t.remove(&tournament_id());
            });
        }
    };

    view! {
        <div class="flex items-center text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <div class="flex relative">
                <div class=div_class>
                    <div>{notification_text}</div>
                </div>
                <a
                    class="absolute top-0 left-0 z-10 w-full h-full"
                    href=format!("/tournament/{}", &tournament_id())
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
