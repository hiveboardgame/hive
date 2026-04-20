use leptos::prelude::*;

const UNREAD_BADGE_BASE_CLASS: &str =
    "shrink-0 inline-flex items-center justify-center min-w-5 h-5 px-1.5 \
     text-xs font-medium leading-none text-white rounded-full";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UnreadBadgeVariant {
    #[default]
    Alert,
    Overlay,
}

impl UnreadBadgeVariant {
    const fn tone_class(self) -> &'static str {
        match self {
            Self::Alert => "bg-ladybug-red dark:bg-red-500",
            Self::Overlay => "bg-black/30 dark:bg-white/30",
        }
    }
}

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
    #[prop(optional)] variant: UnreadBadgeVariant,
) -> impl IntoView {
    view! {
        <Show when=move || count.get().gt(&0)>
            <span class=move || {
                format!("{UNREAD_BADGE_BASE_CLASS} {}", variant.tone_class())
            }>{move || format_unread_count(count.get())}</span>
        </Show>
    }
}
