use crate::providers::auth_context::AuthContext;
use leptos::html::Div;
use leptos::*;
use leptos_use::on_click_outside;

#[derive(Clone)]
pub struct HamburgerDropdown(pub bool);

#[component]
pub fn Hamburger<F, IV>(fallback: F, children: ChildrenFn) -> impl IntoView
where
    F: Fn() -> IV + 'static,
    IV: IntoView,
{
    let target = create_node_ref::<Div>();
    let visible = use_context::<RwSignal<HamburgerDropdown>>().expect("An open/closed context");
    let _ = on_click_outside(target, move |_| {
        visible.update(|b| *b = HamburgerDropdown(false))
    });
    let children = store_value(children);
    let fallback = store_value(fallback);
    let auth_context = use_context::<AuthContext>().expect("Failed to get AuthContext");
    let username = move || {
        if let Some(Ok(user)) = auth_context.user.get() {
            user.username
        } else {
            String::from("not logged in")
        }
    };
    view! {
        <div node_ref=target class="inline-block">
            <button
                on:click=move |_| visible.update(|b| b.0 = !b.0)
                class="bg-blue-500 text-white rounded-md px-2 py-1 m-2 hover:bg-blue-600"
            >
                {username}
            </button>
            <Show
                when=move || visible.get().0
                fallback=move || fallback.with_value(|fallback| fallback())
            >
                <div class="block absolute bg-slate-400 text-black border border-gray-300 rounded-md">
                    {children.with_value(|children| children())}
                </div>
            </Show>
        </div>
    }
}
