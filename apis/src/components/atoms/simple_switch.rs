use leptos::prelude::*;

#[component]
pub fn SimpleSwitch(
    checked: RwSignal<bool>,
    #[prop(optional)] disabled: Signal<bool>,
    #[prop(optional)] optional_action: Option<Callback<()>>,
) -> impl IntoView {
    let on_checked_change = Callback::new(move |_: bool| {
        checked.update(|b| *b = !*b);
        if let Some(optional) = optional_action {
            optional.run(())
        };
    });
    view! {
        /*<SwitchRoot
            checked
            on_checked_change
            attr:disabled=disabled
            attr:class="disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent items-center transition w-8 h-4 bg-white rounded-full relative focus:shadow-black data-[state=checked]:bg-orange-twilight outline-none cursor-default"
        >
            <SwitchThumb attr:class="hover:bg-pillbug-teal bg-button-dawn dark:bg-button-twilight block w-4 h-4 rounded-full shadow-md transition-transform duration-100 translate-x-0.5 will-change-transform data-[state=checked]:translate-x-[19px]" />
        </SwitchRoot>*/
        "FIX SWITCH"
    }
}
