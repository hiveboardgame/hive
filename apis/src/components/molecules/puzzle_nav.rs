use leptos::prelude::*;
use leptos_router::hooks::use_location;

#[component]
pub fn PuzzleNav() -> impl IntoView {
    let pathname = use_location().pathname;
    let link = move |href: &'static str, label: &'static str| {
        let active = move || match href {
            "/puzzles" => {
                let path = pathname.get();
                path == "/puzzles"
                    || (path.starts_with("/puzzles/")
                        && !path.starts_with("/puzzles/browse")
                        && !path.starts_with("/puzzles/daily")
                        && !path.starts_with("/puzzles/history"))
            }
            other => pathname.get().starts_with(other),
        };
        view! {
            <a
                href=href
                class=move || {
                    format!(
                        "inline-flex items-center px-3 min-h-11 text-sm font-semibold no-link-style border-b-2 {}",
                        if active() {
                            "border-pillbug-teal text-pillbug-teal"
                        } else {
                            "border-transparent text-gray-600 dark:text-gray-300 hover:text-pillbug-teal"
                        },
                    )
                }
                aria-current=move || active().then_some("page")
            >
                {label}
            </a>
        }
    };
    view! {
        <nav class="flex gap-1 border-b border-black/10 dark:border-white/10" aria-label="Puzzles">
            {link("/puzzles", "Solve")}
            {link("/puzzles/daily", "Daily")}
            {link("/puzzles/browse", "Sets")}
            {link("/puzzles/history", "History")}
        </nav>
    }
}
