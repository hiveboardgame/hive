use leptos::{
    html::{Dialog, Div},
    prelude::*,
};
use leptos_use::on_click_outside;

#[component]
pub fn Modal(children: Children, dialog_el: NodeRef<Dialog>) -> impl IntoView {
    let inner = NodeRef::<Div>::new();
    #[allow(unused)]
    on_click_outside(inner, move |_| {
        if let Some(dialog_el) = dialog_el.get() {
            dialog_el.close();
        }
    });
    view! {
        <dialog
            node_ref=dialog_el
            class="m-auto rounded-lg border shadow-xl dark:text-white dark:bg-gray-600 dark:border-gray-500 drop-shadow-xl backdrop:backdrop-blur bg-stone-300"
        >
            <div node_ref=inner>
                <div class="flex justify-end">
                    <form class="m-2" method="dialog">
                        <button
                            class="flex justify-center items-center rounded-full duration-300 active:scale-95 size-5 hover:bg-ladybug-red"
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
