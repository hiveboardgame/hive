use crate::{
    components::layouts::base_layout::OrientationSignal,
    functions::games::get::get_rating_history_resource, responses::RatingHistoryResponse,
};
use chrono::{DateTime, Duration, Utc};
use leptos::prelude::*;
use leptos_chartistry::*;
use leptos_meta::Style;
use leptos_use::use_window_size;
use shared_types::GameSpeed;
use uuid::Uuid;

fn build_x_ticks(data: ReadSignal<Vec<RatingHistoryResponse>>) -> TickLabels<DateTime<Utc>> {
    let data_vec = data.get();
    if data_vec.is_empty() {
        return TickLabels::from_generator(Timestamps::<Utc>::from_period(Period::Year));
    }
    let now = Utc::now();
    let min_time = data_vec.first().map(|r| r.updated_at).unwrap_or(now);
    let max_time = data_vec.last().map(|r| r.updated_at).unwrap_or(now);
    let duration = max_time - min_time;
    let period = if duration < Duration::days(20) {
        Period::Day
    } else if duration < Duration::days(365) {
        Period::Month
    } else {
        Period::Year
    };
    TickLabels::from_generator(Timestamps::<Utc>::from_period(period))
}

#[component]
pub fn RatingGraph(user_id: Uuid, game_speed: GameSpeed) -> impl IntoView {
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;
    let history = OnceResource::new(get_rating_history_resource(user_id, game_speed));
    let window_size = use_window_size();
    let padding_right =
        Signal::derive(move || window_size.width.get() * if vertical.get() { 0.01 } else { 0.06 });
    let padding_left =
        Signal::derive(move || window_size.width.get() * if vertical.get() { 0.01 } else { 0.03 });
    let padding_bottom =
        Signal::derive(move || window_size.height.get() * if vertical.get() { 0.04 } else { 0.07 });
    let graph_width =
        Signal::derive(move || window_size.width.get() * if vertical.get() { 0.83 } else { 0.82 });
    let graph_height =
        Signal::derive(move || window_size.height.get() * 0.58 - padding_bottom.get());

    view! {
        <div
            class="w-full overflow-x-auto"
            style=move || {
                format!(
                    "padding-right: {}px; padding-left: {}px; padding-bottom: {}px;",
                    padding_right.get(),
                    padding_left.get(),
                    padding_bottom.get(),
                )
            }
        >
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || {
                    history
                        .get()
                        .map(|res| match res.as_ref() {
                            Ok(hist) => {
                                if hist.is_empty() {
                                    view! { <p>"No rating history yet."</p> }.into_any()
                                } else if hist.len() < 5 {
                                    view! {
                                        <div class="text-center text-gray-600 dark:text-yellow-400 py-8 text-lg">
                                            "Not enough games to build a graph 🐝🐝🐝"
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    let (data, _set_data) = signal(hist.clone());
                                    let x_ticks = build_x_ticks(data);
                                    let x_periods = Timestamps::from_periods(Period::all());
                                    let mut tooltip = Tooltip::left_cursor();
                                    tooltip.x_ticks = TickLabels::from_generator(
                                        x_periods.with_strftime("%Y-%m-%d"),
                                    );
                                    tooltip.y_ticks = TickLabels::aligned_floats()
                                        .with_format(|value, _| format!("{value:.0}"));
                                    view! {
                                        <Style>
                                            "
                                            ._chartistry_line {
                                             stroke: #155dfc;
                                            }
                                            ._chartistry_line_markers {
                                             stroke: #155dfc;
                                            }
                                            
                                            .dark ._chartistry_line {
                                             stroke: #fcc800;
                                            }
                                            .dark ._chartistry_line_markers {
                                             fill: #fcc800;
                                            }
                                            .dark ._chartistry_grid_line_x {
                                             stroke: #1a181845;
                                            }
                                            .dark ._chartistry_grid_line_y {
                                             stroke: #1a181845;
                                            }
                                            ._chartistry_tooltip {
                                             font-family: 'Inter', system-ui, sans-serif !important;
                                             font-size: 14px !important;
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
                                        </Style>
                                        <div class="dark:[&_text]:!fill-gray-200">
                                            <Chart
                                                aspect_ratio=AspectRatio::from_outer_ratio(
                                                    graph_width.get(),
                                                    graph_height.get(),
                                                )
                                                top=RotatedLabel::middle(
                                                    format!("{game_speed} Rating History"),
                                                )
                                                left=TickLabels::aligned_floats()
                                                bottom=x_ticks.clone()
                                                inner=[
                                                    AxisMarker::left_edge().into_inner(),
                                                    AxisMarker::bottom_edge().into_inner(),
                                                    XGridLine::from_ticks(x_ticks.clone()).into_inner(),
                                                    YGridLine::default().into_inner(),
                                                ]
                                                series=Series::new(|data: &RatingHistoryResponse| {
                                                        data.updated_at
                                                    })
                                                    .line(
                                                        Line::new(|data: &RatingHistoryResponse| data.rating as f64)
                                                            .with_interpolation(Interpolation::Linear)
                                                            .with_marker(
                                                                Marker::from_shape(MarkerShape::Circle).with_scale(0.6),
                                                            )
                                                            .with_width(2),
                                                    )
                                                tooltip=tooltip
                                                data=data
                                            />
                                        </div>
                                    }
                                        .into_any()
                                }
                            }
                            Err(e) => {

                                view! { <p>"Error loading history: " {format!("{e}")}</p> }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}
