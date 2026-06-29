use crate::{
    components::atoms::simple_switch::SimpleSwitchWithCallback,
    functions::{
        devices::{list_devices, unregister_device},
        notification_preferences::{get_notification_preferences, set_notification_preferences},
        oauth::get_discord_handle,
    },
    i18n::*,
    providers::ApiRequestsProvider,
    pwa,
    responses::NotificationPreferencesResponse,
};
use leptos::{prelude::*, task::spawn_local};
use shared_types::{NotificationCategory, CHANNEL_DISCORD, CHANNEL_PUSH};

const SECTION_CARD: &str = "px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg \
                            bg-stone-300 border-stone-400 dark:bg-slate-800 dark:border-slate-600";
const HEADING: &str = "mb-4 text-xl font-bold text-center text-indigo-600 dark:text-indigo-400";

const EVENTS: [NotificationCategory; 5] = [
    NotificationCategory::YourTurn,
    NotificationCategory::Challenges,
    NotificationCategory::GameEnded,
    NotificationCategory::Tournament,
    NotificationCategory::Schedules,
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
        <div class="px-4 pb-20 mx-auto max-w-md pt-page">
            <Suspense fallback=move || {
                view! { <p class="dark:text-white">{t!(i18n, notifications.page.loading)}</p> }
            }>
                <Show when=loaded>
                    <div class=SECTION_CARD>
                        <h2 class=HEADING>{t!(i18n, notifications.page.heading)}</h2>
                        <p class="mb-3 text-sm text-gray-700 dark:text-gray-300">
                            {t!(i18n, notifications.page.intro_before_link)}
                            <a href="#discord-notifications" class="underline">
                                {t!(i18n, notifications.page.account_link)}
                            </a> {t!(i18n, notifications.page.intro_after_link)}
                        </p>
                        <div class="grid gap-x-3 gap-y-3 items-center grid-cols-[1fr_auto_auto]">
                            <div></div>
                            <div class="text-sm font-semibold text-center dark:text-white">
                                {t!(i18n, notifications.page.col_push)}
                            </div>
                            <div class="text-sm font-semibold text-center dark:text-white">
                                {t!(i18n, notifications.page.col_discord)}
                            </div>
                            {EVENTS
                                .iter()
                                .map(|&category| {
                                    let label = match category {
                                        NotificationCategory::YourTurn => {
                                            t_string!(i18n, notifications.events.your_turn)
                                        }
                                        NotificationCategory::Challenges => {
                                            t_string!(i18n, notifications.events.challenges)
                                        }
                                        NotificationCategory::GameEnded => {
                                            t_string!(i18n, notifications.events.game_ended)
                                        }
                                        NotificationCategory::Tournament => {
                                            t_string!(i18n, notifications.events.tournament)
                                        }
                                        NotificationCategory::Schedules => {
                                            t_string!(i18n, notifications.events.schedules)
                                        }
                                        NotificationCategory::Dms => {
                                            t_string!(i18n, notifications.events.dms)
                                        }
                                    }
                                        .to_string();
                                    view! {
                                        <div class="text-sm dark:text-white">{label}</div>
                                        <div class="flex justify-center">
                                            <ChannelSwitch
                                                prefs=prefs
                                                category=category
                                                channel=CHANNEL_PUSH
                                                trigger=trigger
                                            />
                                        </div>
                                        <div class="flex justify-center">
                                            <ChannelSwitch
                                                prefs=prefs
                                                category=category
                                                channel=CHANNEL_DISCORD
                                                trigger=trigger
                                            />
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <p class="mt-4 text-xs italic text-gray-600 dark:text-gray-400">
                            {t!(i18n, notifications.page.email_coming_soon)}
                        </p>
                        <div class="mt-3 h-5 text-sm text-center">{save_message}</div>
                    </div>

                    <div id="discord-notifications" class=SECTION_CARD>
                        <h2 class=HEADING>"🤖 Discord Integration"</h2>

                        <div class="p-4 mb-6 bg-blue-50 rounded-lg border border-blue-200 dark:border-blue-700 dark:bg-blue-900/30">
                            <h3 class="mb-2 font-semibold text-blue-800 dark:text-blue-200">
                                "📬 Busybee Messages"
                            </h3>
                            <p class="mb-3 text-sm text-blue-700 dark:text-blue-300">
                                "Get instant notifications about tournament starts, game updates, and important events directly in Discord!"
                            </p>
                            <div class="mb-3 text-xs text-blue-600 dark:text-blue-400">
                                "⚠️ To receive busybee messages, you must:"
                            </div>
                            <ul class="ml-4 space-y-1 text-xs text-blue-600 dark:text-blue-400">
                                <li>"• Join our Discord server"</li>
                                <li>"• Link your Discord account below"</li>
                            </ul>
                        </div>

                        <div class="mb-6 text-center">
                            <div class="mb-3">
                                <span class="text-sm font-medium text-gray-700 dark:text-gray-300">
                                    "Not in our Discord yet?"
                                </span>
                            </div>
                            <a
                                href="https://discord.gg/7EwNTJnfab"
                                target="_blank"
                                rel="noopener noreferrer"
                                class="inline-flex items-center py-2 px-4 font-bold text-white bg-purple-600 rounded-lg shadow-lg transition-all duration-300 cursor-pointer hover:bg-purple-700 focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 focus:outline-none active:scale-95 no-link-style"
                            >
                                <span class="mr-2 text-white">"💬"</span>
                                <span class="text-white">"Join HiveGame Discord"</span>
                                <span class="ml-2 text-white">"↗"</span>
                            </a>
                        </div>

                        <div class="mb-4">
                            <label class="block mb-2 font-semibold text-gray-700 dark:text-gray-300">
                                "🔗 Linked Discord Account"
                            </label>
                            <div class="p-3 bg-gray-100 rounded-lg border dark:bg-gray-700">
                                {move || {
                                    match discord_name.value().get() {
                                        Some(Ok(name)) => {
                                            view! {
                                                <span class="font-mono text-green-700 dark:text-green-400">
                                                    {name.clone()}
                                                </span>
                                            }
                                                .into_any()
                                        }
                                        Some(Err(_)) => {
                                            view! {
                                                <span class="italic text-red-500 dark:text-red-400">
                                                    "Error loading Discord name"
                                                </span>
                                            }
                                                .into_any()
                                        }
                                        None => {
                                            view! {
                                                <span class="italic text-gray-500 dark:text-gray-400">
                                                    "No Discord account linked"
                                                </span>
                                            }
                                                .into_any()
                                        }
                                    }
                                }}
                            </div>
                        </div>

                        <div class="text-center">
                            <button
                                class="py-3 px-4 w-full font-bold text-white bg-purple-600 rounded-lg transition-all duration-300 cursor-pointer hover:bg-purple-700 focus:ring-2 focus:ring-purple-500 focus:ring-offset-2 focus:outline-none active:scale-95"
                                on:click=oauth
                            >
                                <span class="mr-2">"🔗"</span>
                                "Link Discord Account"
                            </button>
                            <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
                                "This will redirect you to Discord for authorization"
                            </div>
                        </div>
                    </div>

                    <WebPushSection device_refresh=device_refresh />
                    <DevicesSection device_refresh=device_refresh />
                </Show>
            </Suspense>
        </div>
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
                                <div class="flex gap-2 justify-between items-center py-2 border-b last:border-b-0 border-stone-400 dark:border-slate-600">
                                    <div class="min-w-0">
                                        <span class="font-semibold dark:text-white">
                                            {platform}
                                        </span>
                                        <Show when=move || is_current>
                                            <span class="ml-1 text-xs text-indigo-600 dark:text-indigo-400">
                                                {t!(i18n, notifications.devices.this_browser)}
                                            </span>
                                        </Show>
                                        <div class="text-xs text-gray-600 dark:text-gray-400">
                                            {t!(i18n, notifications.devices.last_active)} " "
                                            {d.last_seen.clone()}
                                        </div>
                                    </div>
                                    <button
                                        class="py-1 px-3 text-sm rounded border dark:text-white border-stone-400 dark:border-slate-500 dark:hover:bg-slate-700 hover:bg-stone-200"
                                        on:click=move |_| remove(id.clone(), is_current)
                                    >
                                        {t!(i18n, notifications.devices.remove)}
                                    </button>
                                </div>
                            }
                        })
                        .collect_view();
                    view! {
                        <div class=SECTION_CARD>
                            <h2 class=HEADING>{t!(i18n, notifications.devices.heading)}</h2>
                            <p class="mb-3 text-sm text-gray-700 dark:text-gray-300">
                                {t!(i18n, notifications.devices.body)}
                            </p>
                            {rows}
                        </div>
                    }
                })
        }}
    }
}

#[derive(Clone, Copy, Default)]
struct PushStatus {
    supported: bool,
    subscribed: bool,
    ios_hint: bool,
}

#[component]
fn WebPushSection(device_refresh: RwSignal<u32>) -> impl IntoView {
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
            let subscribed = match &endpoint {
                Some(_) => list_devices(endpoint.clone())
                    .await
                    .map(|devs| devs.iter().any(|d| d.is_current))
                    .unwrap_or(false),
                None => false,
            };
            PushStatus {
                supported: true,
                subscribed,
                ios_hint: false,
            }
        }
    });
    let supported = move || status.get().map(|s| s.supported).unwrap_or(false);
    let subscribed = move || status.get().map(|s| s.subscribed).unwrap_or(false);
    let ios_hint = move || status.get().map(|s| s.ios_hint).unwrap_or(false);

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
        <Show when=ios_hint>
            <div class=SECTION_CARD>
                <h2 class=HEADING>{t!(i18n, notifications.ios.heading)}</h2>
                <p class="text-sm text-gray-700 dark:text-gray-300">
                    {t!(i18n, notifications.ios.body)}
                </p>
            </div>
        </Show>
        <Show when=supported>
            <div class=SECTION_CARD>
                <h2 class=HEADING>{t!(i18n, notifications.browser.heading)}</h2>
                <p class="mb-3 text-sm text-gray-700 dark:text-gray-300">
                    {t!(i18n, notifications.browser.body)}
                </p>
                <div class="flex flex-col gap-2 justify-center items-stretch sm:flex-row sm:flex-wrap sm:items-center">
                    <button
                        class="py-2 px-6 w-full font-bold text-white rounded shadow sm:w-auto disabled:opacity-50 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
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
                    <Show when=subscribed>
                        <button
                            class="py-2 px-4 w-full font-bold rounded border shadow sm:w-auto dark:text-white border-stone-400 dark:border-slate-500 dark:hover:bg-slate-700 hover:bg-stone-200"
                            on:click=test
                        >
                            {t!(i18n, notifications.browser.send_test)}
                        </button>
                    </Show>
                </div>
                <Show when=move || error.with(Option::is_some)>
                    <p class="mt-2 text-sm text-center text-red-600 dark:text-red-400">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>
                {move || {
                    test_msg
                        .get()
                        .map(|r| match r {
                            Ok(()) => {
                                view! {
                                    <p class="mt-2 text-sm text-center text-green-600 dark:text-green-400">
                                        {t!(i18n, notifications.browser.test_sent)}
                                    </p>
                                }
                                    .into_any()
                            }
                            Err(e) => {
                                view! {
                                    <p class="mt-2 text-sm text-center text-red-600 dark:text-red-400">
                                        {t!(i18n, notifications.browser.test_failed)} " " {e}
                                    </p>
                                }
                                    .into_any()
                            }
                        })
                }}
            </div>
        </Show>
    }
}

#[component]
fn ChannelSwitch(
    prefs: RwSignal<NotificationPreferencesResponse>,
    category: NotificationCategory,
    channel: &'static str,
    trigger: Callback<()>,
) -> impl IntoView {
    let checked =
        Signal::derive(move || prefs.with(|p| p.channels(category).iter().any(|c| c == channel)));
    let action = Callback::new(move |_| {
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
    view! { <SimpleSwitchWithCallback checked=checked action=action /> }
}
