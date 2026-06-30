use leptos::prelude::*;

const UNREAD_BADGE_BASE_CLASS: &str = "ui-unread-badge";

fn format_unread_count(count: i64) -> String {
    if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    }
}

#[component]
pub fn UnreadBadge(
    count: Signal<i64>,
    #[prop(optional, into)] aria_label: Option<Signal<String>>,
) -> impl IntoView {
    let unread_count = move || count.try_get().unwrap_or_default();
    let aria_label = aria_label.unwrap_or_else(|| Signal::derive(String::new));

    view! {
        <ShowLet
            some=move || {
                let unread_count = unread_count();
                (unread_count > 0).then_some(unread_count)
            }
            let:unread_count
        >
            <span
                class=UNREAD_BADGE_BASE_CLASS
                aria-hidden=move || aria_label.get().is_empty().to_string()
                aria-label=move || {
                    let label = aria_label.get();
                    (!label.is_empty()).then_some(label)
                }
            >
                <span class="relative top-px">{format_unread_count(unread_count)}</span>
            </span>
        </ShowLet>
    }
}
