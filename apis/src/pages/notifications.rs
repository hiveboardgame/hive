// Notification preferences page at /notifications.
//
// Loads the current row via `get_notification_preferences`, keeps a local
// signal per editable field, and writes back via `set_notification_preferences`
// when the user clicks Save. No save-on-toggle: settings pages benefit from
// an explicit commit point — and we avoid debouncing N concurrent flips into
// the wrong final state.
//
// Quiet hours captured but not yet enforced (banner makes that explicit) —
// columns exist server-side, enforcement is a separate dispatcher pass and
// lives behind a "do later" item in the Phase 2 plan.

use crate::{
    components::atoms::simple_switch::SimpleSwitch,
    functions::notification_preferences::{
        get_notification_preferences,
        set_notification_preferences,
    },
    responses::NotificationPreferencesResponse,
};
use leptos::prelude::*;

const SECTION_CARD: &str = "px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg \
                            bg-stone-300 border-stone-400 dark:bg-slate-800 dark:border-slate-600";
const HEADING: &str = "mb-4 text-xl font-bold text-center text-indigo-600 dark:text-indigo-400";

/// Event rows rendered in the channel matrix. Tuple is (display label,
/// internal pref-field name). General chat is intentionally absent — the
/// chat revamp is happening on a separate branch and surfacing the toggle
/// before that lands would create migration noise.
const EVENTS: &[(&str, EventField)] = &[
    ("Your turn", EventField::YourTurn),
    ("Challenge received", EventField::Challenges),
    ("Game ended", EventField::GameEnded),
    ("Tournament invite", EventField::Tournament),
    ("Direct message", EventField::Dms),
];

#[derive(Debug, Clone, Copy)]
enum EventField {
    YourTurn,
    Challenges,
    GameEnded,
    Tournament,
    Dms,
}

#[component]
pub fn Notifications() -> impl IntoView {
    // Per-field local state. Seeded from the Resource the first time it
    // resolves; the Save action writes them back atomically.
    let your_turn = RwSignal::new(Vec::<String>::new());
    let challenges = RwSignal::new(Vec::<String>::new());
    let game_ended = RwSignal::new(Vec::<String>::new());
    let tournament = RwSignal::new(Vec::<String>::new());
    let dms = RwSignal::new(Vec::<String>::new());
    let quiet_start = RwSignal::new(None::<i16>);
    let quiet_end = RwSignal::new(None::<i16>);
    let timezone = RwSignal::new(String::new());
    let loaded = RwSignal::new(false);

    let prefs = OnceResource::new(get_notification_preferences());

    Effect::new(move |_| {
        if let Some(Ok(p)) = prefs.get() {
            your_turn.set(p.your_turn);
            challenges.set(p.challenges);
            game_ended.set(p.game_ended);
            tournament.set(p.tournament);
            dms.set(p.dms);
            quiet_start.set(p.quiet_start);
            quiet_end.set(p.quiet_end);
            timezone.set(p.timezone.unwrap_or_default());
            loaded.set(true);
        }
    });

    let save = Action::new(move |_: &()| {
        let payload = NotificationPreferencesResponse {
            your_turn: your_turn.get(),
            challenges: challenges.get(),
            game_ended: game_ended.get(),
            tournament: tournament.get(),
            dms: dms.get(),
            quiet_start: quiet_start.get(),
            quiet_end: quiet_end.get(),
            timezone: {
                let tz = timezone.get();
                if tz.is_empty() {
                    None
                } else {
                    Some(tz)
                }
            },
        };
        async move { set_notification_preferences(payload).await }
    });

    let saving = save.pending();
    let save_result = save.value();
    let save_message = move || {
        match save_result.get() {
        Some(Ok(_)) => Some(view! { <p class="text-green-600 dark:text-green-400">"Saved."</p> }.into_any()),
        Some(Err(e)) => Some(
            view! { <p class="text-red-600 dark:text-red-400">"Failed to save: " {e.to_string()}</p> }
                .into_any(),
        ),
        None => None,
    }
    };

    view! {
        <div class="px-4 pb-20 mx-auto max-w-md pt-page">
            <Suspense fallback=move || view! { <p class="dark:text-white">"Loading…"</p> }>
                <Show when=move || loaded.get()>
                    <div class=SECTION_CARD>
                        <h2 class=HEADING>"🔔 Notifications"</h2>
                        <p class="mb-3 text-sm text-gray-700 dark:text-gray-300">
                            "Choose how you want to be notified for each kind of event. \
                             Push goes to the HiveGame mobile app. Discord requires linking \
                             your Discord account on the " <a href="/account" class="underline">
                                "account page"
                            </a> "."
                        </p>
                        <div class="grid grid-cols-3 gap-y-3 items-center">
                            <div></div>
                            <div class="text-sm font-semibold text-center dark:text-white">
                                "Push"
                            </div>
                            <div class="text-sm font-semibold text-center dark:text-white">
                                "Discord"
                            </div>
                            {EVENTS
                                .iter()
                                .map(|(label, field)| {
                                    let signal = match field {
                                        EventField::YourTurn => your_turn,
                                        EventField::Challenges => challenges,
                                        EventField::GameEnded => game_ended,
                                        EventField::Tournament => tournament,
                                        EventField::Dms => dms,
                                    };
                                    view! {
                                        <div class="text-sm dark:text-white">{*label}</div>
                                        <div class="flex justify-center">
                                            <ChannelSwitch signal=signal channel="push" />
                                        </div>
                                        <div class="flex justify-center">
                                            <ChannelSwitch signal=signal channel="discord" />
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <p class="mt-4 text-xs italic text-gray-600 dark:text-gray-400">
                            "Email delivery is coming soon."
                        </p>
                    </div>

                    <div class=SECTION_CARD>
                        <h2 class=HEADING>"🌙 Quiet hours"</h2>
                        <div class="p-3 mb-4 text-sm text-amber-700 bg-amber-50 rounded-lg border border-amber-200 dark:text-amber-300 dark:border-amber-700 dark:bg-amber-900/30">
                            "Captured but not yet enforced — pushes still fire 24/7 for now. \
                             Your saved hours will start applying once enforcement ships."
                        </div>
                        <div class="grid grid-cols-2 gap-3">
                            <label class="text-sm dark:text-white">
                                "Start hour (0-23)"
                                <input
                                    class="block py-1 px-2 mt-1 w-full rounded dark:bg-gray-700 bg-stone-100"
                                    type="number"
                                    min="0"
                                    max="23"
                                    prop:value=move || {
                                        quiet_start.get().map(|h| h.to_string()).unwrap_or_default()
                                    }
                                    on:input=move |ev| {
                                        let raw = event_target_value(&ev);
                                        quiet_start
                                            .set(
                                                raw.parse::<i16>().ok().filter(|h| (0..24).contains(h)),
                                            );
                                    }
                                />
                            </label>
                            <label class="text-sm dark:text-white">
                                "End hour (0-23)"
                                <input
                                    class="block py-1 px-2 mt-1 w-full rounded dark:bg-gray-700 bg-stone-100"
                                    type="number"
                                    min="0"
                                    max="23"
                                    prop:value=move || {
                                        quiet_end.get().map(|h| h.to_string()).unwrap_or_default()
                                    }
                                    on:input=move |ev| {
                                        let raw = event_target_value(&ev);
                                        quiet_end
                                            .set(
                                                raw.parse::<i16>().ok().filter(|h| (0..24).contains(h)),
                                            );
                                    }
                                />
                            </label>
                        </div>
                        <label class="block mt-3 text-sm dark:text-white">
                            "Timezone (IANA, e.g. Europe/Berlin)"
                            <input
                                class="block py-1 px-2 mt-1 w-full rounded dark:bg-gray-700 bg-stone-100"
                                type="text"
                                prop:value=move || timezone.get()
                                on:input=move |ev| timezone.set(event_target_value(&ev))
                            />
                        </label>
                    </div>

                    <div class="flex justify-center">
                        <button
                            class="py-2 px-6 font-bold text-white rounded shadow disabled:opacity-50 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
                            disabled=move || saving.get()
                            on:click=move |_| {
                                save.dispatch(());
                            }
                        >
                            {move || if saving.get() { "Saving…" } else { "Save" }}
                        </button>
                    </div>
                    <div class="mt-3 text-center">{save_message}</div>
                </Show>
            </Suspense>
        </div>
    }
}

#[component]
fn ChannelSwitch(signal: RwSignal<Vec<String>>, channel: &'static str) -> impl IntoView {
    // Local mirror signal — SimpleSwitch flips a bool, we translate that
    // back into the array shape the API expects on save.
    let checked = RwSignal::new(signal.with(|v| v.iter().any(|c| c == channel)));
    Effect::new(move |_| {
        let present = signal.with(|v| v.iter().any(|c| c == channel));
        if present != checked.get_untracked() {
            checked.set(present);
        }
    });
    let on_change = Callback::new(move |_| {
        signal.update(|v| {
            let present = v.iter().any(|c| c == channel);
            if present {
                v.retain(|c| c != channel);
            } else {
                v.push(channel.to_string());
            }
        });
    });
    view! { <SimpleSwitch checked=checked optional_action=on_change /> }
}
