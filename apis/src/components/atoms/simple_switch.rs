use leptos::prelude::*;

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
            <div class="w-11 h-6 bg-gray-200 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:size-5 after:transition-all dark:border-gray-600 peer-checked:bg-orange-twilight"></div>
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
            <div class="w-11 h-6 bg-gray-200 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:size-5 after:transition-all dark:border-gray-600 peer-checked:bg-orange-twilight"></div>
        </label>
    }
}
