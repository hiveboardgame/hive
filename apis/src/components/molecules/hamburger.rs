use leptos::html::Div;
use leptos::logging::log;
use leptos::*;
use leptos_use::on_click_outside;

#[component]
pub fn Hamburger<T: IntoView>(
    hamburger_show: RwSignal<bool>,
    children: ChildrenFn,
    #[prop(into)] button_style: MaybeSignal<String>,
    dropdown_style: &'static str,
    content: T,
) -> impl IntoView {
    let target = create_node_ref::<Div>();
    let button_ref = create_node_ref::<html::Button>();
    let _ = on_click_outside(target, move |_| hamburger_show.update(|b| *b = false));
    let children = store_value(children);

    view! {
        <div node_ref=target class="inline-block">
            <button
                ref=button_ref
                on:click=move |_| {
                    let rect = button_ref
                        .get_untracked()
                        .expect("button to have been created")
                        .get_bounding_client_rect();
                    log!("{:?}", rect);
                    hamburger_show.update(|b| *b = !*b)
                }

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
