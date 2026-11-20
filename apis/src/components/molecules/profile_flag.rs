use crate::{
    common::{flag_emoji, flag_name, focus_after_render, FLAGS},
    components::molecules::hamburger::Hamburger,
    functions::users::player_profile::{get_player_details, SetProfileFlag},
    i18n::*,
    providers::AuthContext,
};
use leptos::prelude::*;
use uuid::Uuid;

#[component]
pub fn ProfileFlag(user_id: Uuid) -> impl IntoView {
    let i18n = use_i18n();
    let auth_user = expect_context::<AuthContext>().user;
    let is_owner = move || {
        auth_user.with(|viewer| {
            viewer
                .as_ref()
                .is_some_and(|viewer| viewer.user.uid == user_id)
        })
    };
    let flag = RwSignal::new(None::<String>);
    let initial_flag = LocalResource::new(move || get_player_details(user_id));
    let action = ServerAction::<SetProfileFlag>::new();
    let open = RwSignal::new(false);
    let highlighted = RwSignal::new(String::new());
    let active = RwSignal::new(0usize);
    let mounted = ArcRwSignal::new(true);
    let focus_guard = StoredValue::new(mounted.clone());
    on_cleanup({
        let mounted = mounted.clone();
        move || mounted.set(false)
    });
    Effect::new(move |_| {
        if open.get() {
            focus_after_render("#profile-flag-search".into(), mounted.clone());
        }
    });
    let search = RwSignal::new(String::new());

    Effect::new(move |_| {
        if let Some(Ok(value)) = initial_flag.get() {
            flag.set(value.flag);
        }
    });
    Effect::watch(
        action.version(),
        move |_, _, _| {
            if let Some(Ok(value)) = action.value().get_untracked() {
                flag.set(value);
                open.set(false);
                focus_after_render("#profile-flag".into(), focus_guard.get_value());
                search.set(String::new());
            }
        },
        false,
    );

    let emoji = move || flag.with(|flag| flag.as_deref().and_then(flag_emoji));
    let name = move || flag.with(|flag| flag.as_deref().and_then(flag_name));

    view! {
        <Show
            when=is_owner
            fallback=move || {
                view! {
                    <span class="profile-flag-emoji" title=name>
                        {emoji}
                    </span>
                }
            }
        >
            <Hamburger
                hamburger_show=open
                button_style="ui-button ui-button-ghost ui-button-sm !text-xl"
                dropdown_style="fixed top-24 left-1/2 z-20 flex flex-col overflow-y-auto p-2 w-64 max-h-[calc(100dvh-7rem)] -translate-x-1/2 sm:absolute sm:top-full sm:mt-1"
                id="profile-flag"
                aria_label=Signal::derive(move || {
                    let current = name()
                        .map(str::to_string)
                        .unwrap_or_else(|| t_string!(i18n, profile.flag.none).to_string());
                    format!("{}: {current}", t_string!(i18n, profile.flag.choose))
                })
                popup_role="dialog"
                popup_aria_label=Signal::derive(move || {
                    t_string!(i18n, profile.flag.choose).to_string()
                })
                content=move || {
                    view! {
                        <span class="profile-flag-emoji">
                            {emoji().unwrap_or_else(|| "🏳️".into())}
                        </span>
                    }
                }
            >
                <input
                    id="profile-flag-search"
                    type="search"
                    class="mb-2 w-full shrink-0 ui-field-input"
                    aria-label=move || t_string!(i18n, profile.flag.search)
                    placeholder=move || t_string!(i18n, profile.flag.search)
                    on:keydown=move |event| {
                        if event.key() == "ArrowDown" {
                            event.prevent_default();
                            focus_after_render(
                                "#profile-flag-option-0".into(),
                                focus_guard.get_value(),
                            );
                        }
                    }
                    prop:value=search
                    on:input=move |event| {
                        search.set(event_target_value(&event));
                        active.set(0);
                        highlighted.set(String::new());
                    }
                />
                <button
                    type="button"
                    class="w-full shrink-0 ui-dropdown-link"
                    disabled=move || action.pending().get()
                    on:click=move |_| {
                        action.dispatch(SetProfileFlag { flag: None });
                    }
                >
                    {t!(i18n, profile.flag.none)}
                </button>
                <div class="grid overflow-y-auto grid-cols-6 gap-1 min-h-0 max-h-52">
                    <For
                        each=move || {
                            let query = search.get().trim().to_lowercase();
                            FLAGS
                                .iter()
                                .copied()
                                .filter(|(code, name)| {
                                    code.to_lowercase().contains(&query)
                                        || name.to_lowercase().contains(&query)
                                })
                                .enumerate()
                                .collect::<Vec<_>>()
                        }
                        key=|(index, (code, _))| (*index, *code)
                        let:item
                    >
                        <button
                            id=format!("profile-flag-option-{}", item.0)
                            tabindex=move || if active.get() == item.0 { 0 } else { -1 }
                            on:focus=move |_| {
                                active.set(item.0);
                                highlighted.set(item.1.1.into());
                            }
                            on:keydown=move |event| {
                                let count = FLAGS
                                    .iter()
                                    .filter(|(code, name)| {
                                        let query = search.get_untracked().trim().to_lowercase();
                                        code.to_lowercase().contains(&query)
                                            || name.to_lowercase().contains(&query)
                                    })
                                    .count();
                                let index = item.0;
                                let next = match event.key().as_str() {
                                    "ArrowRight" => Some((index + 1).min(count.saturating_sub(1))),
                                    "ArrowLeft" => Some(index.saturating_sub(1)),
                                    "ArrowDown" => Some((index + 6).min(count.saturating_sub(1))),
                                    "ArrowUp" => Some(index.saturating_sub(6)),
                                    "Home" => Some(0),
                                    "End" => Some(count.saturating_sub(1)),
                                    _ => None,
                                };
                                if let Some(next) = next {
                                    event.prevent_default();
                                    focus_after_render(
                                        format!("#profile-flag-option-{next}"),
                                        focus_guard.get_value(),
                                    );
                                }
                            }
                            type="button"
                            class="text-xl ui-button ui-button-ghost ui-button-sm !px-1"
                            on:pointerenter=move |_| highlighted.set(item.1.1.into())
                            title=item.1.1
                            aria-label=item.1.1
                            aria-pressed=move || {
                                flag.with(|flag| flag.as_deref() == Some(item.1.0)).to_string()
                            }
                            disabled=move || action.pending().get()
                            on:click=move |_| {
                                action
                                    .dispatch(SetProfileFlag {
                                        flag: Some(item.1.0.into()),
                                    });
                            }
                        >
                            <span class="profile-flag-emoji">{flag_emoji(item.1.0)}</span>
                        </button>
                    </For>
                </div>
                <p role="status" aria-live="polite" class="mt-1 text-sm shrink-0">
                    {highlighted}
                </p>
                <Show when=move || action.value().with(|value| matches!(value, Some(Err(_))))>
                    <p role="alert" class="mt-2 text-sm shrink-0">
                        {t!(i18n, profile.flag.save_failed)}
                    </p>
                </Show>
            </Hamburger>
        </Show>
    }
}
