use crate::{
    common::{localized_error_message, LinkedText},
    functions::users::player_profile::{get_player_details, ModerateProfileBio, SaveProfileBio},
    i18n::*,
    providers::AuthContext,
    security::csrf::ActionForm,
};
use leptos::prelude::*;
use leptos_icons::Icon;
use uuid::Uuid;

#[component]
pub fn ProfileAbout(user_id: Uuid) -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>().user;
    let owner =
        Signal::derive(move || auth.with(|a| a.as_ref().is_some_and(|a| a.user.uid == user_id)));
    let admin = Signal::derive(move || auth.with(|a| a.as_ref().is_some_and(|a| a.user.admin)));
    let details = LocalResource::new(move || get_player_details(user_id));
    view! {
        <div class="my-2 mx-1 w-full max-w-4xl text-sm dark:text-white">
            {move || {
                details
                    .get()
                    .map(|result| match result {
                        Ok(data) => {
                            let has_bio = !data.bio.is_empty();
                            view! {
                                <Show when=move || owner.get() || admin.get() || has_bio>
                                    <details
                                        open
                                        class="relative rounded border border-gray-300 dark:border-gray-600 group/profile-about"
                                    >
                                        <summary
                                            aria-label=move || t_string!(i18n, profile.about.title)
                                            class="block py-2 px-3 list-none cursor-pointer [&::-webkit-details-marker]:hidden group-open/profile-about:absolute group-open/profile-about:top-0 group-open/profile-about:right-0"
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
                                            <BioPanel
                                                bio=data.bio.clone()
                                                restricted=data.bio_restricted
                                                owner
                                                admin
                                                user_id
                                                on_saved=Callback::new(move |()| details.refetch())
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
                                    <button class="underline" on:click=move |_| details.refetch()>
                                        {t!(i18n, common.retry)}
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
fn BioPanel(
    bio: String,
    restricted: bool,
    owner: Signal<bool>,
    admin: Signal<bool>,
    user_id: Uuid,
    on_saved: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let editing = RwSignal::new(false);
    let moderating = RwSignal::new(false);
    let text = StoredValue::new(bio.clone());
    view! {
        <LinkedText body=bio />
        <Show when=move || owner.get() && restricted>
            <p class="mt-2">{t!(i18n, profile.about.restricted)}</p>
        </Show>
        <Show when=move || owner.get() && !restricted>
            <Show
                when=move || editing.get()
                fallback=move || {
                    view! {
                        <button
                            type="button"
                            class="flex mt-2 ui-button ui-button-secondary ui-button-icon"
                            title=move || t_string!(i18n, profile.about.edit)
                            aria-label=move || t_string!(i18n, profile.about.edit)
                            on:click=move |_| editing.set(true)
                        >
                            <Icon icon=icondata_bs::BsPencil attr:class="size-5" />
                        </button>
                    }
                }
            >
                <BioEditor
                    bio=text.get_value()
                    on_saved
                    on_close=Callback::new(move |()| editing.set(false))
                />
            </Show>
        </Show>
        <Show when=move || admin.get()>
            <Show
                when=move || moderating.get()
                fallback=move || {
                    view! {
                        <button class="mt-2 ml-3 underline" on:click=move |_| moderating.set(true)>
                            {t!(i18n, profile.about.moderate)}
                        </button>
                    }
                }
            >
                <BioModeration
                    bio=text.get_value()
                    restricted
                    user_id
                    on_saved
                    on_close=Callback::new(move |()| moderating.set(false))
                />
            </Show>
        </Show>
    }
}

#[component]
fn BioEditor(bio: String, on_saved: Callback<()>, on_close: Callback<()>) -> impl IntoView {
    let i18n = use_i18n();
    let action = ServerAction::<SaveProfileBio>::new();
    let text = RwSignal::new(bio);
    Effect::watch(
        action.version(),
        move |_, _, _| {
            if action.value().get_untracked().is_some_and(|r| r.is_ok()) {
                on_close.run(());
                on_saved.run(());
            }
        },
        false,
    );
    view! {
        <ActionForm action attr:class="mt-2 space-y-2">
            <label class="block">
                {t!(i18n, profile.about.title)}
                <textarea
                    name="bio"
                    rows="3"
                    class="w-full ui-field-textarea"
                    prop:value=text
                    on:input=move |e| text.set(event_target_value(&e))
                    aria-describedby="bio-help bio-count"
                />
            </label>
            <p id="bio-help">{t!(i18n, profile.about.links_hint)}</p>
            <p id="bio-count">{move || format!("{}/300", text.with(|s| s.chars().count()))}</p>
            <Show when=move || text.with(|s| s.chars().count() > 300)>
                <p role="alert">{t!(i18n, profile.about.too_long)}</p>
            </Show>
            {move || {
                action
                    .value()
                    .get()
                    .and_then(Result::err)
                    .map(|e| {
                        view! {
                            <p role="alert">{localized_error_message(i18n, &e.to_string())}</p>
                        }
                    })
            }}
            <button
                type="submit"
                class="ui-button ui-button-primary ui-button-sm"
                disabled=move || action.pending().get() || text.with(|s| s.chars().count() > 300)
            >
                {t!(i18n, common.save)}
            </button>
            <button
                type="button"
                class="ui-button ui-button-ghost ui-button-sm"
                on:click=move |_| on_close.run(())
            >
                {t!(i18n, common.cancel)}
            </button>
        </ActionForm>
    }
}

#[component]
fn BioModeration(
    bio: String,
    restricted: bool,
    user_id: Uuid,
    on_saved: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let i18n = use_i18n();
    let action = ServerAction::<ModerateProfileBio>::new();
    let text = RwSignal::new(bio);
    Effect::watch(
        action.version(),
        move |_, _, _| {
            if action.value().get_untracked().is_some_and(|r| r.is_ok()) {
                on_close.run(());
                on_saved.run(());
            }
        },
        false,
    );
    view! {
        <div class="pt-3 mt-3 space-y-2 border-t">
            <p>{t!(i18n, profile.about.moderation_help)}</p>
            <label class="block">
                {t!(i18n, profile.about.replacement)}
                <textarea
                    rows="3"
                    class="w-full ui-field-textarea"
                    prop:value=text
                    on:input=move |e| text.set(event_target_value(&e))
                />
            </label>
            <p>{move || format!("{}/300", text.with(|s| s.chars().count()))}</p>
            {move || {
                action
                    .value()
                    .get()
                    .and_then(Result::err)
                    .map(|e| {
                        view! {
                            <p role="alert">{localized_error_message(i18n, &e.to_string())}</p>
                        }
                    })
            }}
            <fieldset disabled=move || action.pending().get() class="flex flex-wrap gap-2">
                <button
                    class="ui-button ui-button-secondary ui-button-sm"
                    disabled=move || text.with(|s| s.chars().count() > 300)
                    on:click=move |_| {
                        action
                            .dispatch(ModerateProfileBio {
                                user_id,
                                operation: "replace".into(),
                                bio: text.get_untracked(),
                            });
                    }
                >
                    {t!(i18n, common.save)}
                </button>
                <button
                    class="ui-button ui-button-secondary ui-button-sm"
                    on:click=move |_| {
                        action
                            .dispatch(ModerateProfileBio {
                                user_id,
                                operation: "clear".into(),
                                bio: String::new(),
                            });
                    }
                >
                    {t!(i18n, profile.about.clear)}
                </button>
                <Show
                    when=move || !restricted
                    fallback=move || {
                        view! {
                            <button
                                class="ui-button ui-button-secondary ui-button-sm"
                                on:click=move |_| {
                                    action
                                        .dispatch(ModerateProfileBio {
                                            user_id,
                                            operation: "reinstate".into(),
                                            bio: String::new(),
                                        });
                                }
                            >
                                {t!(i18n, profile.about.reinstate)}
                            </button>
                        }
                    }
                >
                    <button
                        class="ui-button ui-button-danger ui-button-sm"
                        on:click=move |_| {
                            action
                                .dispatch(ModerateProfileBio {
                                    user_id,
                                    operation: "restrict".into(),
                                    bio: String::new(),
                                });
                        }
                    >
                        {t!(i18n, profile.about.restrict)}
                    </button>
                </Show>
                <button
                    class="ui-button ui-button-ghost ui-button-sm"
                    on:click=move |_| on_close.run(())
                >
                    {t!(i18n, common.cancel)}
                </button>
            </fieldset>
        </div>
    }
}
