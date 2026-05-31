use crate::{functions::telemetry::read_telemetry, providers::AuthContext};
use chrono::{DateTime, Duration, Utc};
use leptos::prelude::*;
use leptos_chartistry::*;
use leptos_use::use_interval_fn;
use shared_types::{TelemetryRange, TelemetryRow};

const SECTION_TITLE: &str = "py-2 text-lg font-bold dark:text-white";
const RANGE_BTN_BASE: &str = "px-3 py-1 mx-1 text-sm font-semibold rounded transition-colors";
const RANGE_BTN_ACTIVE: &str = "bg-button-dawn dark:bg-button-twilight text-white";
const RANGE_BTN_INACTIVE: &str =
    "bg-gray-200 dark:bg-gray-700 text-black dark:text-white hover:bg-gray-300";

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
        <div class="px-4 pt-20">
            <Show when=move || {
                auth_context.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin))
            }>
                <h1 class="pb-2 text-2xl font-bold dark:text-white">"WS Telemetry"</h1>
                <div class="flex flex-wrap gap-2 items-center pb-3">
                    <RangeButtons range=range />
                    <button
                        class="py-1 px-3 mx-2 text-sm font-semibold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
                        on:click=move |_| refresh_token.update(|n| *n = n.wrapping_add(1))
                    >
                        "Refresh"
                    </button>
                    <span class="text-xs text-gray-600 dark:text-gray-400">
                        "Auto-refresh every 30s"
                    </span>
                </div>
                <ChartStyles />
                <Suspense fallback=move || {
                    view! { <p class="dark:text-white">"Loading…"</p> }
                }>
                    {move || {
                        data.get()
                            .map(|res| match res {
                                Ok(rows) if rows.is_empty() => {
                                    view! {
                                        <p class="dark:text-white">
                                            "No telemetry rows in this range. Either the file is empty or the range is too short."
                                        </p>
                                    }
                                        .into_any()
                                }
                                Ok(rows) => view! { <AllPanels rows=rows /> }.into_any(),
                                Err(e) => {
                                    view! {
                                        <p class="text-red-500">
                                            "Telemetry error: " {format!("{e}")}
                                        </p>
                                    }
                                        .into_any()
                                }
                            })
                    }}
                </Suspense>
            </Show>
        </div>
    }
}

#[component]
fn RangeButtons(range: RwSignal<TelemetryRange>) -> impl IntoView {
    let btn = move |label: &'static str, value: TelemetryRange| {
        let class = move || {
            let active = range.get() == value;
            format!(
                "{RANGE_BTN_BASE} {}",
                if active {
                    RANGE_BTN_ACTIVE
                } else {
                    RANGE_BTN_INACTIVE
                }
            )
        };
        view! {
            <button class=class on:click=move |_| range.set(value)>
                {label}
            </button>
        }
    };
    view! {
        <div>
            {btn("Last hour", TelemetryRange::LastHour)} {btn("Last 24h", TelemetryRange::Last24h)}
            {btn("Last 7d", TelemetryRange::Last7d)} {btn("All", TelemetryRange::All)}
        </div>
    }
}

#[component]
fn AllPanels(rows: Vec<TelemetryRow>) -> impl IntoView {
    let (data, _) = signal(rows);
    view! {
        <div class="grid grid-cols-1 gap-4 xl:grid-cols-2">
            <Panel title="Connections (gauges)">
                <ChartConnections data=data />
            </Panel>
            <Panel title="Memory (MiB)">
                <ChartMemory data=data />
            </Panel>
            <Panel title="Drops — full (per interval)">
                <ChartDropsFull data=data />
            </Panel>
            <Panel title="Drops — closed (per interval)">
                <ChartDropsClosed data=data />
            </Panel>
            <Panel title="Queue health">
                <ChartQueue data=data />
            </Panel>
            <Panel title="Per-game activity (per interval)">
                <ChartActivity data=data />
            </Panel>
            <Panel title="Chat persistence (per interval)">
                <ChartChatPersistence data=data />
            </Panel>
            <Panel title="Sessions">
                <ChartSessions data=data />
            </Panel>
            <Panel title="Membership">
                <ChartMembership data=data />
            </Panel>
            <Panel title="Caches">
                <ChartCaches data=data />
            </Panel>
        </div>
    }
}

#[component]
fn Panel(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="p-3 bg-white rounded shadow dark:bg-gray-800 dark:[&_text]:!fill-gray-200">
            <h2 class=SECTION_TITLE>{title}</h2>
            {children()}
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
    let series = Series::new(ts)
        .line(tline!("sockets", active_sockets))
        .line(tline!("users", active_users))
        .line(tline!("games", active_games));
    base_chart(series, data)
}

#[component]
fn ChartMemory(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    const MIB: f64 = 1024.0 * 1024.0;
    let series = Series::new(ts)
        .line(tline_f!(
            "rss MiB",
            |r: &TelemetryRow| r.process_vm_rss_bytes as f64 / MIB
        ))
        .line(tline_f!(
            "hwm MiB",
            |r: &TelemetryRow| r.process_vm_hwm_bytes as f64 / MIB
        ));
    base_chart(series, data)
}

#[component]
fn ChartDropsFull(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("user", drops_full_user))
        .line(tline!("game", drops_full_game))
        .line(tline!("gamespec", drops_full_gamespec))
        .line(tline!("global", drops_full_global))
        .line(tline!("tour", drops_full_tour))
        .line(tline!("direct", drops_full_direct));
    base_chart(series, data)
}

#[component]
fn ChartDropsClosed(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("user", drops_closed_user))
        .line(tline!("game", drops_closed_game))
        .line(tline!("gamespec", drops_closed_gamespec))
        .line(tline!("global", drops_closed_global))
        .line(tline!("tour", drops_closed_tour))
        .line(tline!("direct", drops_closed_direct));
    base_chart(series, data)
}

#[component]
fn ChartQueue(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("max queue depth", max_queue_depth))
        .line(tline!("loader queued", load_user_state_queued))
        .line(tline!("loader in_flight", load_user_state_in_flight))
        .line(tline!("loader max", load_user_state_permit_max))
        .line(tline!("db pool max", db_pool_max_size))
        .line(tline!("own_state drops", own_state_drops));
    base_chart(series, data)
}

#[component]
fn ChartActivity(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("from_model", from_model_calls))
        .line(tline!("tv broadcasts", tv_broadcasts))
        .line(tline!("games finalized", games_finalized));
    base_chart(series, data)
}

#[component]
fn ChartChatPersistence(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("attempts", chat_persist_attempts))
        .line(tline!("successes", chat_persist_successes))
        .line(tline!("failures", chat_persist_failures))
        .line(tline!("normalizations", chat_message_normalizations));
    base_chart(series, data)
}

#[component]
fn ChartSessions(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("outer", sessions_outer))
        .line(tline!("inner total", sessions_inner_total));
    base_chart(series, data)
}

#[component]
fn ChartMembership(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("games→sockets", membership_games_sockets))
        .line(tline!("sockets→games", membership_sockets_games));
    base_chart(series, data)
}

#[component]
fn ChartCaches(data: ReadSignal<Vec<TelemetryRow>>) -> impl IntoView {
    let series = Series::new(ts)
        .line(tline!("game_response", game_response_cache))
        .line(tline!("last_tv", last_tv_broadcast))
        .line(tline!("lags trackers", lags_trackers))
        .line(tline!("game_start dates", game_start_games_date));
    base_chart(series, data)
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
