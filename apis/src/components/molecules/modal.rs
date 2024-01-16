use leptos::{ev::MouseEvent, html::Dialog, *};
use wasm_bindgen::JsCast;

#[component]
pub fn Modal(
    #[prop(into)] open: Signal<bool>,
    children: Children,
    dialog_el: NodeRef<Dialog>,
) -> impl IntoView {
    let on_click = move |ev: MouseEvent| {
        let rect = dialog_el
            .get_untracked()
            .expect("dialog to have been created")
            .get_bounding_client_rect();
        let click_is_in_dialog = rect.top() <= ev.client_y() as f64
            && ev.client_y() as f64 <= rect.top() + rect.height()
            && rect.left() <= ev.client_x() as f64
            && ev.client_x() as f64 <= rect.left() + rect.width();
        if !click_is_in_dialog {
            ev.target()
                .expect("Event target")
                .unchecked_into::<web_sys::HtmlDialogElement>()
                .close();
        }
    };

    create_effect(move |_| {
        if let Some(dialog) = dialog_el.get_untracked() {
            if open() {
                if dialog.show_modal().is_err() {
                    dialog.set_open(true);
                }
            } else {
                dialog.close();
            }
        }
    });

    view! {
        <dialog
            ref=dialog_el
            open=open.get_untracked()
            class="shadow-xl drop-shadow-xl rounded-lg backdrop:backdrop-blur bg-stone-300 dark:bg-gray-600 dark:border-gray-500 border "
            // clicking on ::backdrop should dismiss modal
            on:click=on_click
        >
            <header class="flex justify-end">
                <form class="m-2" method="dialog">
                    <button
                        class="hover:bg-li-red duration-300 active:scale-95 rounded-full w-5 h-5 flex items-center justify-center"
                        aria-label="Close"
                    >
                        x
                    </button>
                </form>
            </header>
            <main>{children()}</main>
        </dialog>
    }
}
