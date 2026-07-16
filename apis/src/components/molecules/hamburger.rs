use crate::{common::with_class, hooks::tap_feedback::use_tap_feedback};
use leptos::{
    html::{Button, Div},
    prelude::*,
};
use leptos_use::{on_click_outside_with_options, OnClickOutsideOptions};

#[component]
pub fn Hamburger<T: IntoView + 'static>(
    hamburger_show: RwSignal<bool>,
    children: ChildrenFn,
    #[prop(into)] button_style: Signal<String>,
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(into)] dropdown_style: Signal<String>,
    id: &'static str,
    content: T,
    #[prop(optional, into)] aria_label: Option<Signal<String>>,
    #[prop(default = "menu")] popup_role: &'static str,
    #[prop(optional, into)] popup_aria_label: Option<Signal<String>>,
) -> impl IntoView {
    let target = NodeRef::<Div>::new();
    let button_ref = NodeRef::<Button>::new();
    let mark_dropdown_press = use_tap_feedback(".ui-dropdown-link");
    let menu_id = StoredValue::new(format!("{id}-menu"));

    Effect::new(move |_| {
        if hamburger_show() {
            let _ = on_click_outside_with_options(
                target,
                move |_| {
                    hamburger_show.update(|b| *b = false);
                },
                OnClickOutsideOptions::default().ignore([
                    "input",
                    "#ignoreChat",
                    &format!("#{id}"),
                ]),
            );
        }
    });

    view! {
        <div
            node_ref=target
            class=format!("relative inline-block {extend_tw_classes}")
            on:keydown=move |event| {
                if event.key() == "Escape" && hamburger_show.get_untracked() {
                    event.prevent_default();
                    event.stop_propagation();
                    hamburger_show.set(false);
                    if let Some(button) = button_ref.get_untracked() {
                        let _ = button.focus();
                    }
                }
            }
        >
            <button
                node_ref=button_ref
                type="button"
                id=id
                on:click=move |_| hamburger_show.update(|b| *b = !*b)
                aria-expanded=move || hamburger_show().to_string()
                aria-label=aria_label
                aria-haspopup=popup_role
                aria-controls=menu_id.get_value()
                class=button_style
            >
                {content}
            </button>
            <Show when=hamburger_show>
                <div
                    id=menu_id.get_value()
                    role=popup_role
                    aria-label=popup_aria_label
                    class=move || with_class("ui-dropdown-panel", dropdown_style.get())
                    on:pointerdown=move |event| mark_dropdown_press.run(event)
                >
                    {children()}
                </div>
            </Show>
        </div>
    }
}
