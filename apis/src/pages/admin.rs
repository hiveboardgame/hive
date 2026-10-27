use crate::{
    common::{localized_error_message, with_class},
    components::{
        atoms::simple_switch::SimpleSwitch,
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::{panel::Panel, rl_banner::RlBanner},
        organisms::chat::ResolvedChatWindow,
        update_from_event::update_from_input,
    },
    functions::{home_banner, realtime_games},
    i18n::{t, t_string, use_i18n},
    providers::{AuthContext, RealtimeGames},
    security::csrf::ActionForm,
};
use leptos::prelude::*;
use leptos_use::use_interval_fn;
use shared_types::ConversationKey;

#[component]
pub fn Admin() -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();

    view! {
        <PageShell>
            <PageHeader title=move || t_string!(i18n, common.admin.page_title) />
            <Show when=move || {
                auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))
            }>
                <Panel
                    title=move || t_string!(i18n, common.admin.send_global_warning)
                    body_class="space-y-3"
                >
                    <ResolvedChatWindow conversation=ConversationKey::Global />
                </Panel>
                <Panel
                    title=move || t_string!(i18n, common.admin.edit_banner)
                    body_class="space-y-4"
                >
                    <EditBanner />
                </Panel>
                <Panel
                    title=move || t_string!(i18n, common.admin.realtime_games.title)
                    body_class="space-y-3"
                >
                    <RealtimeGamesControl />
                </Panel>
                <Panel
                    title=move || t_string!(i18n, common.admin.telemetry.title)
                    body_class="flex flex-wrap gap-2"
                >
                    <a class="ui-button ui-button-primary ui-button-md" href="/admin/telemetry">
                        {t!(i18n, common.admin.open_ws_telemetry_dashboard)}
                    </a>
                    <a
                        class="ui-button ui-button-secondary ui-button-md"
                        href="/admin/push-metrics"
                    >
                        {t!(i18n, common.admin.open_push_notification_metrics)}
                    </a>
                </Panel>
            </Show>
        </PageShell>
    }
}

#[component]
fn RealtimeGamesControl() -> impl IntoView {
    let i18n = use_i18n();
    let realtime = expect_context::<RealtimeGames>();
    let refresh_token = RwSignal::new(0u32);
    let set_paused = ServerAction::<realtime_games::SetPaused>::new();
    let status = Resource::new(
        move || (refresh_token.get(), realtime.paused.get()),
        |_| realtime_games::status(),
    );
    Effect::new(move |_| {
        if let Some(Ok(current)) = status.get() {
            realtime.apply(current.paused);
        }
    });
    Effect::watch(
        set_paused.version(),
        move |_, _, _| {
            if let Some(Ok(paused)) = set_paused.value().get_untracked() {
                realtime.apply(paused);
            }
        },
        false,
    );
    use_interval_fn(
        move || {
            if realtime.paused.get_untracked() {
                refresh_token.update(|n| *n = n.wrapping_add(1));
            }
        },
        5_000,
    );
    let toggle_error = move || {
        set_paused
            .value()
            .get()
            .and_then(Result::err)
            .map(|error| localized_error_message(i18n, &error.to_string()))
    };
    view! {
        <Transition fallback=move || {
            view! { <p>{t!(i18n, common.loading)}</p> }
        }>
            {move || {
                status
                    .get()
                    .map(|result| match result {
                        Ok(current) => view! { <RealtimeGamesStatusView current /> }.into_any(),
                        Err(error) => {
                            view! {
                                <p class="ui-notice">
                                    {localized_error_message(i18n, &error.to_string())}
                                </p>
                            }
                                .into_any()
                        }
                    })
            }}
        </Transition>
        <button
            type="button"
            class=move || {
                if realtime.paused.get() {
                    "ui-button ui-button-primary ui-button-md"
                } else {
                    "ui-button ui-button-danger ui-button-md"
                }
            }
            disabled=move || set_paused.pending().get()
            on:click=move |_| {
                set_paused
                    .dispatch(realtime_games::SetPaused {
                        paused: !realtime.paused.get_untracked(),
                    });
            }
        >
            {move || {
                if realtime.paused.get() {
                    t_string!(i18n, common.admin.realtime_games.resume)
                } else {
                    t_string!(i18n, common.admin.realtime_games.stop)
                }
            }}
        </button>
        <Show when=move || toggle_error().is_some()>
            <p class="ui-notice">{toggle_error}</p>
        </Show>
        <p class="text-sm text-gray-600 dark:text-gray-400">
            {t!(i18n, common.admin.realtime_games.explanation)}
        </p>
    }
}

#[component]
fn RealtimeGamesStatusView(current: realtime_games::RealtimeGamesStatus) -> impl IntoView {
    let i18n = use_i18n();
    let late = current.started_while_paused;
    view! {
        <p class="font-semibold">
            {if current.paused {
                t_string!(i18n, common.admin.realtime_games.paused)
            } else {
                t_string!(i18n, common.admin.realtime_games.allowed)
            }}
        </p>
        <p>
            {t_string!(i18n, common.admin.realtime_games.in_progress, games = current.live_games)
                .to_string()}
        </p>
        {(current.starting > 0)
            .then(|| {
                view! {
                    <p class="font-semibold">
                        {t_string!(
                            i18n, common.admin.realtime_games.starting, starts = current.starting
                        )
                            .to_string()}
                    </p>
                }
            })}
        {(!late.is_empty())
            .then(|| {
                let count = late.len();
                view! {
                    <div
                        role="alert"
                        class="p-3 space-y-2 text-sm bg-orange-50 rounded border border-orange-400 dark:text-orange-100 text-orange-950 dark:bg-orange-950"
                    >
                        <p class="font-semibold">
                            {t_string!(
                                i18n, common.admin.realtime_games.started_while_paused, games = count
                            )
                                .to_string()}
                        </p>
                        <p>{t!(i18n, common.admin.realtime_games.started_while_paused_reason)}</p>
                        <ul class="list-disc ps-5">
                            {late
                                .into_iter()
                                .map(|game| {
                                    view! {
                                        <li>
                                            <a
                                                class="ui-text-link"
                                                href=format!("/game/{}", game.game_id.0)
                                            >
                                                {t_string!(
                                                    i18n, common.versus_players, white = game.white, black = game.black
                                                )
                                                    .to_string()}
                                            </a>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    </div>
                }
            })}
    }
}

#[component]
fn EditBanner() -> impl IntoView {
    let i18n = use_i18n();
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
                                    if !show_preview() {
                                        t_string!(i18n, common.admin.preview_banner).to_string()
                                    } else {
                                        t_string!(i18n, common.admin.edit_banner).to_string()
                                    }
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
                                            {t!(i18n, common.title)}
                                        </label>
                                        <input
                                            class="ui-field-input"
                                            name="title"
                                            type="text"
                                            prop:value=title
                                            placeholder=move || {
                                                t_string!(i18n, common.admin.banner_title)
                                            }
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
                                            {t!(i18n, common.admin.markdown_cheat_sheet)}
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
                                        {t!(i18n, common.submit)}
                                    </button>
                                    {move || {
                                        if display() {
                                            t_string!(i18n, common.admin.display_banner).to_string()
                                        } else {
                                            t_string!(i18n, common.admin.hide_banner).to_string()
                                        }
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
