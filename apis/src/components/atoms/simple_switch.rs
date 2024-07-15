use leptix_primitives::components::switch::{SwitchRoot, SwitchThumb};
use leptos::*;

#[component]
pub fn SimpleSwitch(checked: RwSignal<bool>) -> impl IntoView {
    view! {
        <SwitchRoot
            checked
            on_checked_change=move |_| checked.update(|b| *b = !*b)
            attr:class="items-center transition w-8 h-4 bg-white rounded-full relative focus:shadow-black data-[state=checked]:bg-orange-twilight outline-none cursor-default "
        >
            <SwitchThumb attr:class="hover:bg-pillbug-teal bg-button-dawn dark:bg-button-twilight block w-4 h-4 rounded-full shadow-md transition-transform duration-100 translate-x-0.5 will-change-transform data-[state=checked]:translate-x-[19px]"/>
        </SwitchRoot>
    }
}
