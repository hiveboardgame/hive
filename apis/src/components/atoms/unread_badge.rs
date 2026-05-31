use leptos::prelude::*;

const UNREAD_BADGE_BASE_CLASS: &str = "shrink-0 inline-grid place-items-center min-w-5 h-5 px-1.5 \
     text-center text-xs font-medium tabular-nums leading-none text-white rounded-full \
     bg-ladybug-red dark:bg-red-500";

fn format_unread_count(count: i64) -> String {
    if count > 99 {
        "99+".to_string()
    } else {
        count.to_string()
    }
}

#[component]
pub fn UnreadBadge(count: Signal<i64>) -> impl IntoView {
    let unread_count = move || count.try_get().unwrap_or_default();

    view! {
        <ShowLet some=move || {
            let unread_count = unread_count();
            (unread_count > 0).then_some(unread_count)
        } let:unread_count>
            <span class=UNREAD_BADGE_BASE_CLASS>
                <span class="relative top-px">{format_unread_count(unread_count)}</span>
            </span>
        </ShowLet>
    }
}
