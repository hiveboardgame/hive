use crate::{functions::puzzles::vote_puzzle, responses::PuzzleVoteResponse};
use leptos::{prelude::*, task::spawn_local};

/// 👍 / 👎 for one puzzle. Pressing the chosen one again takes the vote back.
#[component]
pub fn PuzzleVote(
    #[prop(into)] nanoid: Signal<String>,
    my_vote: RwSignal<Option<bool>>,
    #[prop(optional)] on_voted: Option<Callback<PuzzleVoteResponse>>,
    #[prop(optional)] large: bool,
) -> impl IntoView {
    let saving = RwSignal::new(false);
    let vote = move |liked: bool| {
        if saving.get_untracked() {
            return;
        }
        saving.set(true);
        let previous = my_vote.get_untracked();
        let wanted = (previous != Some(liked)).then_some(liked);
        my_vote.set(wanted);
        let nanoid = nanoid.get_untracked();
        spawn_local(async move {
            match vote_puzzle(nanoid, wanted).await {
                Ok(response) => {
                    if let Some(on_voted) = on_voted {
                        on_voted.run(response);
                    }
                }
                Err(error) => {
                    log::warn!("could not save the puzzle vote: {error}");
                    let _ = my_vote.try_set(previous);
                }
            }
            let _ = saving.try_set(false);
        });
    };
    let size = if large { "w-12 h-12" } else { "w-11 h-11" };
    let button = move |liked: bool, emoji: &'static str, title: &'static str| {
        view! {
            <button
                type="button"
                title=title
                aria-label=title
                aria-pressed=move || (my_vote.get() == Some(liked)).to_string()
                class=move || {
                    let state = match (my_vote.get() == Some(liked), liked) {
                        (true, true) => "bg-grasshopper-green/25 border-grasshopper-green",
                        (true, false) => "bg-ladybug-red/20 border-ladybug-red",
                        (false, _) => {
                            "border-black/15 dark:border-white/20 grayscale-[60%] hover:grayscale-0"
                        }
                    };
                    format!(
                        "grid place-items-center text-xl rounded-lg border shrink-0 transition-[filter] {size} {state}",
                    )
                }
                on:click=move |_| vote(liked)
            >
                {emoji}
            </button>
        }
    };
    view! {
        <div class="flex gap-2 items-center shrink-0">
            {button(true, "👍", "Good puzzle")}
            {button(false, "👎", "Bad puzzle: broken, trivial or wrong")}
        </div>
    }
}
