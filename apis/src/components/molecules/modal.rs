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
            class="m-auto rounded-lg border shadow-xl drop-shadow-xl backdrop:backdrop-blur bg-stone-300 dark:bg-gray-600 dark:border-gray-500 dark:text-white"
        >
            <div node_ref=inner>
                <div class="flex justify-end">
                    <form class="m-2" method="dialog">
                        <button
                            class="flex justify-center items-center size-5 rounded-full duration-300 hover:bg-ladybug-red active:scale-95"
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
