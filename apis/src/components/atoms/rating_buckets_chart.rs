use crate::components::layouts::base_layout::OrientationSignal;
use crate::responses::RatingBucketsResponse;
use leptos::prelude::*;
use leptos_chartistry::*;
use leptos_use::use_window_size;

#[component]
pub fn RatingBucketsChart(data: Signal<Vec<RatingBucketsResponse>>) -> impl IntoView {
    let window_size = use_window_size();
    let vertical = expect_context::<OrientationSignal>().orientation_vertical;
    let graph_width = Signal::derive(move || {
        let width = window_size.width.get() * if vertical.get() { 0.83 } else { 0.5 };
        width.min(750.0)
    });
    let graph_height = Signal::derive(move || window_size.height.get() * 0.3);

    view! {
        <div class="dark:[&_text]:!fill-gray-200">
            <Show
                when=move || !data.get().is_empty()
                fallback=|| {
                    view! {
                        <div class="text-center dark:text-gray-400">
                            "No data available"
                        </div>
                    }
                }
            >
                {move || {
                    let width = graph_width.get();
                    let height = graph_height.get();
                    if width == 0.0 || height == 0.0 {
                        return view! { <div>"Loading..."</div> }.into_any();
                    }
                    let aspect_ratio = AspectRatio::from_outer_ratio(width, height);
                    let max_number_of_players = data
                        .get()
                        .iter()
                        .map(|d| d.number_of_players)
                        .max()
                        .unwrap_or(1);
                    let series = Series::new(|data: &RatingBucketsResponse| data.bucket as f64)
                        .bar(|data: &RatingBucketsResponse| data.number_of_players as f64)
                        .with_y_range(0.0, max_number_of_players as f64);

                    view! {
                        <Chart
                            aspect_ratio=aspect_ratio
                            series=series
                            data=data
                            left=TickLabels::aligned_floats()
                            bottom=TickLabels::aligned_floats()
                            inner=[
                                AxisMarker::left_edge().into_inner(),
                                AxisMarker::bottom_edge().into_inner(),
                                YGridLine::default().into_inner(),
                            ]
                            tooltip=Tooltip::left_cursor()
                        />
                    }
                        .into_any()
                }}
            </Show>
        </div>
    }
}
