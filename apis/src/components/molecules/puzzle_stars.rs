use crate::{functions::puzzles::set_puzzle_stars, responses::PuzzleStarsResponse};
use leptos::{prelude::*, task::spawn_local};

const LABELS: [&str; 6] = [
    "Broken or pointless",
    "Poor",
    "Meh",
    "Good",
    "Great",
    "Brilliant",
];

/// 0-5 stars for one puzzle. `prominent` is the occasional nudge: a framed prompt instead of the
/// quiet inline row. The control is always usable; rating is never forced.
#[component]
pub fn PuzzleStars(
    #[prop(into)] nanoid: Signal<String>,
    my_stars: RwSignal<Option<i16>>,
    #[prop(into)] prominent: Signal<bool>,
    #[prop(optional)] on_rated: Option<Callback<PuzzleStarsResponse>>,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let hovered = RwSignal::new(None::<i16>);
    let saving = RwSignal::new(false);
    let saved = RwSignal::new(false);
    let rate = move |stars: i16| {
        if saving.get_untracked() {
            return;
        }
        saving.set(true);
        let previous = my_stars.get_untracked();
        my_stars.set(Some(stars));
        let nanoid = nanoid.get_untracked();
        spawn_local(async move {
            match set_puzzle_stars(nanoid, stars).await {
                Ok(response) => {
                    saved.set(true);
                    if let Some(on_rated) = on_rated {
                        on_rated.run(response);
                    }
                }
                Err(error) => {
                    log::warn!("could not save puzzle stars: {error}");
                    my_stars.set(previous);
                }
            }
            saving.set(false);
        });
    };
    let report_armed = RwSignal::new(false);
    let shown = move || hovered.get().or(my_stars.get()).unwrap_or(-1);
    let label = move || {
        let value = hovered.get().or(my_stars.get());
        match value {
            Some(value) => LABELS[value.clamp(0, 5) as usize].to_string(),
            None if compact => "Rate it".to_string(),
            None => "Rate this puzzle".to_string(),
        }
    };
    let frame = move || {
        if prominent.get() && my_stars.get().is_none() {
            "flex flex-col gap-1 p-2 rounded-lg border-2 border-pillbug-teal bg-pillbug-teal/10 animate-[pulse_1.6s_ease-in-out_2]"
        } else {
            "flex flex-col gap-1"
        }
    };
    view! {
        <div class=frame>
            <Show when=move || prominent.get() && my_stars.get().is_none()>
                <div class="text-sm font-semibold">"How was this puzzle?"</div>
            </Show>
            <div class="flex flex-wrap gap-1 items-center" on:mouseleave=move |_| hovered.set(None)>
                {(1..=5i16)
                    .map(|value| {
                        view! {
                            <button
                                type="button"
                                class=move || {
                                    format!(
                                        "text-2xl leading-none min-w-11 min-h-11 transition-transform hover:scale-110 {}",
                                        if shown() >= value {
                                            "text-orange-twilight"
                                        } else {
                                            "text-gray-300 dark:text-gray-500"
                                        },
                                    )
                                }
                                title=LABELS[value as usize]
                                aria-label=star_label(value)
                                on:mouseenter=move |_| hovered.set(Some(value))
                                on:click=move |_| rate(value)
                            >
                                "★"
                            </button>
                        }
                    })
                    .collect_view()}
                <span class="ml-2 text-xs text-gray-600 whitespace-nowrap dark:text-gray-400">
                    {label}
                </span>
                <Show when=move || saved.get()>
                    <span class="ml-1 text-xs text-grasshopper-green">"Saved ✓"</span>
                </Show>
            </div>
            <Show when=move || !compact>
                <button
                    type="button"
                    class=move || {
                        format!(
                            "self-start min-h-11 text-xs underline-offset-2 hover:underline {}",
                            if my_stars.get() == Some(0) {
                                "text-ladybug-red font-semibold"
                            } else {
                                "text-gray-500 hover:text-ladybug-red"
                            },
                        )
                    }
                    title="Counts as 0 stars: the puzzle is broken, wrong or pointless"
                    on:click=move |_| {
                        if my_stars.get_untracked() == Some(0) {
                            return;
                        }
                        if report_armed.get_untracked() {
                            report_armed.set(false);
                            rate(0);
                        } else {
                            report_armed.set(true);
                        }
                    }
                    on:blur=move |_| report_armed.set(false)
                >
                    {move || {
                        if my_stars.get() == Some(0) {
                            "⚑ Reported, thanks (pick stars to take it back)"
                        } else if report_armed.get() {
                            "⚑ Broken or wrong? Click again to report"
                        } else {
                            "⚑ Report a problem"
                        }
                    }}
                </button>
            </Show>
        </div>
    }
}

fn star_label(value: i16) -> String {
    if value == 1 {
        "1 star".to_string()
    } else {
        format!("{value} stars")
    }
}
