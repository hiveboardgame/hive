use crate::providers::auth_context::AuthContext;
use leptos::html::Div;
use leptos::*;
use leptos_use::on_click_outside;

#[component]
pub fn Hamburger<F, IV>(
    hamburger_show: RwSignal<bool>,
    fallback: F,
    children: ChildrenFn,
) -> impl IntoView
where
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    let target = create_node_ref::<Div>();
    let _ = on_click_outside(target, move |_| hamburger_show.update(|b| *b = false));
    let children = store_value(children);
    let fallback = store_value(fallback);
    let auth_context = expect_context::<AuthContext>();
    let username = move || {
        if let Some(Ok(Some(user))) = auth_context.user.get() {
            user.username
        } else {
            String::from("not logged in")
        }
    };
    view! {
        <div node_ref=target class="inline-block">
            <button
                on:click=move |_| hamburger_show.update(|b| *b = !*b)
                class="bg-blue-500 text-white rounded-md px-2 py-1 m-2 hover:bg-blue-600"
            >
                {username}
            </button>
            <Show
                when=move || hamburger_show()
                fallback=move || fallback.with_value(|fallback| fallback())
            >
                <div class="block absolute bg-slate-400 text-black border border-gray-300 rounded-md">
                    {children.with_value(|children| children())}
                </div>
            </Show>
        </div>
    }
}

