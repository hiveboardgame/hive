use crate::{
    common::with_class,
    components::{atoms::rating::icon_for_speed, molecules::dropdown_panel::DropdownPanel},
    i18n::*,
    pages::profile_view::tab_from_path,
    providers::{
        initial_profile_filters_for_tab,
        load_games,
        searchable_profile_filters_for_tab,
        FilterState,
        GamesSearchContext,
        ResultType,
    },
};
use hive_lib::Color;
use leptos::{either::Either, html, prelude::*};
use leptos_i18n::I18nContext;
use leptos_icons::*;
use leptos_router::hooks::use_location;
use leptos_use::{on_click_outside_with_options, OnClickOutsideOptions};
use shared_types::{GameProgress, GameSpeed};

#[derive(Clone, Copy)]
pub enum TriStateType {
    Color,
    Expansion,
    Rating,
}

impl TriStateType {
    fn get_value(&self, state: &FilterState) -> Option<bool> {
        match self {
            TriStateType::Color => state.color.map(|c| matches!(c, Color::White)),
            TriStateType::Expansion => state.expansions,
            TriStateType::Rating => state.rated,
        }
    }

    fn update(&self, state: &mut FilterState, value: Option<bool>) {
        match self {
            TriStateType::Color => {
                state.color = value.map(|v| if v { Color::White } else { Color::Black });
            }
            TriStateType::Expansion => {
                state.expansions = value;
                if value == Some(false) && state.rated == Some(true) {
                    state.rated = Some(false);
                }
            }
            TriStateType::Rating => {
                state.rated = value;
                if value == Some(true) {
                    if state.expansions == Some(false) {
                        state.expansions = Some(true);
                    }
                    state.speeds.retain(|s| *s != GameSpeed::Untimed);
                }
            }
        }
    }
}

#[component]
pub fn GamesFilter(username: String, ctx: GamesSearchContext) -> impl IntoView {
    let username = StoredValue::new(username);
    let location = use_location();
    let current_tab = Signal::derive(move || tab_from_path(&location.pathname.get()));
    let pending = ctx.pending;
    let i18n = use_i18n();

    let dropdown_ref = NodeRef::<html::Details>::new();

    let close_dropdown = move || {
        if let Some(details) = dropdown_ref.get() {
            let _ = details.remove_attribute("open");
        }
    };

    Effect::new(move |_| {
        let _ = on_click_outside_with_options(
            dropdown_ref,
            move |_| close_dropdown(),
            OnClickOutsideOptions::default(),
        );
    });

    let perform_search = move |new_filters: FilterState| {
        let effective_filters =
            searchable_profile_filters_for_tab(new_filters, current_tab.get_untracked());
        ctx.filters.set(effective_filters.clone());
        ctx.pending.set(effective_filters.clone());
        ctx.has_more.set_value(true);
        ctx.is_first_batch.set_value(true);
        ctx.next_batch_token.set(None);
        let batch_size = ctx.initial_batch_size.get();
        load_games(
            effective_filters,
            current_tab.get_untracked(),
            username.get_value(),
            None,
            ctx.next_batch,
            batch_size,
        );
    };

    let apply_filters = move |_| {
        perform_search(pending.get_untracked());
        close_dropdown();
    };

    let reset_filters = move |_| {
        let reset_to = initial_profile_filters_for_tab(
            current_tab.get_untracked(),
            ctx.get_filter_cookie.get_untracked(),
        );
        pending.set(reset_to.clone());

        if ctx.filters.get_untracked() != reset_to {
            perform_search(reset_to);
        }
        close_dropdown();
    };

    let set_default_filters = move |_| {
        let current_pending = pending.get_untracked();
        ctx.set_filter_cookie.set(Some(current_pending.clone()));

        if ctx.filters.get_untracked() != current_pending {
            perform_search(current_pending);
        }
        close_dropdown();
    };

    let clear_default_filters = move |_| {
        ctx.set_filter_cookie.set(None);
        let system_defaults = FilterState::default();
        pending.set(system_defaults.clone());

        if ctx.filters.get_untracked() != system_defaults {
            perform_search(system_defaults);
        }
        close_dropdown();
    };

    let toggle_speeds = move |speed: &GameSpeed| {
        pending.update(|state| {
            if state.speeds.contains(speed) {
                state.speeds.retain(|s| s != speed);
            } else {
                state.speeds.push(*speed);
            }

            if state.rated == Some(true) && state.speeds.contains(&GameSpeed::Untimed) {
                if state.speeds.len() == 1 {
                    state.rated = Some(false);
                } else {
                    state.rated = None;
                }
            }
        });
    };

    let no_changes = Signal::derive(move || ctx.filters.get() == pending.get());
    let no_speeds = Signal::derive(move || pending.with(|state| state.speeds.is_empty()));
    let saved_default_exists =
        Signal::derive(move || ctx.get_filter_cookie.with(|cookie| cookie.is_some()));
    let is_current_saved_default = Signal::derive(move || {
        let Some(saved_default) = ctx.get_filter_cookie.get() else {
            return false;
        };
        if no_changes() {
            saved_default == ctx.filters.get()
        } else {
            saved_default == pending.get()
        }
    });
    let is_system_default = Signal::derive(move || ctx.filters.get() == FilterState::default());
    let reset_disabled = Signal::derive(move || {
        let current_filters = ctx.filters.get();
        let reset_target =
            initial_profile_filters_for_tab(current_tab(), ctx.get_filter_cookie.get());
        current_filters == reset_target
    });

    view! {
        <div class="relative">
            <details node_ref=dropdown_ref>
                <summary class="py-1 px-3 ml-0.5 text-sm cursor-pointer ui-button ui-button-secondary ui-button-md max-w-fit [&::-webkit-details-marker]:hidden">
                    <Icon icon=icondata_lu::LuSlidersHorizontal attr:class="size-4" />
                    <span>"Filters"</span>
                </summary>
                <DropdownPanel class="absolute right-0 top-full z-40 p-2 mt-1 max-w-screen-sm lg:p-0 lg:mt-2 lg:max-w-none w-[90vw] lg:w-[31rem]">
                    <ActiveFiltersDisplay ctx current_tab i18n />

                    <div class="space-y-2 lg:flex lg:space-y-0">
                        <div class=with_class(
                            "lg:border-r lg:border-black/10 lg:dark:border-white/10",
                            "space-y-2 lg:flex-1 lg:p-4",
                        )>
                            <TriStateFilter
                                filter_type=TriStateType::Color
                                pending
                                grid_class="lg:grid-cols-3 lg:gap-2"
                            />

                            <Show when=move || current_tab() == GameProgress::Finished>
                                <div class="space-y-1 lg:space-y-2">
                                    <label class="ui-field-label">
                                        {t!(i18n, profile.game_result)}
                                    </label>
                                    <div class="grid grid-cols-2 gap-1 lg:gap-2">
                                        <FilterButton
                                            is_active=Signal::derive(move || {
                                                pending.with(|p| p.result == Some(ResultType::Win))
                                            })
                                            on_click=move |_| {
                                                pending.update(|state| state.result = Some(ResultType::Win))
                                            }
                                        >
                                            {t!(i18n, profile.result_buttons.win)}
                                        </FilterButton>
                                        <FilterButton
                                            is_active=Signal::derive(move || {
                                                pending.with(|p| p.result == Some(ResultType::Loss))
                                            })
                                            on_click=move |_| {
                                                pending
                                                    .update(|state| state.result = Some(ResultType::Loss))
                                            }
                                        >
                                            {t!(i18n, profile.result_buttons.loss)}
                                        </FilterButton>
                                        <FilterButton
                                            is_active=Signal::derive(move || {
                                                pending.with(|p| p.result == Some(ResultType::Draw))
                                            })
                                            on_click=move |_| {
                                                pending
                                                    .update(|state| state.result = Some(ResultType::Draw))
                                            }
                                        >
                                            {t!(i18n, profile.result_buttons.draw)}
                                        </FilterButton>
                                        <FilterButton
                                            is_active=Signal::derive(move || {
                                                pending.with(|p| p.result.is_none())
                                            })
                                            on_click=move |_| {
                                                pending.update(|state| state.result = None)
                                            }
                                        >
                                            "Any"
                                        </FilterButton>
                                    </div>
                                </div>
                            </Show>

                            <div class="space-y-1 lg:space-y-2">
                                <label class="ui-field-label">
                                    {t!(i18n, profile.include_speeds)}
                                </label>
                                <div class="grid grid-cols-3 gap-1 lg:gap-2">
                                    {GameSpeed::all_games()
                                        .into_iter()
                                        .map(|speed| {
                                            view! {
                                                <FilterButton
                                                    is_active=Signal::derive(move || {
                                                        pending.with(|state| state.speeds.contains(&speed))
                                                    })
                                                    on_click=move |_| toggle_speeds(&speed)
                                                    fit_label=false
                                                    flex_class=""
                                                >
                                                    <Icon
                                                        icon=icon_for_speed(speed)
                                                        attr:class="size-4 xs:size-5 lg:size-5"
                                                    />
                                                </FilterButton>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>
                        </div>

                        <div class="space-y-2 lg:flex-1 lg:p-4">
                            <div class="grid grid-cols-2 gap-2 lg:block lg:space-y-4">
                                <TriStateFilter
                                    filter_type=TriStateType::Expansion
                                    pending
                                    grid_class="lg:grid-cols-3 lg:gap-2"
                                />

                                <TriStateFilter
                                    filter_type=TriStateType::Rating
                                    pending
                                    grid_class="lg:grid-cols-3 lg:gap-2"
                                />
                            </div>

                            <div class="flex justify-between items-center lg:pt-2">
                                <span class="ui-field-label">"Bot Games:"</span>
                                <FilterButton
                                    is_active=Signal::derive(move || {
                                        !pending.with(|state| state.exclude_bots)
                                    })
                                    on_click=move |_| {
                                        pending
                                            .update(|state| state.exclude_bots = !state.exclude_bots)
                                    }
                                    flex_class="w-24 shrink-0"
                                >
                                    {move || {
                                        if pending.with(|state| state.exclude_bots) {
                                            "Exclude"
                                        } else {
                                            "Include"
                                        }
                                    }}
                                </FilterButton>
                            </div>
                        </div>
                    </div>

                    <Show when=no_speeds>
                        <div class=with_class(
                            "ui-warning-notice",
                            "mx-0 mb-0 lg:mx-4 lg:mb-4",
                        )>"Please select at least one game speed"</div>
                    </Show>

                    <div class=with_class(
                        "border-t border-black/10 bg-odd-light/70 dark:border-white/10 dark:bg-surface-muted",
                        "mt-3 flex gap-1 pt-2 xs:gap-2 lg:justify-end lg:gap-3 lg:p-4",
                    )>
                        <ActionButton
                            on_click=apply_filters
                            disabled=Signal::derive(move || no_changes() || no_speeds())
                            variant=FilterActionKind::Apply
                        />
                        <ActionButton
                            on_click=reset_filters
                            disabled=reset_disabled
                            variant=FilterActionKind::Reset
                        />
                        <Show when=move || current_tab() == GameProgress::Finished>
                            <ActionButton
                                on_click=set_default_filters
                                disabled=Signal::derive(move || {
                                    is_current_saved_default() || no_speeds()
                                })
                                variant=FilterActionKind::Save
                            />
                            <ActionButton
                                on_click=clear_default_filters
                                disabled=Signal::derive(move || {
                                    !saved_default_exists() && is_system_default()
                                })
                                variant=FilterActionKind::Clear
                            />
                        </Show>
                    </div>
                </DropdownPanel>
            </details>
        </div>
    }
}

#[component]
fn ActiveFiltersDisplay(
    ctx: GamesSearchContext,
    current_tab: Signal<GameProgress>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    view! {
        <div class="pb-2 mb-2 ui-divider-bottom">
            <div class="flex flex-wrap gap-1 items-center p-2 min-h-[24px]">
                <span class="mr-1 text-xs font-medium text-gray-600 dark:text-gray-400">
                    "Active:"
                </span>

                <Show when=move || ctx.filters.with(|state| state.color.is_some())>
                    <FilterPill>
                        {move || match ctx.filters.with(|state| state.color) {
                            Some(Color::Black) => t_string!(i18n, profile.color_buttons.black),
                            Some(Color::White) => t_string!(i18n, profile.color_buttons.white),
                            None => "",
                        }}
                    </FilterPill>
                </Show>

                <Show when=move || {
                    current_tab() == GameProgress::Finished
                        && ctx.filters.with(|state| state.result.is_some())
                }>
                    <FilterPill>
                        {move || match ctx.filters.get().result {
                            Some(ResultType::Win) => t_string!(i18n, profile.result_buttons.win),
                            Some(ResultType::Loss) => t_string!(i18n, profile.result_buttons.loss),
                            Some(ResultType::Draw) => t_string!(i18n, profile.result_buttons.draw),
                            None => "",
                        }}
                    </FilterPill>
                </Show>

                <FilterPill>
                    <For each=move || ctx.filters.get().speeds key=|speed| *speed let:speed>
                        <Icon icon=icon_for_speed(speed) attr:class="size-3" />
                    </For>
                </FilterPill>

                <Show when=move || ctx.filters.with(|state| state.expansions.is_some())>
                    <FilterPill>
                        {move || match ctx.filters.get().expansions {
                            Some(true) => "Full",
                            Some(false) => "Basic",
                            None => "",
                        }}
                    </FilterPill>
                </Show>

                <Show when=move || ctx.filters.with(|state| state.rated.is_some())>
                    <FilterPill>
                        {move || match ctx.filters.get().rated {
                            Some(true) => "Rated",
                            Some(false) => "Unrated",
                            None => "",
                        }}
                    </FilterPill>
                </Show>

                <Show when=move || !ctx.filters.with(|state| state.exclude_bots)>
                    <FilterPill>
                        <Icon icon=icondata_mdi::MdiRobotHappy attr:class="size-3" />
                        <span>"Bots"</span>
                    </FilterPill>
                </Show>
            </div>
        </div>
    }
}

#[component]
fn FilterPill(children: Children) -> impl IntoView {
    view! {
        <div class="flex gap-0.5 items-center py-0.5 px-2 text-xs rounded-full border bg-pillbug-teal/10 text-pillbug-teal border-pillbug-teal/20">
            {children()}
        </div>
    }
}

#[component]
fn FilterButton<F>(
    is_active: Signal<bool>,
    on_click: F,
    children: Children,
    #[prop(default = true)] fit_label: bool,
    #[prop(optional)] flex_class: &'static str,
) -> impl IntoView
where
    F: Fn(leptos::ev::MouseEvent) + 'static,
{
    let content = if fit_label {
        Either::Left(view! { <span class="ui-fit-label">{children()}</span> })
    } else {
        Either::Right(children())
    };

    view! {
        <button
            class=move || {
                let state = if is_active() { "ui-choice-active" } else { "ui-choice-inactive" };
                let size = if fit_label {
                    "ui-choice-xs ui-fit-container"
                } else {
                    "ui-choice-icon"
                };
                with_class(&format!("ui-choice {state} {size} cursor-pointer"), flex_class)
            }
            on:click=on_click
        >
            {content}
        </button>
    }
}

#[component]
fn TriStateFilter(
    filter_type: TriStateType,
    pending: RwSignal<FilterState>,
    #[prop(optional)] grid_class: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();

    let is_option1_active =
        Signal::derive(move || filter_type.get_value(&pending.get()) == Some(false));
    let is_option2_active =
        Signal::derive(move || filter_type.get_value(&pending.get()) == Some(true));
    let is_any_active = Signal::derive(move || filter_type.get_value(&pending.get()).is_none());

    let set_option1 = move |_| {
        pending.update(|state| filter_type.update(state, Some(false)));
    };
    let set_option2 = move |_| {
        pending.update(|state| filter_type.update(state, Some(true)));
    };
    let set_any = move |_| {
        pending.update(|state| filter_type.update(state, None));
    };

    view! {
        <div class="space-y-1 lg:space-y-2">
            <label class="ui-field-label">
                {move || match filter_type {
                    TriStateType::Color => "Player Color",
                    TriStateType::Expansion => "Expansions",
                    TriStateType::Rating => "Rating",
                }}
            </label>
            <div class=format!("grid grid-cols-3 gap-1 {}", grid_class)>
                <FilterButton is_active=is_option1_active on_click=set_option1 flex_class="">
                    {move || match filter_type {
                        TriStateType::Color => t_string!(i18n, profile.color_buttons.black),
                        TriStateType::Expansion => "Basic",
                        TriStateType::Rating => "Casual",
                    }}
                </FilterButton>
                <FilterButton is_active=is_option2_active on_click=set_option2 flex_class="">
                    {move || match filter_type {
                        TriStateType::Color => t_string!(i18n, profile.color_buttons.white),
                        TriStateType::Expansion => "Full",
                        TriStateType::Rating => "Rated",
                    }}
                </FilterButton>
                <FilterButton is_active=is_any_active on_click=set_any flex_class="">
                    "Any"
                </FilterButton>
            </div>
        </div>
    }
}

#[derive(Clone, Copy)]
enum FilterActionKind {
    Apply,
    Save,
    Reset,
    Clear,
}

impl FilterActionKind {
    fn label(&self) -> &'static str {
        match self {
            FilterActionKind::Apply => "Apply",
            FilterActionKind::Save => "Save",
            FilterActionKind::Reset => "Reset",
            FilterActionKind::Clear => "Clear",
        }
    }

    fn tone_class(&self) -> &'static str {
        match self {
            FilterActionKind::Apply => "ui-button-primary",
            FilterActionKind::Save | FilterActionKind::Reset => "ui-button-secondary",
            FilterActionKind::Clear => "ui-button-danger",
        }
    }

    fn wide_class(&self) -> &'static str {
        match self {
            FilterActionKind::Apply => "xs:px-4",
            _ => "",
        }
    }
}

#[component]
fn ActionButton<F>(on_click: F, disabled: Signal<bool>, variant: FilterActionKind) -> impl IntoView
where
    F: Fn(leptos::ev::MouseEvent) + 'static,
{
    view! {
        <button
            on:click=on_click
            disabled=disabled
            class=move || {
                format!(
                    "ui-button {} ui-button-xs ui-fit-container min-w-16 flex-1 lg:flex-none {}",
                    variant.tone_class(),
                    variant.wide_class(),
                )
            }
        >
            <span class="ui-fit-label">{variant.label()}</span>
        </button>
    }
}
