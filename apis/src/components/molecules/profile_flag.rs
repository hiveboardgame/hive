use crate::{
    common::{flag_emoji, focus_after_render, localized_error_message, FLAG_CODES},
    components::molecules::hamburger::Hamburger,
    functions::users::player_profile::{ProfileResource, SetProfileFlag},
    i18n::*,
    providers::AuthContext,
};
use leptos::prelude::*;
use uuid::Uuid;

fn region_names(locale: Locale) -> impl Fn(&str) -> String {
    use web_sys::js_sys::{
        Array,
        Intl::{DisplayNames, DisplayNamesOptions, DisplayNamesType},
    };

    let options = DisplayNamesOptions::new();
    options.set_type(DisplayNamesType::Region);
    let locales = Array::of1(&locale.to_string().into());
    let names = DisplayNames::new(&locales, &options);
    move |code| {
        names
            .of(code)
            .map(String::from)
            .unwrap_or_else(|| code.to_owned())
    }
}

#[component]
pub fn ProfileFlag(user_id: Uuid, details: ProfileResource) -> impl IntoView {
    let i18n = use_i18n();
    let identity = expect_context::<AuthContext>().identity;
    let can_edit = move || {
        identity.get().and_then(|viewer| viewer.user_id()) == Some(user_id)
            && details.with(|result| matches!(result, Some(Ok(_))))
    };
    let flag = Memo::new(move |_| details.with(|r| r.as_ref()?.as_ref().ok()?.flag.clone()));
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

    Effect::watch(
        action.version(),
        move |_, _, _| {
            if let Some(Ok(value)) = action.value().get_untracked() {
                details.update(|result| {
                    if let Some(Ok(profile)) = result {
                        profile.flag = value;
                    }
                });
                open.set(false);
                focus_after_render("#profile-flag".into(), focus_guard.get_value());
                search.set(String::new());
            }
        },
        false,
    );

    let emoji = move || flag.with(|flag| flag.as_deref().and_then(flag_emoji));
    let name = Memo::new(move |_| {
        flag.get()
            .map(|code| region_names(i18n.get_locale())(&code))
    });
    let options = Memo::new(move |_| {
        let name = region_names(i18n.get_locale());
        let mut flags: Vec<_> = FLAG_CODES.iter().map(|&code| (code, name(code))).collect();
        flags.sort_by(|a, b| a.1.cmp(&b.1));
        flags
    });
    let filtered = Memo::new(move |_| {
        let query = search.get().trim().to_lowercase();
        options.with(|flags| {
            flags
                .iter()
                .filter(|(code, name)| {
                    code.to_ascii_lowercase().contains(&query)
                        || name.to_lowercase().contains(&query)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    });

    view! {
        <Show
            when=can_edit
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
                    let current = name
                        .get()
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
                    <ForEnumerate
                        each=move || filtered.get()
                        key=|item| item.clone()
                        children=move |index, (code, name)| {
                            let name = StoredValue::new(name);
                            view! {
                                <button
                                    id=move || format!("profile-flag-option-{}", index.get())
                                    tabindex=move || {
                                        if active.get() == index.get() { 0 } else { -1 }
                                    }
                                    on:focus=move |_| {
                                        active.set(index.get_untracked());
                                        highlighted.set(name.get_value());
                                    }
                                    on:keydown=move |event| {
                                        let count = filtered.with_untracked(Vec::len);
                                        let index = index.get_untracked();
                                        let next = match event.key().as_str() {
                                            "ArrowRight" => {
                                                Some((index + 1).min(count.saturating_sub(1)))
                                            }
                                            "ArrowLeft" => Some(index.saturating_sub(1)),
                                            "ArrowDown" => {
                                                Some((index + 6).min(count.saturating_sub(1)))
                                            }
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
                                    on:pointerenter=move |_| highlighted.set(name.get_value())
                                    title=name.get_value()
                                    aria-label=name.get_value()
                                    aria-pressed=move || {
                                        flag.with(|flag| flag.as_deref() == Some(code)).to_string()
                                    }
                                    disabled=move || action.pending().get()
                                    on:click=move |_| {
                                        action
                                            .dispatch(SetProfileFlag {
                                                flag: Some(code.into()),
                                            });
                                    }
                                >
                                    <span class="profile-flag-emoji">{flag_emoji(code)}</span>
                                </button>
                            }
                        }
                    />
                </div>
                <p role="status" aria-live="polite" class="mt-1 text-sm shrink-0">
                    {highlighted}
                </p>
                <a
                    class="text-xs ui-text-link"
                    href="/assets/fonts/profile-flags-LICENSE.txt"
                    target="_blank"
                    rel="noopener"
                >
                    {t!(i18n, profile.flag.credits)}
                </a>
                {move || {
                    action
                        .value()
                        .get()
                        .and_then(Result::err)
                        .map(|err| {
                            view! {
                                <p role="alert" class="mt-2 text-sm shrink-0">
                                    {localized_error_message(i18n, &err.to_string())}
                                </p>
                            }
                        })
                }}
            </Hamburger>
        </Show>
    }
}
