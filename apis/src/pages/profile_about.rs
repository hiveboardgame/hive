use crate::{
    functions::users::player_profile::{ProfileDetails, SavePlayerDetails},
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
                                <Show when=move || owner() || has_details>
                                    <details
                                        open
                                        class="rounded-xl border border-gray-300 dark:border-gray-600 group/profile-about"
                                    >
                                        <summary
                                            class="py-2 px-3 font-semibold cursor-pointer"
                                            aria-label=move || t_string!(i18n, profile.about.title)
                                        >
                                            <span class="group-open/profile-about:hidden">
                                                {t!(i18n, profile.about.title)}
                                            </span>
                                        </summary>
                                        <div class="overflow-y-auto px-3 pb-3 max-h-[50vh]">
                                            <Show when=move || saved.get()>
                                                <p role="status" class="mb-2">
                                                    {t!(i18n, profile.about.saved)}
                                                </p>
                                            </Show>
                                            <ProfileDetailsPanel
                                                data=data.clone()
                                                owner=Signal::derive(owner)
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
    on_saved: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let editing = RwSignal::new(false);
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
    }
}
