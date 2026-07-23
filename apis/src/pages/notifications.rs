use crate::{
    common::with_class,
    components::{
        atoms::simple_switch::SimpleSwitchWithCallback,
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::panel::Panel,
    },
    functions::{
        devices::{list_devices, unregister_device},
        notification_preferences::{get_notification_preferences, set_notification_preferences},
        oauth::{get_discord_handle, DiscordHandleStatus},
    },
    i18n::*,
    providers::ApiRequestsProvider,
    pwa,
    responses::NotificationPreferencesResponse,
};
use leptos::{prelude::*, task::spawn_local};
use leptos_i18n::I18nContext;
use shared_types::{NotificationCategory, CHANNEL_DISCORD, CHANNEL_PUSH};

const EVENTS: [NotificationCategory; 6] = [
    NotificationCategory::YourTurn,
    NotificationCategory::Challenges,
    NotificationCategory::GameEnded,
    NotificationCategory::Tournament,
    NotificationCategory::Schedules,
    NotificationCategory::Dms,
];

#[component]
pub fn Notifications() -> impl IntoView {
    let i18n = use_i18n();
    let prefs = RwSignal::new(NotificationPreferencesResponse::default());
    let loaded = RwSignal::new(false);
    let device_refresh = RwSignal::new(0_u32);
    let api = expect_context::<ApiRequestsProvider>();
    let oauth = move |_: leptos::ev::MouseEvent| {
        api.0.get().link_discord();
    };
    let discord_name = Action::new(move |_: &()| async { get_discord_handle().await });
    Effect::new(move |_| {
        discord_name.dispatch(());
    });

    let resource = OnceResource::new(get_notification_preferences());
    Effect::new(move |_| {
        if let Some(Ok(p)) = resource.get() {
            prefs.set(p);
            loaded.set(true);
        }
    });
    let discord_disabled = Signal::derive(move || {
        !matches!(
            discord_name.value().get(),
            Some(Ok(DiscordHandleStatus::Linked(_)))
        )
    });
    let push_delivery_ready = RwSignal::new(false);
    let push_disabled = Signal::derive(move || !push_delivery_ready.get());

    let save = Action::new(move |_: &()| {
        let payload = prefs.get_untracked();
        async move { set_notification_preferences(payload).await }
    });

    let resave = RwSignal::new(false);
    let trigger = Callback::new(move |_| {
        if save.pending().get_untracked() {
            resave.set(true);
        } else {
            save.dispatch(());
        }
    });
    Effect::new(move |was_pending: Option<bool>| {
        let pending = save.pending().get();
        if was_pending == Some(true) && !pending && resave.get_untracked() {
            resave.set(false);
            save.dispatch(());
        }
        pending
    });
    let saving = save.pending();
    let save_result = save.value();
    let save_message = move || {
        if saving.get() {
            return Some(
                view! { <p class="text-gray-500 dark:text-gray-400">{t!(i18n, notifications.page.saving)}</p> }
                .into_any(),
            );
        }
        match save_result.get() {
            Some(Ok(_)) => Some(
                view! { <p class="text-green-600 dark:text-green-400">{t!(i18n, notifications.page.saved)}</p> }
                .into_any(),
            ),
            Some(Err(e)) => Some(
                view! {
                    <p class="text-red-600 dark:text-red-400">
                        {t!(i18n, notifications.page.save_failed)} " " {e.to_string()}
                    </p>
                }
                .into_any(),
            ),
            None => None,
        }
    };

    view! {
        <PageShell variant=PageShellVariant::Content class="max-w-4xl">
            <PageHeader
                title=move || t_string!(i18n, notifications.page.heading)
                subtitle="Push, Discord, and device delivery settings."
            />
            <Suspense fallback=move || {
                view! {
                    <Panel>
                        <p class="ui-field-helper">{t!(i18n, notifications.page.loading)}</p>
                    </Panel>
                }
            }>
                <Show when=loaded>
                    <Panel title="Notification preferences" body_class="space-y-4">
                        <p class="ui-field-helper">
                            {t!(i18n, notifications.page.intro_before_link)}
                            <a href="#discord-notifications" class="ui-text-link">
                                {t!(i18n, notifications.page.account_link)}
                            </a> {t!(i18n, notifications.page.intro_after_link)}
                        </p>
                        <BrowserPushSetup
                            device_refresh=device_refresh
                            push_delivery_ready=push_delivery_ready
                        />
                        <div class="overflow-x-auto">
                            <div class="grid gap-x-2 gap-y-3 items-center p-2 min-w-0 rounded-lg border sm:gap-x-3 sm:p-3 max-[320px]:min-w-[18rem] grid-cols-[minmax(0,1fr)_3rem_3.75rem] border-black/5 bg-odd-light/70 dark:border-white/10 dark:bg-surface-muted">
                                <div></div>
                                <div class="text-xs font-semibold text-center text-gray-700 uppercase dark:text-gray-300">
                                    {t!(i18n, notifications.page.col_push)}
                                </div>
                                <div class="text-xs font-semibold text-center text-gray-700 uppercase dark:text-gray-300">
                                    {t!(i18n, notifications.page.col_discord)}
                                </div>
                                {EVENTS
                                    .iter()
                                    .map(|&category| {
                                        view! {
                                            <div class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                                {move || notification_event_label(i18n, category)}
                                            </div>
                                            <div class="flex justify-center">
                                                <ChannelSwitch
                                                    prefs=prefs
                                                    category=category
                                                    channel=CHANNEL_PUSH
                                                    disabled=push_disabled
                                                    trigger=trigger
                                                />
                                            </div>
                                            <div class="flex justify-center">
                                                <ChannelSwitch
                                                    prefs=prefs
                                                    category=category
                                                    channel=CHANNEL_DISCORD
                                                    disabled=discord_disabled
                                                    trigger=trigger
                                                />
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                        <Show when=push_disabled>
                            <p class="ui-field-helper">
                                "Enable browser notifications before choosing push delivery."
                            </p>
                        </Show>
                        <Show when=discord_disabled>
                            <p class="ui-field-helper">
                                "Join the HiveGame Discord server and link your Discord account below to enable Discord notification settings."
                            </p>
                        </Show>
                        <div class="text-sm min-h-5">{save_message}</div>
                    </Panel>

                    <DevicesSection device_refresh=device_refresh />

                    <div id="discord-notifications">
                        <Panel title="Discord integration" body_class="space-y-3">
                            <div class="ui-setting-group">
                                <div class="flex flex-col gap-3 sm:flex-row sm:justify-between sm:items-center">
                                    <div class="min-w-0">
                                        <p class="ui-field-label">"Step 1"</p>
                                        <p class="ui-field-helper">
                                            "Join the HiveGame Discord server."
                                        </p>
                                    </div>
                                    <a
                                        href="https://discord.gg/7EwNTJnfab"
                                        target="_blank"
                                        rel="noopener noreferrer"
                                        class="w-full ui-button ui-button-primary ui-button-md no-link-style sm:w-fit"
                                    >
                                        "Join Discord"
                                    </a>
                                </div>
                            </div>
                            <div class="ui-setting-group">
                                <div class="flex flex-col gap-4 sm:flex-row sm:justify-between sm:items-end">
                                    <div class="flex-1 min-w-0">
                                        <p class="ui-field-label">"Step 2"</p>
                                        <p class="ui-field-helper">
                                            "Link this HiveGame account to Discord."
                                        </p>
                                        <div class=with_class(
                                            "ui-field-input",
                                            "mt-2 py-2",
                                        )>
                                            {move || {
                                                match discord_name.value().get() {
                                                    Some(Ok(DiscordHandleStatus::Linked(name))) => {
                                                        view! {
                                                            <span class="font-mono text-grasshopper-green">
                                                                {name.clone()}
                                                            </span>
                                                        }
                                                            .into_any()
                                                    }
                                                    Some(Ok(DiscordHandleStatus::NotLinked)) => {
                                                        view! {
                                                            <span class="ui-field-helper">
                                                                "No Discord account linked"
                                                            </span>
                                                        }
                                                            .into_any()
                                                    }
                                                    Some(Ok(DiscordHandleStatus::NotLoggedIn)) => {
                                                        view! {
                                                            <span class="ui-field-helper">
                                                                "Sign in to link Discord"
                                                            </span>
                                                        }
                                                            .into_any()
                                                    }
                                                    Some(Ok(DiscordHandleStatus::Unavailable)) => {
                                                        view! {
                                                            <span class="ui-field-error">
                                                                "Discord status temporarily unavailable"
                                                            </span>
                                                        }
                                                            .into_any()
                                                    }
                                                    Some(Err(_)) => {
                                                        view! {
                                                            <span class="ui-field-error">
                                                                "Error loading Discord name"
                                                            </span>
                                                        }
                                                            .into_any()
                                                    }
                                                    None => {
                                                        view! {
                                                            <span class="ui-field-helper">
                                                                "No Discord account linked"
                                                            </span>
                                                        }
                                                            .into_any()
                                                    }
                                                }
                                            }}
                                        </div>
                                    </div>
                                    <button
                                        type="button"
                                        class="w-full sm:w-auto ui-button ui-button-primary ui-button-md"
                                        on:click=oauth
                                    >
                                        "Link Discord account"
                                    </button>
                                </div>
                            </div>
                        </Panel>
                    </div>
                </Show>
            </Suspense>
        </PageShell>
    }
}

#[component]
fn DevicesSection(device_refresh: RwSignal<u32>) -> impl IntoView {
    let i18n = use_i18n();
    let devices = LocalResource::new(move || {
        device_refresh.track();
        async move {
            let endpoint = pwa::current_endpoint().await;
            list_devices(endpoint).await.unwrap_or_default()
        }
    });

    let remove = move |id: String, is_current: bool| {
        spawn_local(async move {
            let _ = if is_current {
                pwa::unsubscribe().await
            } else {
                unregister_device(id).await.map_err(|e| e.to_string())
            };
            device_refresh.update(|n| *n += 1);
        });
    };

    view! {
        {move || {
            devices
                .get()
                .filter(|list| !list.is_empty())
                .map(|list| {
                    let rows = list
                        .into_iter()
                        .map(|d| {
                            let id = d.id.clone();
                            let is_current = d.is_current;
                            let platform = match d.platform.as_str() {
                                "web" => {
                                    t_string!(i18n, notifications.devices.platform_web).to_string()
                                }
                                other => other.to_string(),
                            };
                            view! {
                                <div class="flex gap-3 justify-between items-center py-2 border-b last:border-b-0 border-black/10 dark:border-white/10">
                                    <div class="min-w-0">
                                        <span class="font-semibold text-gray-900 dark:text-gray-100">
                                            {platform}
                                        </span>
                                        <Show when=move || is_current>
                                            <span class="ml-1 text-xs font-medium text-pillbug-teal">
                                                {t!(i18n, notifications.devices.this_browser)}
                                            </span>
                                        </Show>
                                        <div class="text-xs text-gray-600 dark:text-gray-400">
                                            {t!(i18n, notifications.devices.last_active)} " "
                                            {d.last_seen.clone()}
                                        </div>
                                    </div>
                                    <button
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        on:click=move |_| remove(id.clone(), is_current)
                                    >
                                        {t!(i18n, notifications.devices.remove)}
                                    </button>
                                </div>
                            }
                        })
                        .collect_view();
                    view! {
                        <Panel
                            title=move || t_string!(i18n, notifications.devices.heading)
                            body_class="space-y-3"
                        >
                            <p class="ui-field-helper">{t!(i18n, notifications.devices.body)}</p>
                            <div>{rows}</div>
                        </Panel>
                    }
                })
        }}
    }
}

#[derive(Clone, Copy, Default)]
struct PushStatus {
    supported: bool,
    subscribed: bool,
    has_devices: bool,
    ios_hint: bool,
}

#[component]
fn BrowserPushSetup(
    device_refresh: RwSignal<u32>,
    push_delivery_ready: RwSignal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let test_msg = RwSignal::new(None::<Result<(), String>>);

    let status = LocalResource::new(move || {
        device_refresh.track();
        async move {
            if !pwa::supported() {
                return PushStatus {
                    ios_hint: pwa::ios_needs_install(),
                    ..Default::default()
                };
            }
            if !pwa::push_available().await {
                return PushStatus::default();
            }
            let endpoint = pwa::current_endpoint().await;
            let devices = list_devices(endpoint.clone()).await.unwrap_or_default();
            let subscribed = devices.iter().any(|d| d.is_current);
            PushStatus {
                supported: true,
                has_devices: !devices.is_empty(),
                subscribed,
                ios_hint: false,
            }
        }
    });
    Effect::new(move |_| {
        push_delivery_ready.set(status.get().map(|s| s.has_devices).unwrap_or(false));
    });
    let supported = move || status.get().map(|s| s.supported).unwrap_or(false);
    let subscribed = move || status.get().map(|s| s.subscribed).unwrap_or(false);
    let has_devices = move || status.get().map(|s| s.has_devices).unwrap_or(false);
    let ios_hint = move || status.get().map(|s| s.ios_hint).unwrap_or(false);
    let loaded = move || status.get().is_some();

    let toggle = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(None);
        let enable = !subscribed();
        spawn_local(async move {
            let result = if enable {
                pwa::subscribe().await
            } else {
                pwa::unsubscribe().await
            };
            match result {
                Ok(()) => device_refresh.update(|n| *n += 1),
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let test = move |_| {
        test_msg.set(None);
        spawn_local(async move {
            test_msg.set(Some(pwa::send_test().await));
        });
    };

    view! {
        <div class="ui-setting-group">
            <div class="flex flex-col gap-4 sm:flex-row sm:justify-between sm:items-end">
                <div class="flex-1 min-w-0">
                    <p class="ui-field-label">{t!(i18n, notifications.browser.heading)}</p>
                    <Show
                        when=ios_hint
                        fallback=move || {
                            view! {
                                <p class="ui-field-helper">
                                    {t!(i18n, notifications.browser.body)}
                                </p>
                            }
                        }
                    >
                        <p class="ui-field-helper">{t!(i18n, notifications.ios.body)}</p>
                    </Show>
                    <Show when=loaded>
                        <p class="mt-2 ui-field-helper">
                            {move || {
                                if has_devices() {
                                    if subscribed() {
                                        "Push delivery is enabled for this browser."
                                    } else {
                                        "Push delivery is enabled on another browser."
                                    }
                                } else if supported() {
                                    "Enable browser notifications before choosing push delivery."
                                } else {
                                    "Browser notifications are not available in this browser."
                                }
                            }}
                        </p>
                    </Show>
                </div>
                <div class="flex flex-col gap-2 items-stretch sm:flex-row sm:flex-wrap sm:items-center">
                    <Show when=supported>
                        <button
                            class="w-full sm:w-auto ui-button ui-button-primary ui-button-md"
                            disabled=busy
                            on:click=toggle
                        >
                            {move || {
                                if busy.get() {
                                    t_string!(i18n, notifications.browser.working)
                                } else if subscribed() {
                                    t_string!(i18n, notifications.browser.disable)
                                } else {
                                    t_string!(i18n, notifications.browser.enable)
                                }
                            }}
                        </button>
                    </Show>
                    <Show when=has_devices>
                        <button
                            class="w-full sm:w-auto ui-button ui-button-secondary ui-button-md"
                            on:click=test
                        >
                            {t!(i18n, notifications.browser.send_test)}
                        </button>
                    </Show>
                </div>
            </div>
            <Show when=move || error.with(Option::is_some)>
                <p class="mt-2 ui-field-error">{move || error.get().unwrap_or_default()}</p>
            </Show>
            {move || {
                test_msg
                    .get()
                    .map(|r| match r {
                        Ok(()) => {
                            view! {
                                <p class="mt-2 text-sm font-medium text-grasshopper-green">
                                    {t!(i18n, notifications.browser.test_sent)}
                                </p>
                            }
                                .into_any()
                        }
                        Err(e) => {
                            view! {
                                <p class="mt-2 ui-field-error">
                                    {t!(i18n, notifications.browser.test_failed)} " " {e}
                                </p>
                            }
                                .into_any()
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn ChannelSwitch(
    prefs: RwSignal<NotificationPreferencesResponse>,
    category: NotificationCategory,
    channel: &'static str,
    #[prop(optional)] disabled: Signal<bool>,
    trigger: Callback<()>,
) -> impl IntoView {
    let checked = Signal::derive(move || {
        !disabled.get() && prefs.with(|p| p.channels(category).iter().any(|c| c == channel))
    });
    let action = Callback::new(move |_| {
        if disabled.get_untracked() {
            return;
        }
        prefs.update(|p| {
            let v = p.channels_mut(category);
            if v.iter().any(|c| c == channel) {
                v.retain(|c| c != channel);
            } else {
                v.push(channel.to_string());
            }
        });
        trigger.run(());
    });
    view! { <SimpleSwitchWithCallback checked=checked disabled=disabled action=action /> }
}

fn notification_event_label(
    i18n: I18nContext<Locale, I18nKeys>,
    category: NotificationCategory,
) -> String {
    match category {
        NotificationCategory::YourTurn => t_string!(i18n, notifications.events.your_turn),
        NotificationCategory::Challenges => t_string!(i18n, notifications.events.challenges),
        NotificationCategory::GameEnded => t_string!(i18n, notifications.events.game_ended),
        NotificationCategory::Tournament => t_string!(i18n, notifications.events.tournament),
        NotificationCategory::Schedules => t_string!(i18n, notifications.events.schedules),
        NotificationCategory::Dms => t_string!(i18n, notifications.events.dms),
        NotificationCategory::GeneralChat => t_string!(i18n, notifications.events.general_chat),
    }
    .to_string()
}
