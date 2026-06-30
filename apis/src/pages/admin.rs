use crate::{
    common::with_class,
    components::{
        atoms::simple_switch::SimpleSwitch,
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::{panel::Panel, rl_banner::RlBanner},
        organisms::chat::ChatInput,
        update_from_event::update_from_input,
    },
    functions::home_banner,
    providers::AuthContext,
};
use leptos::prelude::*;
use shared_types::ChatDestination;

#[component]
pub fn Admin() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();

    view! {
        <PageShell>
            <PageHeader title="Admin" />
            <Show when=move || {
                auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))
            }>
                <Panel title="Send Global Warning" body_class="space-y-3">
                    <p class="ui-field-helper">
                        "Broadcast a global chat warning. Press Enter to send."
                    </p>
                    <ChatInput
                        destination=Signal::derive(|| ChatDestination::Global)
                        disabled=Signal::derive(|| false)
                    />
                </Panel>
                <Panel title="Edit Banner" body_class="space-y-4">
                    <EditBanner />
                </Panel>
                <Panel title="Telemetry" body_class="flex flex-wrap gap-2">
                    <a class="ui-button ui-button-primary ui-button-md" href="/admin/telemetry">
                        "Open WS telemetry dashboard"
                    </a>
                    <a
                        class="ui-button ui-button-secondary ui-button-md"
                        href="/admin/push-metrics"
                    >
                        "Open push notification metrics"
                    </a>
                </Panel>
            </Show>
        </PageShell>
    }
}

#[component]
fn EditBanner() -> impl IntoView {
    let banner =
        OnceResource::new(async move { home_banner::get_with_display().await.unwrap_or_default() });
    view! {
        <Transition>
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
                                class="ui-button ui-button-secondary ui-button-md w-fit"
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
                                    <div class="flex flex-col gap-1.5">
                                        <label class="ui-field-label" for="title">
                                            Title
                                        </label>
                                        <input
                                            class="ui-field-input"
                                            name="title"
                                            type="text"
                                            prop:value=title
                                            placeholder="banner title"
                                            on:input=update_from_input(title)
                                        />
                                    </div>

                                    <textarea
                                        class="mt-3 h-32 ui-field-textarea"
                                        name="content"
                                        prop:value=content
                                        on:input=update_from_input(content)
                                        maxlength="2000"
                                    ></textarea>
                                    <div class="flex flex-row gap-1 p-1">
                                        <a
                                            class="ui-text-link"
                                            href="https://commonmark.org/help/"
                                            target="_blank"
                                        >
                                            "Markdown Cheat Sheet"
                                        </a>
                                    </div>
                                </div>
                                <div class=with_class(
                                    "ui-setting-group",
                                    "mt-4 flex flex-wrap items-center gap-3",
                                )>
                                    <button
                                        type="submit"
                                        class="ui-button ui-button-primary ui-button-md"
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
        </Transition>
    }
}
