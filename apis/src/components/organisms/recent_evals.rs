use crate::{
    common::with_class,
    components::{
        atoms::grade_mark::{grade_fill_class, grade_plural},
        molecules::empty_state::EmptyState,
    },
    hooks::recent_evals::RecentEvalsFeed,
};
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use shared_types::{Grade, RecentEval};

fn ago(then: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let minutes = (now - then).num_minutes().max(0);
    match minutes {
        0 => "just now".to_string(),
        1..=59 => format!("{minutes} min ago"),
        60..=1439 => format!("{} h ago", minutes / 60),
        _ => format!("{} d ago", minutes / 1440),
    }
}

/// Finished engine evals and who asked for them, so people see the feature and see that the
/// queue is shared.
#[component]
pub fn RecentEvals(evals: RecentEvalsFeed) -> impl IntoView {
    view! {
        <div class="overflow-hidden w-full ui-panel">
            <div class=with_class("ui-panel-header", "flex-col justify-center text-center")>
                <h2 class="text-xl font-bold">"Recent evaluations"</h2>
            </div>
            <div class="pt-3 ui-panel-body">
                // A Transition keeps the last list up while the minutely refresh loads.
                <Transition fallback=|| {
                    view! { <EmptyState title="Loading evaluations..." /> }
                }>
                    {move || {
                        evals
                            .get()
                            .flatten()
                            .map(|result| match result {
                                Ok(evals) => {
                                    let now = Utc::now();
                                    let in_line = evals.in_line;
                                    let rows = if evals.recent.is_empty() {
                                        view! {
                                            <EmptyState title="No evaluations of tournament or strong games yet" />
                                        }
                                            .into_any()
                                    } else {
                                        evals
                                            .recent
                                            .into_iter()
                                            .map(|eval| view! { <RecentEvalRow eval now /> })
                                            .collect_view()
                                            .into_any()
                                    };
                                    view! {
                                        {rows}
                                        <HowToRequest in_line />
                                    }
                                        .into_any()
                                }
                                Err(_) => {
                                    view! { <EmptyState title="Could not load evaluations" /> }
                                        .into_any()
                                }
                            })
                    }}
                </Transition>
            </div>
        </div>
    }
}

#[component]
fn HowToRequest(in_line: usize) -> impl IntoView {
    let waiting = match in_line {
        0 => None,
        1 => Some("1 game is being evaluated or waiting for the engine.".to_string()),
        n => Some(format!(
            "{n} games are being evaluated or waiting for the engine."
        )),
    };
    view! {
        <div class="flex flex-col gap-1 px-2 pt-3 mt-2 text-xs border-t opacity-80 border-black/10 dark:border-white/10">
            <p>
                <span class="font-semibold">"Want your own game evaluated?"</span>
                " Open it in analysis, go to the Evals tab and press \u{201c}Request computer eval\u{201d}."
            </p>
            {waiting.map(|text| view! { <p>{text}</p> })}
        </div>
    }
}

/// "White won on the board", "Draw by repetition"; anything unrecognised as stored.
fn result_text(game_status: &str, conclusion: &str) -> String {
    let winner = match game_status {
        "Finished(1-0)" => Some("White won"),
        "Finished(0-1)" => Some("Black won"),
        "Finished(½-½)" => None,
        other => return other.to_string(),
    };
    match (winner, conclusion) {
        (Some(winner), "Board") => format!("{winner} on the board"),
        (Some(winner), "Timeout") => format!("{winner} on time"),
        (Some(winner), "Resigned") => format!("{winner} by resignation"),
        (Some(winner), "Forfeit") => format!("{winner} by forfeit"),
        (Some(winner), _) => winner.to_string(),
        (None, "Repetition") => "Draw by repetition".to_string(),
        (None, "Draw") => "Draw agreed".to_string(),
        (None, _) => "Draw".to_string(),
    }
}

/// Ratings are cut to whole points, as everywhere else on the site.
fn player(name: &str, rating: Option<f64>) -> String {
    match rating {
        Some(rating) => format!("{name} ({})", rating.trunc() as i64),
        None => name.to_string(),
    }
}

/// "2 blunders · 1 mistake", worst first; None when the engine marked nothing.
fn marks_text([inaccuracies, mistakes, blunders]: [usize; 3]) -> Option<String> {
    let parts: Vec<String> = [
        (blunders, Grade::Blunder),
        (mistakes, Grade::Mistake),
        (inaccuracies, Grade::Inaccuracy),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, grade)| format!("{count} {}", grade_plural(grade, count)))
    .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

#[component]
fn RecentEvalRow(eval: RecentEval, now: DateTime<Utc>) -> impl IntoView {
    let asked = match (eval.automatic, eval.requested_by) {
        (true, _) => "evaluated automagically".to_string(),
        (false, Some(name)) => format!("requested by {name}"),
        (false, None) => "requested by a deleted account".to_string(),
    };
    let casual = eval.white_rating.is_none() && eval.black_rating.is_none();
    let details = format!(
        "{}{} · {} moves · {}",
        if casual { "Casual · " } else { "" },
        result_text(&eval.game_status, &eval.conclusion),
        eval.moves,
        ago(eval.finished_at, now),
    );
    let worst = [Grade::Blunder, Grade::Mistake, Grade::Inaccuracy]
        .into_iter()
        .zip([eval.marks[2], eval.marks[1], eval.marks[0]])
        .find(|(_, count)| *count > 0)
        .map(|(grade, _)| grade);
    let marks = marks_text(eval.marks).unwrap_or_else(|| "no mistakes found".to_string());
    view! {
        <a
            href=format!("/analysis/{}?tab=evals", eval.game_id)
            class="flex flex-col gap-0.5 py-2 px-2 text-sm rounded transition-colors no-link-style dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70"
        >
            {eval
                .tournament
                .map(|name| {
                    view! { <span class="text-xs font-semibold opacity-75 truncate">{name}</span> }
                })}
            <span class="font-semibold truncate">
                {format!(
                    "{}  vs  {}",
                    player(&eval.white, eval.white_rating),
                    player(&eval.black, eval.black_rating),
                )}
            </span>
            <span class="text-xs opacity-75 truncate">{details}</span>
            <span class="inline-flex gap-1.5 items-center text-xs truncate">
                {worst
                    .map(|grade| {
                        view! {
                            <span class=format!(
                                "inline-block w-2 h-2 rounded-full shrink-0 {}",
                                grade_fill_class(grade),
                            )></span>
                        }
                    })} {marks}
            </span>
            <span class="text-xs italic opacity-60 truncate">{asked}</span>
        </a>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn ages_read_in_the_largest_whole_unit() {
        let now = Utc::now();
        assert_eq!(ago(now - Duration::seconds(30), now), "just now");
        assert_eq!(ago(now - Duration::minutes(5), now), "5 min ago");
        assert_eq!(ago(now - Duration::minutes(150), now), "2 h ago");
        assert_eq!(ago(now - Duration::days(3), now), "3 d ago");
    }

    #[test]
    fn results_read_as_a_sentence() {
        assert_eq!(
            result_text("Finished(1-0)", "Board"),
            "White won on the board"
        );
        assert_eq!(result_text("Finished(0-1)", "Timeout"), "Black won on time");
        assert_eq!(
            result_text("Finished(½-½)", "Repetition"),
            "Draw by repetition"
        );
        assert_eq!(result_text("Finished(½-½)", "Draw"), "Draw agreed");
        assert_eq!(result_text("Adjudicated", "Committee"), "Adjudicated");
    }

    #[test]
    fn marks_read_worst_first_and_skip_zeros() {
        assert_eq!(
            marks_text([3, 0, 1]),
            Some("1 blunder · 3 inaccuracies".to_string())
        );
        assert_eq!(
            marks_text([1, 2, 0]),
            Some("2 mistakes · 1 inaccuracy".to_string())
        );
        assert_eq!(marks_text([0, 0, 0]), None);
    }

    #[test]
    fn ratings_are_shown_whole_and_only_when_known() {
        assert_eq!(player("OrdepCubik", Some(2115.92)), "OrdepCubik (2115)");
        assert_eq!(player("guest", None), "guest");
    }

    #[test]
    fn a_clock_running_behind_never_shows_a_future_time() {
        let now = Utc::now();
        assert_eq!(ago(now + Duration::minutes(2), now), "just now");
    }
}
