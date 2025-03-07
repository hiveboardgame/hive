use leptos::{html::Dialog, prelude::*};
use leptos_use::on_click_outside;

#[component]
pub fn Modal(
    open: RwSignal<bool>,
    children: Children,
    dialog_el: NodeRef<Dialog>,
) -> impl IntoView {
    let close_dialog = move || {
        if let Some(dialog_el) = dialog_el.get() { 
            dialog_el.close();
            open.set(false); 
        }
    };
    #[allow(unused)]
    on_click_outside(dialog_el, move |_| close_dialog());
    view! {
        <dialog
            node_ref=dialog_el
            open=open
            class="rounded-lg border shadow-xl drop-shadow-xl backdrop:backdrop-blur bg-stone-300 dark:bg-gray-600 dark:border-gray-500"
        >
            <header class="flex justify-end">
                <form class="m-2" method="dialog">
                    <button
                        class="flex justify-center items-center w-5 h-5 rounded-full duration-300 hover:bg-ladybug-red active:scale-95"
                        aria-label="Close"
                        on:click= move |_| close_dialog()
                    >
                        x
                    </button>
                </form>
            </header>
            <main>{children()}</main>
        </dialog>
    }
}
