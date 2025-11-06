use leptos::html;
use leptos::prelude::*;
use leptos_use::{on_click_outside_with_options, OnClickOutsideOptions};

#[component]
pub fn StickyControls(children: Children) -> impl IntoView {
    let dropdown_ref = NodeRef::<html::Details>::new();

    let close_dropdown = move || {
        if let Some(details) = dropdown_ref.get() {
            let _ = details.remove_attribute("open");
        }
    };

    Effect::new(move |_| {
        let _ = on_click_outside_with_options(
            dropdown_ref,
            move |_| close_dropdown(),
            OnClickOutsideOptions::default(),
        );
    });

    view! {
        <div class="sticky top-[52px] z-40">
            <div class="mx-auto w-full max-w-4xl px-4 sm:px-6 lg:px-8">
                <div class="flex justify py-2">
                    <details node_ref=dropdown_ref>
                        <summary class="px-2 py-1 text-sm font-semibold text-gray-900 bg-gray-100 rounded-lg border-2 border-transparent cursor-pointer hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-100 dark:hover:bg-gray-700">
                            "🔧 Filters"
                        </summary>
                        <div class="absolute top-full z-50 p-4 mt-1 bg-white rounded-lg border border-gray-200 shadow-xl w-[90vw] max-w-md dark:bg-gray-900 dark:border-gray-700">
                            <div class="space-y-3">{children()}</div>
                        </div>
                    </details>
                </div>
            </div>
        </div>
    }
}
