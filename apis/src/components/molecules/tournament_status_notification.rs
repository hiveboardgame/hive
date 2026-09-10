use crate::{i18n::*, providers::NotificationContext};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;

#[component]
pub fn TournamentStatusNotification(
    tournament_id: TournamentId,
    tournament_name: String,
    finished: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let notifications = expect_context::<NotificationContext>();
    let tournament_id = StoredValue::new(tournament_id);
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
        <div class="ui-notification-item">
            <div class="relative flex-1 min-w-0">
                <div class="ui-notification-label">
                    {t!(i18n, notifications.tournament_status.label)}
                </div>
                <div class="ui-notification-title">{tournament_name}</div>
                <div class="ui-notification-meta">
                    {if finished {
                        t_string!(i18n, notifications.tournament_status.finished).to_string()
                    } else {
                        t_string!(i18n, notifications.tournament_status.started).to_string()
                    }}
                </div>
                <a
                    class="absolute top-0 left-0 z-10 size-full"
                    href=format!("/tournament/{}", &tournament_id.get_value())
                ></a>
            </div>
            <button
                title=move || t_string!(i18n, notifications.actions.dismiss).to_string()
                on:click=dismiss
                class="z-20 ui-button ui-button-danger ui-button-icon"
            >
                <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
            </button>
        </div>
    }
}
