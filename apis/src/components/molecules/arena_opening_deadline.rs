use crate::{
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        game_state::{GameStateStore, GameStateStoreFields},
        AuthContext,
        Config,
        SoundType,
        Sounds,
    },
};
use chrono::{DateTime, Utc};
use hive_lib::Color;
use leptos::prelude::*;
use shared_types::{GameId, GameStart};

fn opening_side(turn: usize) -> Color {
    if turn.is_multiple_of(2) {
        Color::White
    } else {
        Color::Black
    }
}

fn seconds_until(deadline: DateTime<Utc>, now: DateTime<Utc>) -> i64 {
    deadline.signed_duration_since(now).num_seconds().max(0)
}

fn deadline_is_urgent(deadline: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    deadline.signed_duration_since(now).num_milliseconds() < 8_000
}

#[component]
pub fn ArenaOpeningDeadline(#[prop(optional)] class: &'static str) -> impl IntoView {
    let i18n = use_i18n();
    let game_state = expect_context::<GameStateStore>();
    let auth_context = expect_context::<AuthContext>();
    let sounds = expect_context::<Sounds>();
    let config = expect_context::<Config>().0;
    let background_style = move || {
        let background = config.with(|config| {
            config
                .tile
                .get_effective_background_color(config.prefers_dark)
        });
        format!("background-color: {background}")
    };
    let user_color = game_state.user_color_as_signal(auth_context.identity);
    let game_response = game_state.game_response();
    let now = use_ticking_now();

    let active_deadline = Memo::new(move |_| {
        game_response.with(|response| {
            let response = response.as_ref()?;
            let due_at = response.arena_move_due_at?;
            let due_side = opening_side(response.turn);
            let viewer_is_due = user_color.get() == Some(due_side);
            (response.game_start == GameStart::Arena
                && !response.finished
                && response.turn < 2
                && viewer_is_due)
                .then(|| (response.game_id.clone(), response.turn, due_at))
        })
    });
    let seconds_left = Signal::derive(move || {
        active_deadline
            .get()
            .map(|(_, _, due_at)| seconds_until(due_at, now.get()))
            .unwrap_or_default()
    });
    let urgent = Signal::derive(move || {
        active_deadline
            .get()
            .is_some_and(|(_, _, due_at)| deadline_is_urgent(due_at, now.get()))
    });
    let warning_key = Memo::new(move |_| {
        if !urgent.get() {
            return None;
        }
        active_deadline.get()
    });
    let warned = StoredValue::new(None::<(GameId, usize, DateTime<Utc>)>);

    Effect::watch(
        warning_key,
        move |warning_key, _, _| {
            let Some(warning_key) = warning_key else {
                return;
            };
            if warned.get_value().as_ref() != Some(warning_key) {
                sounds.play_sound(SoundType::LowTime);
                warned.set_value(Some(warning_key.clone()));
            }
        },
        true,
    );

    let deadline_text = Signal::derive(move || {
        let seconds = seconds_left.get();
        if seconds == 0 {
            t_string!(i18n, game.realtime_assignments.opening_due_now).to_string()
        } else {
            t_string!(i18n, game.realtime_assignments.opening_due, count = seconds).to_string()
        }
    });

    view! {
        <Show when=move || active_deadline.get().is_some()>
            <div
                class=move || {
                    format!(
                        "pointer-events-none flex justify-center items-center px-2 py-1 text-center text-sm font-semibold tabular-nums transition-colors {class} {}",
                        if urgent.get() { "text-ladybug-red" } else { "text-grasshopper-green" },
                    )
                }
                style=background_style
                aria-label=move || deadline_text.get()
            >
                {move || deadline_text.get()}
            </div>
        </Show>
    }
}
