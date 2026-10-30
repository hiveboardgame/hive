use crate::{
    functions::users::player_profile::{ProfileDetails, SavePlayerDetails},
    i18n::*,
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
                            let has_details = data.location.is_some() || data.age.is_some()
                                || data.gender.is_some() || !data.bio.is_empty()
                                || data.member_since.is_some();
                            view! {
                                <Show when=move || owner() || has_details>
                                    <details
                                        open
                                        class="rounded-xl border border-gray-300 dark:border-gray-600"
                                    >
                                        <summary class="py-2 px-3 font-semibold cursor-pointer">
                                            {t!(i18n, profile.about.title)}
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
    let location = RwSignal::new(data.location.clone().unwrap_or_default());
    let year = RwSignal::new(
        data.owner_birth_year
            .map(|year| year.to_string())
            .unwrap_or_default(),
    );
    let gender = RwSignal::new(data.gender.clone().unwrap_or_default());
    let bio = RwSignal::new(data.bio.clone());
    let original = StoredValue::new(data.clone());
    let reset = move || {
        let data = original.get_value();
        flag.set(data.flag.unwrap_or_default());
        location.set(data.location.unwrap_or_default());
        year.set(
            data.owner_birth_year
                .map(|year| year.to_string())
                .unwrap_or_default(),
        );
        gender.set(data.gender.unwrap_or_default());
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
    let mut facts = Vec::new();
    if let Some(location) = &data.location {
        facts.push(
            location
                .rsplit_once(", ")
                .map(|(city, _)| city)
                .unwrap_or(location)
                .to_string(),
        );
    }
    if let Some(age) = data.age {
        facts.push(age.to_string());
    }
    if let Some(gender) = &data.gender {
        facts.push(gender.clone());
    }
    let facts = facts.join(", ");
    let biography = data.bio.clone();
    let member_since = data.member_since;
    view! {
        <Show
            when=move || !editing.get()
            fallback=move || {
                view! {
                    <ActionForm action=action attr:class="space-y-3">
                        <p class="text-sm opacity-75">{t!(i18n, profile.about.optional_hint)}</p>
                        <fieldset disabled=move || action.pending().get() class="space-y-3">
                            <div class="grid gap-3 sm:grid-cols-2">
                                <label class="flex flex-col gap-1" for="profile-flag">
                                    <span class="ui-field-label">
                                        {t!(i18n, profile.about.flag)}
                                    </span>
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
                                <label class="flex flex-col gap-1" for="profile-location">
                                    <span class="ui-field-label">
                                        {t!(i18n, profile.about.location)}
                                    </span>
                                    <input
                                        id="profile-location"
                                        name="location"
                                        class="ui-field-input"
                                        list="profile-locations"
                                        autocomplete="off"
                                        maxlength="150"
                                        prop:value=location
                                        on:input=move |ev| location.set(event_target_value(&ev))
                                        placeholder=move || {
                                            t_string!(i18n, profile.about.location_placeholder)
                                        }
                                    />
                                    <datalist id="profile-locations">
                                        {move || {
                                            let query = location.get().to_lowercase();
                                            catalog()
                                                .locations
                                                .iter()
                                                .filter(|city| {
                                                    query.len() >= 2 && city.to_lowercase().contains(&query)
                                                })
                                                .take(30)
                                                .map(|city| view! { <option value=city.clone() /> })
                                                .collect_view()
                                        }}
                                    </datalist>
                                </label>
                                <label class="flex flex-col gap-1" for="profile-year">
                                    <span class="ui-field-label">
                                        {t!(i18n, profile.about.birth_year)}
                                    </span>
                                    <input
                                        id="profile-year"
                                        name="birth_year"
                                        class="ui-field-input"
                                        type="number"
                                        min="1900"
                                        step="1"
                                        prop:value=year
                                        on:input=move |ev| year.set(event_target_value(&ev))
                                        aria-describedby="profile-age-help"
                                    />
                                </label>
                                <label class="flex flex-col gap-1" for="profile-gender">
                                    <span class="ui-field-label">
                                        {t!(i18n, profile.about.gender)}
                                    </span>
                                    <select
                                        id="profile-gender"
                                        name="gender"
                                        class="ui-field-input"
                                        prop:value=gender
                                        on:change=move |ev| gender.set(event_target_value(&ev))
                                    >
                                        <option value="">{t!(i18n, profile.about.unset)}</option>
                                        <option value="M" selected=gender.get_untracked() == "M">
                                            {t!(i18n, profile.about.male)}
                                        </option>
                                        <option value="F" selected=gender.get_untracked() == "F">
                                            {t!(i18n, profile.about.female)}
                                        </option>
                                        <option value="X" selected=gender.get_untracked() == "X">
                                            {t!(i18n, profile.about.other)}
                                        </option>
                                        <option value="?" selected=gender.get_untracked() == "?">
                                            {t!(i18n, profile.about.unspecified)}
                                        </option>
                                    </select>
                                </label>
                            </div>
                            <p class="text-xs opacity-75">
                                {t!(i18n, profile.about.city_data)}" "
                                <a
                                    href="https://www.geonames.org/"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    class="underline"
                                >
                                    "GeoNames"
                                </a> " (CC BY 4.0)"
                            </p>
                            <p id="profile-age-help" class="text-xs opacity-75">
                                {t!(i18n, profile.about.age_help)}
                            </p>
                            <label class="flex flex-col gap-1" for="profile-bio">
                                <span class="ui-field-label">{t!(i18n, profile.about.bio)}</span>
                                <textarea
                                    id="profile-bio"
                                    name="bio"
                                    class="ui-field-input"
                                    rows="3"
                                    prop:value=bio
                                    on:input=move |ev| bio.set(event_target_value(&ev))
                                    aria-describedby="profile-bio-count"
                                />
                            </label>
                            <p class="text-xs opacity-75">{t!(i18n, profile.about.no_links)}</p>
                            <Show when=move || crate::profile_bio::bio_contains_link(&bio.get())>
                                <p role="alert" class="text-red-600 dark:text-red-400">
                                    {t!(i18n, profile.about.no_links)}
                                </p>
                            </Show>
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
                                            || bio
                                                .with(|text| {
                                                    text.chars().count() > 300
                                                        || crate::profile_bio::bio_contains_link(text)
                                                })
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
            <p class="whitespace-pre-wrap break-words">{biography.clone()}</p>
            <p class="mt-2">{facts.clone()}</p>
            {member_since
                .map(|date| {
                    view! {
                        <p class="mt-3 text-xs opacity-75">
                            {t!(i18n, profile.about.member_since)}" "
                            <time datetime=date
                                .to_string()>{date.format("%-d %B %Y").to_string()}</time>
                        </p>
                    }
                })}
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
