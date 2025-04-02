use crate::{
    components::{
        atoms::simple_switch::SimpleSwitch, molecules::rl_banner::RlBanner,
        organisms::chat::ChatWindow, update_from_event::update_from_input,
    },
    functions::home_banner,
    providers::AuthContext,
};
use leptos::prelude::*;
use shared_types::SimpleDestination;

const LINE_CLASS: &str = "flex items-center py-3 text-sm before:flex-1 before:border-t before:border-black before:me-6 after:flex-1 after:border-t after:border-black after:ms-6 dark:text-white dark:before:border-neutral-600 dark:after:border-neutral-600";

#[component]
pub fn Admin() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();

    view! {
        <div class="pt-20">
            <Show when=move || {
                auth_context.user.get().is_some_and(|account| account.user.admin)
            }>
                <div class=LINE_CLASS>Send Global Warning</div>
                <ChatWindow destination=SimpleDestination::Global />
                <div class=LINE_CLASS>Edit Banner</div>
                <EditBanner />
            </Show>
        </div>
    }
}

#[component]
fn EditBanner() -> impl IntoView {
    let banner =
        OnceResource::new(async move { home_banner::get_with_display().await.unwrap_or_default() });
    view! {
        <Suspense>
            {move || {
                banner
                    .get()
                    .map(|(banner, display)| {
                        let content = RwSignal::new(banner.content);
                        let title = RwSignal::new(banner.title);
                        let show_preview = RwSignal::new(false);
                        let display = RwSignal::new(display);
                        let update = ServerAction::<home_banner::Update>::new();
                        view! {
                            <button
                                on:click=move |_| show_preview.update(|b| *b = !*b)
                                class="flex gap-1 justify-center items-center px-4 m-4 h-7 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                            >
                                {move || {
                                    if !show_preview() { "Preview Banner" } else { "Edit Banner" }
                                }}
                            </button>
                            <ActionForm action=update>
                                <div class=move || {
                                    if show_preview() { "" } else { "hidden" }
                                }>
                                    {move || {
                                        view! { <RlBanner title=title() content=content() /> }
                                    }}
                                </div>
                                <div class=move || if !show_preview() { "" } else { "hidden" }>
                                    <div class="flex flex-col m-2">
                                        <label for="title">Title:</label>
                                        <input
                                            class="px-3 py-2 w-10/12 leading-tight rounded border shadow appearance-none focus:outline-none"
                                            name="title"
                                            type="text"
                                            prop:value=title
                                            placeholder="banner title"
                                            on:input=update_from_input(title)
                                        />
                                    </div>

                                    <textarea
                                        class="px-3 py-2 m-2 w-10/12 h-32 leading-tight rounded border shadow appearance-none focus:outline-none"
                                        name="content"
                                        prop:value=content
                                        on:input=update_from_input(content)
                                        maxlength="2000"
                                    ></textarea>
                                    <div class="flex flex-row gap-1 p-1">
                                        <a
                                            class="font-bold text-blue-500 hover:underline"
                                            href="https://commonmark.org/help/"
                                            target="_blank"
                                        >
                                            "Markdown Cheat Sheet"
                                        </a>
                                    </div>
                                </div>
                                <div class="m-4">

                                    <button
                                        type="submit"
                                        class="flex gap-1 justify-center items-center px-4 h-7 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                                    >
                                        "Submit"
                                    </button>
                                    {move || {
                                        if display() { "Display banner" } else { "Hide banner" }
                                    }}
                                    <SimpleSwitch checked=display />
                                    <input
                                        class="hidden"
                                        type="text"
                                        name="display"
                                        prop:value=display
                                    />
                                </div>
                            </ActionForm>
                        }
                    })
            }}
        </Suspense>
    }
}
