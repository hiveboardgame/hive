use leptos::{
    html::{Dialog, Div},
    prelude::*,
};
use leptos_use::on_click_outside;

#[component]
pub fn Modal(
    children: Children,
    dialog_el: NodeRef<Dialog>,
    #[prop(optional, into)] aria_label: Option<String>,
    #[prop(optional, into)] aria_labelledby: Option<String>,
    #[prop(optional)] on_close: Option<Callback<()>>,
    #[prop(optional)] hide_close_button: bool,
) -> impl IntoView {
    let inner = NodeRef::<Div>::new();
    _ = on_click_outside(inner, move |_| {
        if let Some(dialog_el) = dialog_el.get() {
            dialog_el.close();
        }
    });
    view! {
        <dialog
            node_ref=dialog_el
            class="ui-modal-panel"
            aria-label=aria_label
            aria-labelledby=aria_labelledby
            on:close=move |_| {
                if let Some(on_close) = on_close {
                    let _ = on_close.try_run(());
                }
            }
        >
            <div
                node_ref=inner
                on:mousedown=|ev| ev.stop_propagation()
                on:click=|ev| ev.stop_propagation()
            >
                <div class="flex justify-end" class:hidden=hide_close_button>
                    <form class="m-2" method="dialog">
                        <button
                            class="hover:text-white ui-button ui-button-ghost ui-button-icon-sm hover:bg-ladybug-red"
                            aria-label="Close"
                        >
                            x
                        </button>
                    </form>
                </div>
                <div>{children()}</div>
            </div>
        </dialog>
    }
}
