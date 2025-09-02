use leptos::html::Div;
use leptos::prelude::*;
use leptos_use::{on_click_outside_with_options, OnClickOutsideOptions};

#[component]
pub fn Hamburger<T: IntoView + 'static>(
    hamburger_show: RwSignal<bool>,
    children: ChildrenFn,
    #[prop(into)] button_style: Signal<String>,
    #[prop(optional)] extend_tw_classes: &'static str,
    dropdown_style: &'static str,
    id: &'static str,
    content: T,
) -> impl IntoView {
    let target = NodeRef::<Div>::new();
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
                id=id
                on:click=move |_| hamburger_show.update(|b| *b = !*b)

                class=button_style
            >
                {content}
            </button>
            <Show when=hamburger_show>
                <div class=dropdown_style>{children()}</div>
            </Show>
        </div>
    }
}
