use leptos::prelude::*;

const SWITCH_TRACK_CLASS: &str =
    "h-6 w-11 rounded-full border border-black/10 bg-even-light shadow-sm transition-colors peer peer-checked:bg-pillbug-teal peer-disabled:opacity-50 after:absolute after:start-[2px] after:top-[2px] after:size-5 after:rounded-full after:border after:border-black/10 after:bg-white after:transition-all after:content-[''] peer-checked:after:translate-x-full peer-checked:after:border-white rtl:peer-checked:after:-translate-x-full dark:border-white/10 dark:bg-[#222b35] dark:after:border-white/10";

#[component]
pub fn SimpleSwitch(
    checked: RwSignal<bool>,
    #[prop(optional)] disabled: Signal<bool>,
    #[prop(optional)] optional_action: Option<Callback<()>>,
) -> impl IntoView {
    let on_checked_change = move |_| {
        checked.update(|b| *b = !*b);
        if let Some(optional) = optional_action {
            optional.run(())
        };
    };
    view! {
        <label class="inline-flex relative items-center cursor-pointer">
            <input
                on:change=on_checked_change
                prop:disabled=disabled
                type="checkbox"
                value=""
                class="sr-only peer"
                prop:checked=checked
            />
            <div class=SWITCH_TRACK_CLASS></div>
        </label>
    }
}

#[component]
pub fn SimpleSwitchWithCallback(
    checked: Signal<bool>,
    #[prop(optional)] disabled: Signal<bool>,
    action: Callback<()>,
) -> impl IntoView {
    let on_checked_change = move |_| {
        action.run(());
    };
    view! {
        <label class="inline-flex relative items-center cursor-pointer">
            <input
                on:change=on_checked_change
                prop:disabled=disabled
                type="checkbox"
                value=""
                class="sr-only peer"
                prop:checked=checked
            />
            <div class=SWITCH_TRACK_CLASS></div>
        </label>
    }
}
