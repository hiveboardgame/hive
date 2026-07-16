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
    #[prop(into)] count: Signal<i64>,
    #[prop(into)] aria_label: Signal<String>,
) -> impl IntoView {
    let unread_count = move || count.try_get().unwrap_or_default();

    view! {
        <ShowLet
            some=move || {
                let unread_count = unread_count();
                (unread_count > 0).then_some(unread_count)
            }
            let:unread_count
        >
            <span class=UNREAD_BADGE_BASE_CLASS aria-hidden="true">
                <span class="relative top-px">{format_unread_count(unread_count)}</span>
            </span>
            <span class="sr-only">{move || aria_label.try_get().unwrap_or_default()}</span>
        </ShowLet>
    }
}
