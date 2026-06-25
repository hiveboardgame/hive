use crate::{common::with_class, hooks::tap_feedback::use_tap_feedback};
use leptos::{html::Div, prelude::*};
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
    #[prop(optional)] aria_label: Option<&'static str>,
) -> impl IntoView {
    let target = NodeRef::<Div>::new();
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
        <div node_ref=target class=format!("relative inline-block {extend_tw_classes}")>
            <button
                type="button"
                id=id
                on:click=move |_| hamburger_show.update(|b| *b = !*b)
                aria-expanded=move || hamburger_show().to_string()
                aria-label=aria_label
                aria-haspopup="menu"
                aria-controls=menu_id.get_value()
                class=button_style
            >
                {content}
            </button>
            <Show when=hamburger_show>
                <div
                    id=menu_id.get_value()
                    role="menu"
                    class=move || with_class("ui-dropdown-panel", dropdown_style.get())
                    on:pointerdown=move |event| mark_dropdown_press.run(event)
                >
                    {children()}
                </div>
            </Show>
        </div>
    }
}
