use crate::hooks::clipboard_copy::use_clipboard_copy;
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn CopyButton(
    label: &'static str,
    #[prop(into)] value: Signal<String>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let clipboard = use_clipboard_copy();
    let copied = clipboard.copied;
    let copy_text = clipboard.copy_text;
    let copy = move |_| copy_text(value.get_untracked());

    let button_class = move || {
        let base = "ui-button ui-button-sm flex w-full items-center gap-2 px-3 text-xs";
        if copied.get() {
            format!("{base} ui-button-success {extend_tw_classes}")
        } else {
            format!("{base} ui-button-secondary {extend_tw_classes}")
        }
    };

    view! {
        <button
            type="button"
            class=button_class
            aria-label=format!("Copy {label}")
            title=value
            on:click=copy
        >
            <b class="shrink-0">{label}</b>
            <span class="flex-1 min-w-0 font-mono text-xs text-left truncate">{value}</span>
            <Icon
                icon=Signal::derive(move || {
                    if copied.get() {
                        icondata_ai::AiCheckOutlined
                    } else {
                        icondata_ai::AiCopyOutlined
                    }
                })
                attr:class="size-4 shrink-0"
            />
            // Announce the confirmation to screen readers; the icon swap alone is silent.
            <span class="sr-only" aria-live="polite">
                {move || if copied.get() { "Copied" } else { "" }}
            </span>
        </button>
    }
}
