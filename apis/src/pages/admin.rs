use crate::{
    common::with_class,
    components::{
        atoms::simple_switch::{SimpleSwitch, SimpleSwitchWithCallback},
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::{panel::Panel, rl_banner::RlBanner},
        organisms::chat::ResolvedChatWindow,
        update_from_event::update_from_input,
    },
    functions::{home_banner, site_config},
    providers::{AuthContext, RealtimeAvailability},
};
use leptos::prelude::*;
use leptos_use::{use_interval_fn_with_options, UseIntervalFnOptions};
use shared_types::ConversationKey;

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
                    <ResolvedChatWindow conversation=ConversationKey::Global />
                </Panel>
                <Panel title="Edit Banner" body_class="space-y-4">
                    <EditBanner />
                </Panel>
                <RealtimeMaintenance />
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
fn RealtimeMaintenance() -> impl IntoView {
    let realtime = expect_context::<RealtimeAvailability>();
    let count = RwSignal::new(None::<i64>);

    let transition = Action::new(move |enabled: &bool| {
        let enabled = *enabled;
        async move { site_config::set_realtime_enabled(enabled).await }
    });
    let refresh_count =
        Action::new(move |_: &()| async move { site_config::count_active_realtime_clocks().await });
    let transition_error = Signal::derive(move || {
        transition
            .value()
            .get()
            .and_then(|result| result.err().map(|error| error.to_string()))
    });
    let count_error = Signal::derive(move || {
        refresh_count
            .value()
            .get()
            .and_then(|result| result.err().map(|error| error.to_string()))
    });

    Effect::new(move |_| {
        if let Some(Ok(enabled)) = transition.value().get() {
            realtime.apply_incremental(enabled);
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(active)) = refresh_count.value().get() {
            count.set(Some(active));
        }
    });

    let interval = use_interval_fn_with_options(
        move || {
            if realtime.state() == Some(false) && !refresh_count.pending().get_untracked() {
                refresh_count.dispatch(());
            }
        },
        5_000,
        UseIntervalFnOptions::default().immediate(false),
    );
    Effect::new(move |_| match realtime.state() {
        Some(false) => {
            if !refresh_count.pending().get_untracked() {
                refresh_count.dispatch(());
            }
            (interval.resume)();
        }
        _ => (interval.pause)(),
    });

    let toggle = Callback::new(move |()| {
        if !transition.pending().get_untracked() {
            if let Some(enabled) = realtime.state() {
                transition.dispatch(!enabled);
            }
        }
    });
    let toggle_disabled: Signal<bool> = transition.pending().into();
    let checked = Signal::derive(move || realtime.state() == Some(true));

    view! {
        <Panel title="Realtime Maintenance" body_class="space-y-3">
            <div class="flex flex-wrap gap-3 items-center ui-setting-group">
                <span class="font-semibold">"Realtime game starts"</span>
                <Show
                    when=move || realtime.state().is_some()
                    fallback=|| {
                        view! { <span class="ui-notice">"Waiting for server state..."</span> }
                    }
                >
                    <SimpleSwitchWithCallback checked disabled=toggle_disabled action=toggle />
                    <span>{move || if checked() { "Enabled" } else { "Disabled" }}</span>
                </Show>
            </div>
            <Show when=move || transition_error.get().is_some()>
                <p class="ui-notice">{move || transition_error.get().unwrap_or_default()}</p>
            </Show>
            <Show when=move || realtime.state() == Some(false)>
                <p class="font-semibold">
                    {move || {
                        count
                            .get()
                            .map_or_else(
                                || "Realtime games still in progress: loading...".to_string(),
                                |active| format!("Realtime games still in progress: {active}"),
                            )
                    }}
                </p>
                <Show when=move || count_error.get().is_some()>
                    <p class="ui-notice">{move || count_error.get().unwrap_or_default()}</p>
                </Show>
            </Show>
        </Panel>
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
