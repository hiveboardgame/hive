use crate::{
    components::{
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::{empty_state::EmptyState, panel::Panel},
    },
    functions::telemetry::read_telemetry,
    i18n::{t, t_string, use_i18n},
    providers::AuthContext,
};
use chrono::{DateTime, Duration, Utc};
use leptos::prelude::*;
use leptos_chartistry::*;
use leptos_use::use_interval_fn;
use shared_types::{TelemetryRange, TelemetryRow};

fn ts(row: &TelemetryRow) -> DateTime<Utc> {
    DateTime::from_timestamp(row.timestamp as i64, 0).unwrap_or_else(Utc::now)
}

fn build_x_ticks(rows: &[TelemetryRow]) -> TickLabels<DateTime<Utc>> {
    if rows.len() < 2 {
        return TickLabels::from_generator(Timestamps::<Utc>::from_period(Period::Hour));
    }
    let first = ts(rows.first().unwrap());
    let last = ts(rows.last().unwrap());
    let duration = last - first;
    let period = if duration < Duration::hours(2) {
        Period::Minute
    } else if duration < Duration::days(2) {
        Period::Hour
    } else if duration < Duration::days(60) {
        Period::Day
    } else {
        Period::Month
    };
    TickLabels::from_generator(Timestamps::<Utc>::from_period(period))
}

#[component]
pub fn AdminTelemetry() -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let range = RwSignal::new(TelemetryRange::LastHour);
    let refresh_token = RwSignal::new(0u32);

    let data = Resource::new(
        move || (range.get(), refresh_token.get()),
        |(r, _)| read_telemetry(r),
    );

    use_interval_fn(
        move || refresh_token.update(|n| *n = n.wrapping_add(1)),
        30_000,
    );

    view! {
        <PageShell variant=PageShellVariant::Dashboard>
            <Show
                when=move || {
                    auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))
                }
                fallback=move || {
                    view! {
                        <EmptyState
                            title=move || {
                                t_string!(i18n, common.admin.telemetry.telemetry_unavailable)
                            }
                            message=move || t_string!(i18n, common.admin.administrators_only)
                        />
                    }
                }
            >
                <div class="flex flex-col gap-4 w-full max-w-7xl">
                    <PageHeader
                        title=move || t_string!(i18n, common.admin.telemetry.ws_telemetry)
                        subtitle=move || {
                            t_string!(
                                i18n, common.admin.telemetry.connection_queue_and_chat_health_over_time
                            )
                        }
                    />
                    <div class="flex flex-wrap gap-2 items-center">
                        <RangeButtons range=range />
                        <button
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| refresh_token.update(|n| *n = n.wrapping_add(1))
                        >
                            {t!(i18n, common.refresh)}
                        </button>
                        <span class="text-xs text-gray-600 dark:text-gray-400">
                            {t!(i18n, common.admin.telemetry.auto_refresh_every_30s)}
                        </span>
                    </div>
                    <ChartStyles />
                    <Suspense fallback=move || {
                        view! { <EmptyState title=move || t_string!(i18n, common.loading) /> }
                    }>
                        {move || {
                            data.get()
                                .map(|res| match res {
                                    Ok(rows) if rows.is_empty() => {
                                        view! {
                                            <EmptyState
                                                title=move || {
                                                    t_string!(i18n, common.admin.telemetry.no_telemetry_rows)
                                                }
                                                message=move || {
                                                    t_string!(
                                                        i18n, common.admin.telemetry.the_file_is_empty_or_the_selected_range_is
                                                    )
                                                }
                                            />
                                        }
                                            .into_any()
                                    }
                                    Ok(rows) => view! { <AllPanels rows=rows /> }.into_any(),
                                    Err(e) => {
                                        view! {
                                            <EmptyState
                                                title=move || {
                                                    t_string!(i18n, common.admin.telemetry.telemetry_error)
                                                }
                                                message=format!("{e}")
                                            />
                                        }
                                            .into_any()
                                    }
                                })
                        }}
                    </Suspense>
                </div>
            </Show>
        </PageShell>
    }
}

#[component]
fn RangeButtons(range: RwSignal<TelemetryRange>) -> impl IntoView {
    let i18n = use_i18n();
    let btn = move |label: TextProp, value: TelemetryRange| {
        let class = move || {
            let active = range.get() == value;
            if active {
                "ui-button ui-button-primary ui-button-sm"
            } else {
                "ui-button ui-button-secondary ui-button-sm"
            }
        };
        view! {
            <button class=class on:click=move |_| range.set(value)>
                {move || label.get()}
            </button>
        }
    };
    view! {
        <div>
            {btn(
                (move || t_string!(i18n, common.admin.telemetry.last_hour)).into(),
                TelemetryRange::LastHour,
            )}
            {btn(
                (move || t_string!(i18n, common.admin.telemetry.last_24h)).into(),
                TelemetryRange::Last24h,
            )}
            {btn(
                (move || t_string!(i18n, common.admin.telemetry.last_7d)).into(),
                TelemetryRange::Last7d,
            )} {btn((move || t_string!(i18n, common.all)).into(), TelemetryRange::All)}
        </div>
    }
}

#[component]
fn AllPanels(rows: Vec<TelemetryRow>) -> impl IntoView {
    let i18n = use_i18n();
    let (data, _) = signal(rows);
    view! {
        <div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.connections_gauges)
                body_class="overflow-x-auto"
            >
                <ChartConnections data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.memory_mib)
                body_class="overflow-x-auto"
            >
                <ChartMemory data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.drops_full_per_interval)
                body_class="overflow-x-auto"
            >
                <ChartDropsFull data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.drops_closed_per_interval)
                body_class="overflow-x-auto"
            >
                <ChartDropsClosed data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.queue_health)
                body_class="overflow-x-auto"
            >
                <ChartQueue data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.per_game_activity_per_interval)
                body_class="overflow-x-auto"
            >
                <ChartActivity data=data />
            </Panel>
            <Panel
                title=move || {
                    t_string!(i18n, common.admin.telemetry.persistent_chat_health_per_interval)
                }
                body_class="overflow-x-auto"
            >
                <ChartPersistentChatHealth data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.sessions)
                body_class="overflow-x-auto"
            >
                <ChartSessions data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.membership)
                body_class="overflow-x-auto"
            >
                <ChartMembership data=data />
            </Panel>
            <Panel
                title=move || t_string!(i18n, common.admin.telemetry.caches)
                body_class="overflow-x-auto"
            >
                <ChartCaches data=data />
            </Panel>
        </div>
    }
}

fn aspect() -> AspectRatio {
    AspectRatio::from_outer_ratio(720.0, 320.0)
}

macro_rules! tline {
    ($name:expr, $field:ident) => {
        Line::new(|r: &TelemetryRow| r.$field as f64)
            .with_name($name)
            .with_width(2.0)
    };
}

macro_rules! tline_f {
    ($name:expr, $expr:expr) => {
        Line::new($expr).with_name($name).with_width(2.0)
    };
}

fn base_chart(
    series: Series<TelemetryRow, DateTime<Utc>, f64>,
    data: ReadSignal<Vec<TelemetryRow>>,
) -> impl IntoView {
    let x_ticks = build_x_ticks(&data.get_untracked());
    let mut tooltip = Tooltip::left_cursor();
    tooltip.x_ticks = TickLabels::from_generator(
        Timestamps::from_periods(Period::all()).with_strftime("%Y-%m-%d %H:%M"),
    );
    tooltip.y_ticks = TickLabels::aligned_floats().with_format(|v, _| format!("{v:.0}"));
    view! {
        <Chart
            aspect_ratio=aspect()
            left=TickLabels::aligned_floats()
            bottom=x_ticks.clone()
            inner=[
                AxisMarker::left_edge().into_inner(),
                AxisMarker::bottom_edge().into_inner(),
                XGridLine::from_ticks(x_ticks.clone()).into_inner(),
                YGridLine::default().into_inner(),
            ]
            top=Legend::middle()
            series=series
            tooltip=tooltip
            data=data
        />
    }
}

#[component]
fn ChartConnections(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.sockets).to_string(),
                active_sockets
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.users).to_string(),
                active_users
            ))
            .line(tline!(
                t_string!(i18n, common.games).to_string(),
                active_games
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartMemory(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        const MIB: f64 = 1024.0 * 1024.0;
        let series = Series::new(ts)
            .line(tline_f!(
                t_string!(i18n, common.admin.telemetry.rss_mib).to_string(),
                |r: &TelemetryRow| r.process_vm_rss_bytes as f64 / MIB
            ))
            .line(tline_f!(
                t_string!(i18n, common.admin.telemetry.hwm_mib).to_string(),
                |r: &TelemetryRow| r.process_vm_hwm_bytes as f64 / MIB
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartDropsFull(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.user).to_string(),
                drops_full_user
            ))
            .line(tline!(
                t_string!(i18n, common.game).to_string(),
                drops_full_game
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.gamespec).to_string(),
                drops_full_gamespec
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.global).to_string(),
                drops_full_global
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.tour).to_string(),
                drops_full_tour
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.direct).to_string(),
                drops_full_direct
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartDropsClosed(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.user).to_string(),
                drops_closed_user
            ))
            .line(tline!(
                t_string!(i18n, common.game).to_string(),
                drops_closed_game
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.gamespec).to_string(),
                drops_closed_gamespec
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.global).to_string(),
                drops_closed_global
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.tour).to_string(),
                drops_closed_tour
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.direct).to_string(),
                drops_closed_direct
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartQueue(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.max_queue_depth).to_string(),
                max_queue_depth
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.loader_queued).to_string(),
                load_user_state_queued
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.loader_in_flight).to_string(),
                load_user_state_in_flight
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.loader_max).to_string(),
                load_user_state_permit_max
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.db_pool_max).to_string(),
                db_pool_max_size
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.own_state_drops).to_string(),
                own_state_drops
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartActivity(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.from_model).to_string(),
                from_model_calls
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.tv_broadcasts).to_string(),
                tv_broadcasts
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.games_finalized).to_string(),
                games_finalized
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartPersistentChatHealth(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.failures).to_string(),
                chat_persist_failures
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.rate_limit_rejections).to_string(),
                chat_rate_limit_rejections
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartSessions(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.outer).to_string(),
                sessions_outer
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.inner_total).to_string(),
                sessions_inner_total
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartMembership(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.games_sockets).to_string(),
                membership_games_sockets
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.sockets_games).to_string(),
                membership_sockets_games
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartCaches(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        let series = Series::new(ts)
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.game_response).to_string(),
                game_response_cache
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.last_tv).to_string(),
                last_tv_broadcast
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.lags_trackers).to_string(),
                lags_trackers
            ))
            .line(tline!(
                t_string!(i18n, common.admin.telemetry.game_start_dates).to_string(),
                game_start_games_date
            ));
        base_chart(series, data)
    }
}

#[component]
fn ChartStyles() -> impl IntoView {
    view! {
        <leptos_meta::Style>
            "
            ._chartistry_line_0 { stroke: #155dfc; }
            ._chartistry_line_1 { stroke: #16a34a; }
            ._chartistry_line_2 { stroke: #ea580c; }
            ._chartistry_line_3 { stroke: #9333ea; }
            ._chartistry_line_4 { stroke: #db2777; }
            ._chartistry_line_5 { stroke: #0891b2; }
            ._chartistry_line_markers { fill: currentColor; }
            .dark ._chartistry_y_axis text,
            .dark ._chartistry_x_axis text { fill: #e5e7eb; }
            .dark ._chartistry_grid_line_x,
            .dark ._chartistry_grid_line_y { stroke: #1a181845; }
            ._chartistry_tooltip {
                font-family: 'Inter', system-ui, sans-serif !important;
                font-size: 13px !important;
                color: #111 !important;
                background-color: #fff !important;
                border-radius: 6px !important;
            }
            .dark ._chartistry_tooltip {
                color: #f5f5f5 !important;
                background-color: #1a1a1a !important;
                border-color: #333 !important;
            }
            "
        </leptos_meta::Style>
    }
}
