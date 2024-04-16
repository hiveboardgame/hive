use leptos::html::Div;
use leptos::*;
use leptos_use::on_click_outside;

#[component]
pub fn Hamburger<T: IntoView>(
    hamburger_show: RwSignal<bool>,
    children: ChildrenFn,
    #[prop(into)] button_style: MaybeSignal<String>,
    #[prop(optional)] extend_tw_classes: &'static str,
    dropdown_style: &'static str,
    content: T,
) -> impl IntoView {
    let target = create_node_ref::<Div>();
    let _ = on_click_outside(target, move |_| hamburger_show.update(|b| *b = false));
    let children = store_value(children);

    view! {
        <div node_ref=target class=format!("inline-block {extend_tw_classes}")>
            <button
                on:click=move |_| { hamburger_show.update(|b| *b = !*b) }

                class=button_style
            >
                {content}
            </button>
            <Show when=hamburger_show>
                <div class=dropdown_style>{children.with_value(|children| children())}</div>
            </Show>
        </div>
    }
}
