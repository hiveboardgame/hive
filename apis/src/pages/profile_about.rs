use crate::{
    functions::users::player_profile::{
        get_bio_moderation,
        BioModerationDetails,
        ProfileDetails,
        SaveBioModeration,
        SavePlayerDetails,
    },
    i18n::*,
    profile_bio::bio_to_html,
    profile_catalog::{catalog, flag_emoji},
    providers::AuthContext,
    security::csrf::ActionForm,
};
use leptos::prelude::*;
use uuid::Uuid;

#[component]
pub fn ProfileAbout(
    details: LocalResource<Result<ProfileDetails, ServerFnError>>,
    user_id: Uuid,
) -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>().user;
    let owner =
        move || auth.with(|user| user.as_ref().is_some_and(|user| user.user.uid == user_id));
    let admin = move || auth.with(|user| user.as_ref().is_some_and(|user| user.user.admin));
    let saved = RwSignal::new(false);
    view! {
        <div class="mx-1 mt-3 w-full max-w-4xl text-sm dark:text-white">
            {move || {
                details
                    .get()
                    .map(|result| match result {
                        Ok(data) => {
                            let has_details = !data.bio.is_empty();
                            view! {
                                <Show when=move || owner() || admin() || has_details>
                                    <details
                                        open
                                        class="relative rounded-xl border border-gray-300 dark:border-gray-600 group/profile-about"
                                    >
                                        <summary
                                            class="block py-2 px-3 font-semibold list-none cursor-pointer [&::-webkit-details-marker]:hidden group-open/profile-about:absolute group-open/profile-about:right-0 group-open/profile-about:top-0 group-open/profile-about:z-10 group-open/profile-about:p-3"
                                            aria-label=move || t_string!(i18n, profile.about.title)
                                        >
                                            <span
                                                aria-hidden="true"
                                                class="inline-block group-open/profile-about:rotate-90"
                                            >
                                                "▸"
                                            </span>
                                            <span class="ml-1 group-open/profile-about:hidden">
                                                {t!(i18n, profile.about.title)}
                                            </span>
                                        </summary>
                                        <div class="overflow-y-auto py-3 pr-12 pl-3 max-h-[50vh]">
                                            <Show when=move || saved.get()>
                                                <p role="status" class="mb-2">
                                                    {t!(i18n, profile.about.saved)}
                                                </p>
                                            </Show>
                                            <ProfileDetailsPanel
                                                data=data.clone()
                                                owner=Signal::derive(owner)
                                                admin=Signal::derive(admin)
                                                user_id=user_id
                                                on_saved=Callback::new(move |()| {
                                                    saved.set(true);
                                                    details.refetch();
                                                })
                                            />
                                        </div>
                                    </details>
                                </Show>
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! {
                                <p role="alert">
                                    {t!(i18n, profile.about.load_error)}
                                    <button
                                        class="ml-2 underline"
                                        on:click=move |_| details.refetch()
                                    >
                                        {t!(i18n, profile.about.retry)}
                                    </button>
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
fn ProfileDetailsPanel(
    data: ProfileDetails,
    owner: Signal<bool>,
    admin: Signal<bool>,
    user_id: Uuid,
    on_saved: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let editing = RwSignal::new(false);
    let moderating = RwSignal::new(false);
    let bio_blocked = data.bio_blocked;
    let action = ServerAction::<SavePlayerDetails>::new();
    let flag = RwSignal::new(data.flag.clone().unwrap_or_default());
    let bio = RwSignal::new(data.bio.clone());
    let original = StoredValue::new(data.clone());
    let reset = move || {
        let data = original.get_value();
        flag.set(data.flag.unwrap_or_default());
        bio.set(data.bio);
        action.clear();
    };
    Effect::watch(
        action.version(),
        move |_, _, _| {
            if action
                .value()
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                editing.set(false);
                on_saved.run(());
            }
        },
        false,
    );
    let about_html = bio_to_html(&data.bio);
    view! {
        <Show when=move || owner.get() && bio_blocked>
            <p role="status" class="mb-2 text-sm">
                {t!(i18n, profile.about.bio_blocked)}
            </p>
        </Show>
        <Show
            when=move || !editing.get()
            fallback=move || {
                view! {
                    <ActionForm action=action attr:class="space-y-3">
                        <p class="text-sm opacity-75">
                            {t!(i18n, profile.about.optional_hint)}" "
                            <a href="/privacy" class="underline">
                                {t!(i18n, profile.about.privacy)}
                            </a>
                        </p>
                        <fieldset disabled=move || action.pending().get() class="space-y-3">
                            <label class="flex flex-col gap-1 max-w-sm" for="profile-flag">
                                <span class="ui-field-label">{t!(i18n, profile.about.flag)}</span>
                                <select
                                    id="profile-flag"
                                    name="flag"
                                    class="ui-field-input"
                                    prop:value=flag
                                    on:change=move |ev| flag.set(event_target_value(&ev))
                                >
                                    <option value="">{t!(i18n, profile.about.unset)}</option>
                                    {catalog()
                                        .countries
                                        .iter()
                                        .map(|(code, country)| {
                                            view! {
                                                <option
                                                    value=code.clone()
                                                    selected=flag.get_untracked() == *code
                                                >
                                                    {format!("{} {}", flag_emoji(code), country)}
                                                </option>
                                            }
                                        })
                                        .collect_view()}
                                </select>
                            </label>
                            <p class="text-xs opacity-75">{t!(i18n, profile.about.flag_help)}</p>
                            <label class="flex flex-col gap-1" for="profile-bio">
                                <span class="ui-field-label">{t!(i18n, profile.about.bio)}</span>
                                <textarea
                                    id="profile-bio"
                                    name="bio"
                                    class="ui-field-input"
                                    rows="3"
                                    readonly=bio_blocked
                                    prop:value=bio
                                    on:input=move |ev| bio.set(event_target_value(&ev))
                                    aria-describedby="profile-bio-count profile-links-help"
                                />
                            </label>
                            <p id="profile-links-help" class="text-xs opacity-75">
                                {t!(i18n, profile.about.links_hint)}
                            </p>
                            <p id="profile-bio-count" class="text-xs text-right" aria-live="polite">
                                {move || format!("{}/300", bio.with(|text| text.chars().count()))}
                            </p>
                            {move || {
                                action
                                    .value()
                                    .get()
                                    .and_then(Result::err)
                                    .map(|error| {
                                        view! {
                                            <p role="alert" class="text-red-600 dark:text-red-400">
                                                {error.to_string()}
                                            </p>
                                        }
                                    })
                            }}
                            <div class="flex gap-2">
                                <button
                                    type="submit"
                                    class="ui-button ui-button-primary"
                                    disabled=move || {
                                        action.pending().get()
                                            || bio.with(|text| text.chars().count() > 300)
                                    }
                                >
                                    {t!(i18n, profile.about.save)}
                                </button>
                                <button
                                    type="button"
                                    class="ui-button"
                                    on:click=move |_| {
                                        reset();
                                        editing.set(false);
                                    }
                                >
                                    {t!(i18n, profile.about.cancel)}
                                </button>
                            </div>
                        </fieldset>
                    </ActionForm>
                }
            }
        >
            <div
                class="whitespace-pre-wrap break-words [&_a]:underline"
                inner_html=about_html.clone()
            />
            <Show when=move || owner.get()>
                <button
                    class="mt-2 underline"
                    on:click=move |_| {
                        reset();
                        editing.set(true);
                    }
                >
                    {t!(i18n, profile.about.edit)}
                </button>
            </Show>
        </Show>
        <Show when=move || admin.get()>
            <Show
                when=move || !moderating.get()
                fallback=move || {
                    view! {
                        <BioModerationPanel
                            user_id=user_id
                            on_saved=on_saved
                            on_close=Callback::new(move |()| moderating.set(false))
                        />
                    }
                }
            >
                <button
                    type="button"
                    class="mt-2 ml-3 underline"
                    on:click=move |_| {
                        editing.set(false);
                        moderating.set(true);
                    }
                >
                    {t!(i18n, profile.about.moderate_bio)}
                </button>
            </Show>
        </Show>
    }
}

#[component]
fn BioModerationPanel(
    user_id: Uuid,
    on_saved: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let details = LocalResource::new(move || get_bio_moderation(user_id));
    view! {
        <div class="pt-3 mt-3 border-t border-gray-300 dark:border-gray-600">
            <p class="mb-2 font-semibold">{t!(i18n, profile.about.moderate_bio)}</p>
            {move || {
                details
                    .get()
                    .map(|result| match result {
                        Ok(data) => {
                            view! {
                                <BioModerationForm
                                    user_id=user_id
                                    data=data
                                    on_saved=on_saved
                                    on_close=on_close
                                />
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! {
                                <p role="alert">{t!(i18n, profile.about.load_error)}</p>
                                <button
                                    type="button"
                                    class="mr-3 underline"
                                    on:click=move |_| details.refetch()
                                >
                                    {t!(i18n, profile.about.retry)}
                                </button>
                                <button
                                    type="button"
                                    class="underline"
                                    on:click=move |_| on_close.run(())
                                >
                                    {t!(i18n, profile.about.cancel)}
                                </button>
                            }
                                .into_any()
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn BioModerationForm(
    user_id: Uuid,
    data: BioModerationDetails,
    on_saved: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let action = ServerAction::<SaveBioModeration>::new();
    let bio = RwSignal::new(data.bio);
    let blocked = RwSignal::new(data.blocked);
    Effect::watch(
        action.version(),
        move |_, _, _| {
            if action
                .value()
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                on_close.run(());
                on_saved.run(());
            }
        },
        false,
    );
    view! {
        <ActionForm action=action attr:class="space-y-3">
            <input type="hidden" name="user_id" value=user_id.to_string() />
            <input type="hidden" name="blocked" value=move || blocked.get().to_string() />
            <fieldset disabled=move || action.pending().get() class="space-y-3">
                <label for="moderated-bio" class="flex flex-col gap-1">
                    <span class="ui-field-label">{t!(i18n, profile.about.bio)}</span>
                    <textarea
                        id="moderated-bio"
                        name="bio"
                        class="ui-field-input"
                        rows="3"
                        prop:value=bio
                        on:input=move |ev| bio.set(event_target_value(&ev))
                        aria-describedby="moderated-bio-count"
                    />
                </label>
                <p id="moderated-bio-count" class="text-xs text-right">
                    {move || format!("{}/300", bio.with(|text| text.chars().count()))}
                </p>
                <label class="flex gap-2 items-center">
                    <input
                        type="checkbox"
                        prop:checked=blocked
                        on:change=move |ev| blocked.set(event_target_checked(&ev))
                    />
                    {t!(i18n, profile.about.block_bio)}
                </label>
                <p class="text-xs opacity-75">{t!(i18n, profile.about.block_bio_help)}</p>
                {move || {
                    action
                        .value()
                        .get()
                        .and_then(Result::err)
                        .map(|error| {
                            view! {
                                <p role="alert" class="text-red-600 dark:text-red-400">
                                    {error.to_string()}
                                </p>
                            }
                        })
                }}
                <div class="flex gap-2">
                    <button
                        type="submit"
                        class="ui-button ui-button-primary"
                        disabled=move || {
                            action.pending().get() || bio.with(|text| text.chars().count() > 300)
                        }
                    >
                        {t!(i18n, profile.about.save_moderation)}
                    </button>
                    <button type="button" class="ui-button" on:click=move |_| on_close.run(())>
                        {t!(i18n, profile.about.cancel)}
                    </button>
                </div>
            </fieldset>
        </ActionForm>
    }
}
