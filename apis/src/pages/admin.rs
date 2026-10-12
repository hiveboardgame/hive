use crate::{
    common::with_class,
    components::{
        atoms::simple_switch::SimpleSwitch,
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::{panel::Panel, rl_banner::RlBanner},
        organisms::chat::ResolvedChatWindow,
        update_from_event::update_from_input,
    },
    functions::{
        home_banner,
        puzzles::{daily_schedule, queue_daily_puzzle, unqueue_daily_puzzle},
    },
    providers::AuthContext,
    security::csrf::ActionForm,
};
use leptos::prelude::*;
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
                <Panel title="Puzzle of the day" body_class="space-y-4">
                    <DailyPuzzleQueue />
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

#[component]
fn DailyPuzzleQueue() -> impl IntoView {
    let refresh = RwSignal::new(0u32);
    let schedule = Resource::new(move || refresh.get(), |_| daily_schedule());
    let input = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let queue = move |_| {
        let nanoid = input.get_untracked();
        if nanoid.trim().is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            match queue_daily_puzzle(nanoid).await {
                Ok(()) => {
                    input.set(String::new());
                    error.set(None);
                    refresh.update(|n| *n += 1);
                }
                Err(err) => error.set(Some(server_message(err))),
            }
        });
    };
    let remove = move |nanoid: String| {
        leptos::task::spawn_local(async move {
            if let Err(err) = unqueue_daily_puzzle(nanoid).await {
                error.set(Some(server_message(err)));
            }
            refresh.update(|n| *n += 1);
        });
    };
    view! {
        <p class="text-sm text-gray-600 dark:text-gray-400">
            "Queued puzzles become the puzzle of the day in order, one per day (UTC). Days without a queued puzzle are picked automatically."
        </p>
        <div class="flex flex-wrap gap-2 items-center">
            <input
                class="flex-1 min-w-48 ui-field-input"
                type="text"
                placeholder="Puzzle id or link"
                prop:value=input
                on:input=update_from_input(input)
                on:keydown=move |ev| {
                    if ev.key() == "Enter" {
                        queue(());
                    }
                }
            />
            <button class="ui-button ui-button-primary ui-button-md" on:click=move |_| queue(())>
                "Queue"
            </button>
        </div>
        {move || error.get().map(|error| view! { <p class="text-sm text-ladybug-red">{error}</p> })}
        <Transition>
            {move || {
                schedule
                    .get()
                    .map(|schedule| match schedule {
                        Ok(schedule) => {
                            view! {
                                <div class="grid gap-4 sm:grid-cols-2">
                                    <div class="flex flex-col gap-1">
                                        <h3 class="font-bold">"Coming up"</h3>
                                        {if schedule.queued.is_empty() {
                                            view! {
                                                <p class="text-sm text-gray-600 dark:text-gray-400">
                                                    "Nothing queued."
                                                </p>
                                            }
                                                .into_any()
                                        } else {
                                            schedule
                                                .queued
                                                .into_iter()
                                                .map(|entry| {
                                                    let nanoid = entry.nanoid.clone();
                                                    view! {
                                                        <div class="flex gap-2 items-center text-sm">
                                                            <span class="w-24 tabular-nums">
                                                                {entry.day.format("%a %d %b").to_string()}
                                                            </span>
                                                            <a
                                                                class="ui-text-link"
                                                                href=format!("/puzzles/{}", entry.nanoid)
                                                                target="_blank"
                                                            >
                                                                {entry.nanoid.clone()}
                                                            </a>
                                                            <span class="text-gray-600 dark:text-gray-400">
                                                                {format!(
                                                                    "{} · win in {} · {}",
                                                                    entry.game_type,
                                                                    entry.solver_moves,
                                                                    entry.rating,
                                                                )}
                                                            </span>
                                                            <button
                                                                class="ml-auto ui-button ui-button-secondary ui-button-sm"
                                                                on:click=move |_| remove(nanoid.clone())
                                                            >
                                                                "Remove"
                                                            </button>
                                                        </div>
                                                    }
                                                })
                                                .collect_view()
                                                .into_any()
                                        }}
                                    </div>
                                    <div class="flex flex-col gap-1">
                                        <h3 class="font-bold">"Recent"</h3>
                                        {schedule
                                            .recent
                                            .into_iter()
                                            .map(|(day, nanoid)| {
                                                view! {
                                                    <div class="flex gap-2 text-sm">
                                                        <span class="w-24 tabular-nums">
                                                            {day.format("%a %d %b").to_string()}
                                                        </span>
                                                        <a
                                                            class="ui-text-link"
                                                            href=format!("/puzzles/{nanoid}")
                                                            target="_blank"
                                                        >
                                                            {nanoid.clone()}
                                                        </a>
                                                    </div>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            }
                                .into_any()
                        }
                        Err(err) => {
                            view! { <p class="text-sm text-ladybug-red">{server_message(err)}</p> }
                                .into_any()
                        }
                    })
            }}
        </Transition>
    }
}

fn server_message(error: ServerFnError) -> String {
    match error {
        ServerFnError::ServerError(message) => message,
        other => other.to_string(),
    }
}
