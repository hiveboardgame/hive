use crate::components::atoms::rating::icon_for_speed;
use crate::i18n::*;
use crate::pages::profile_view::tab_from_path;
use crate::providers::{load_games, FilterState, GamesSearchContext};
use hive_lib::Color;
use leptos::{html, prelude::*};
use leptos_i18n::I18nContext;
use leptos_icons::*;
use leptos_router::hooks::use_location;
use leptos_use::{on_click_outside_with_options, OnClickOutsideOptions};
use shared_types::{GameProgress, GameSpeed, ResultType};

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
        ctx.filters.set(new_filters);
        ctx.has_more.set_value(true);
        ctx.is_first_batch.set_value(true);
        let batch_size = ctx.initial_batch_size.get();
        load_games(
            ctx.filters.get(),
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
        let reset_to = if current_tab.get_untracked() == GameProgress::Finished {
            ctx.get_filter_cookie.get_untracked().unwrap_or_default()
        } else {
            FilterState::default()
        };
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
        let saved_default = ctx.get_filter_cookie.get().unwrap_or_default();
        if no_changes() {
            saved_default == ctx.filters.get()
        } else {
            saved_default == pending.get()
        }
    });
    let is_system_default = Signal::derive(move || ctx.filters.get() == FilterState::default());
    let reset_disabled = Signal::derive(move || {
        let current_filters = ctx.filters.get();
        let reset_target = if current_tab() == GameProgress::Finished {
            ctx.get_filter_cookie.get().unwrap_or_default()
        } else {
            FilterState::default()
        };
        current_filters == reset_target
    });

    view! {
        <div class="relative">
            <details node_ref=dropdown_ref>
                <summary class="px-2 py-1 ml-0.5 text-sm font-semibold text-gray-900 bg-gray-100 rounded-lg border-2 border-transparent cursor-pointer max-w-fit hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-100 dark:hover:bg-gray-700">
                    "🔧 Filters"
                </summary>
                <div class="absolute right-0 top-full z-40 p-2 mt-1 max-w-screen-sm bg-white rounded-lg border border-gray-200 shadow-lg w-[90vw] lg:p-0 lg:mt-2 lg:shadow-xl lg:w-96 lg:max-w-none dark:bg-gray-900 dark:border-gray-700">
                    <ActiveFiltersDisplay ctx current_tab i18n />

                    <div class="space-y-2 lg:space-y-0 lg:flex">
                        <div class="space-y-2 lg:flex-1 lg:p-4 lg:border-r lg:border-gray-200 lg:dark:border-gray-700">
                            <TriStateFilter
                                filter_type=TriStateType::Color
                                pending
                                grid_class="lg:grid-cols-3 lg:gap-2"
                            />

                            <Show when=move || current_tab() == GameProgress::Finished>
                                <div class="space-y-1 lg:space-y-2">
                                    <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 lg:text-sm">
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
                                <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 lg:text-sm">
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
                                                    flex_class="relative p-1 xs:p-1.5 sm:p-2 lg:p-2 hover:border-pillbug-teal min-h-8 xs:min-h-9 sm:min-h-10 lg:min-h-11 flex items-center justify-center"
                                                >
                                                    <Icon
                                                        icon=icon_for_speed(speed)
                                                        attr:class="w-4 h-4 xs:w-5 xs:h-5 lg:w-5 lg:h-5"
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
                                <span class="text-xs font-medium text-gray-700 dark:text-gray-300 lg:text-sm">
                                    "Bot Games:"
                                </span>
                                <FilterButton
                                    is_active=Signal::derive(move || {
                                        !pending.with(|state| state.exclude_bots)
                                    })
                                    on_click=move |_| {
                                        pending
                                            .update(|state| state.exclude_bots = !state.exclude_bots)
                                    }
                                    flex_class=""
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
                        <div class="px-3 py-2 mx-0 mb-0 text-sm text-yellow-800 bg-yellow-100 rounded-lg border border-yellow-300 dark:bg-yellow-900 dark:text-yellow-200 dark:border-yellow-700 lg:mx-4 lg:mb-4">
                            "Please select at least one game speed"
                        </div>
                    </Show>

                    <div class="flex gap-1 pt-2 mt-3 border-t border-gray-200 xs:gap-2 dark:border-gray-700 lg:gap-3 lg:justify-end lg:p-4 lg:bg-gray-50 lg:dark:bg-gray-800/30 lg:border-gray-200 lg:dark:border-gray-700">
                        <ActionButton
                            on_click=apply_filters
                            disabled=Signal::derive(move || no_changes() || no_speeds())
                            variant=ActionButtonVariant::Apply
                        />
                        <ActionButton
                            on_click=reset_filters
                            disabled=reset_disabled
                            variant=ActionButtonVariant::Reset
                        />
                        <Show when=move || current_tab() == GameProgress::Finished>
                            <ActionButton
                                on_click=set_default_filters
                                disabled=Signal::derive(move || {
                                    is_current_saved_default() || no_speeds()
                                })
                                variant=ActionButtonVariant::Save
                            />
                            <ActionButton
                                on_click=clear_default_filters
                                disabled=Signal::derive(move || {
                                    !saved_default_exists() && is_system_default()
                                })
                                variant=ActionButtonVariant::Clear
                            />
                        </Show>
                    </div>
                </div>
            </details>
        </div>
    }
}

fn button_style(is_active: bool) -> &'static str {
    if is_active {
        "border-pillbug-teal bg-pillbug-teal/10 text-pillbug-teal"
    } else {
        "border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800 text-gray-700 dark:text-gray-300"
    }
}

#[component]
fn ActiveFiltersDisplay(
    ctx: GamesSearchContext,
    current_tab: Signal<GameProgress>,
    i18n: I18nContext<Locale, I18nKeys>,
) -> impl IntoView {
    view! {
        <div class="pb-2 mb-2 border-b border-gray-200 dark:border-gray-700">
            <div class="flex flex-wrap gap-1 items-center min-h-[24px] p-2">
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
                        <Icon icon=icon_for_speed(speed) attr:class="w-3 h-3" />
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
                        <Icon icon=icondata_mdi::MdiRobotHappy attr:class="w-3 h-3" />
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
        <div class="flex gap-0.5 items-center px-2 py-0.5 text-xs rounded-full border bg-pillbug-teal/10 text-pillbug-teal border-pillbug-teal/20">
            {children()}
        </div>
    }
}

#[component]
fn FilterButton<F>(
    is_active: Signal<bool>,
    on_click: F,
    children: Children,
    #[prop(optional)] flex_class: &'static str,
) -> impl IntoView
where
    F: Fn(leptos::ev::MouseEvent) + 'static,
{
    view! {
        <button
            class=move || {
                format!(
                    "rounded-lg border-2 cursor-pointer transition-all font-medium text-center flex items-center justify-center px-1 py-1 text-xs min-h-6 xs:px-3 xs:py-1.5 xs:min-h-7 sm:px-4 sm:py-2 sm:min-h-9 lg:px-3 lg:py-1.5 lg:min-h-7 {} {}",
                    button_style(is_active()),
                    flex_class,
                )
            }
            on:click=on_click
        >
            {children()}
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
            <label class="block text-xs font-medium text-gray-700 dark:text-gray-300 lg:text-sm">
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
enum ActionButtonVariant {
    Apply,
    Save,
    Reset,
    Clear,
}

impl ActionButtonVariant {
    fn label(&self) -> &'static str {
        match self {
            ActionButtonVariant::Apply => "Apply",
            ActionButtonVariant::Save => "Save",
            ActionButtonVariant::Reset => "Reset",
            ActionButtonVariant::Clear => "Clear",
        }
    }

    fn enabled_classes(&self) -> &'static str {
        match self {
            ActionButtonVariant::Apply => "bg-pillbug-teal hover:bg-pillbug-teal/90 text-white",
            ActionButtonVariant::Save => "text-blue-700 bg-blue-100 hover:bg-blue-200 dark:bg-blue-900 dark:text-blue-200 dark:hover:bg-blue-800",
            ActionButtonVariant::Reset => "text-gray-700 bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-300",
            ActionButtonVariant::Clear => "text-red-700 bg-red-100 hover:bg-red-200 dark:bg-red-900 dark:text-red-200 dark:hover:bg-red-800",
        }
    }

    fn base_classes(&self) -> &'static str {
        match self {
            ActionButtonVariant::Apply => {
                "px-2 py-1 text-xs xs:px-4 xs:py-2 xs:text-sm font-semibold rounded-lg transition-all min-w-0"
            }
            _ => "px-2 py-1 text-xs xs:px-3 xs:py-2 xs:text-sm font-medium rounded-lg transition-all min-w-0",
        }
    }
}

#[component]
fn ActionButton<F>(
    on_click: F,
    disabled: Signal<bool>,
    variant: ActionButtonVariant,
) -> impl IntoView
where
    F: Fn(leptos::ev::MouseEvent) + 'static,
{
    view! {
        <button
            on:click=on_click
            disabled=disabled
            class=move || {
                format!(
                    "{} {}",
                    variant.base_classes(),
                    if disabled() {
                        "bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 cursor-not-allowed"
                    } else {
                        variant.enabled_classes()
                    },
                )
            }
        >
            {variant.label()}
        </button>
    }
}
