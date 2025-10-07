use crate::responses::RatingHistoryResponse;
use chrono::DateTime;
use chrono::Utc;
use leptos::prelude::*;
use leptos_chartistry::*;
use server_fn::codec;
use shared_types::GameSpeed;
use std::sync::Arc;
use uuid::Uuid;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_rating_history_resource(
    user_id: Uuid,
    game_speed: GameSpeed,
) -> Result<RatingHistoryResponse, ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::get_conn;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    RatingHistoryResponse::from_uuid(&user_id, &game_speed, &mut conn)
        .await
        .map_err(ServerFnError::new)
}

#[derive(Clone)]
struct RatingPoint {
    datetime: DateTime<Utc>,
    rating: f64,
}

#[component]
pub fn RatingGraph(user_id: Uuid, game_speed: GameSpeed) -> impl IntoView {
    let history = OnceResource::new(get_rating_history_resource(user_id, game_speed));

    view! {
        <div class="rating-graph w-full overflow-x-auto">
            <Suspense fallback=move || {
                view! { <p>"Loading..."</p> }
            }>
                {move || {
                    history
                        .get()
                        .map(|res| match res.as_ref() {
                            Ok(hist) => {
                                let points: Vec<RatingPoint> = hist
                                    .data
                                    .iter()
                                    .map(|p| RatingPoint {
                                        datetime: p.updated_at,
                                        rating: p.rating as f64,
                                    })
                                    .collect();
                                if points.is_empty() {
                                    view! { <p>"No rating history yet."</p> }.into_any()
                                } else if points.len() < 5 {
                                    view! {
                                        <div class="text-center text-gray-600 py-8 text-lg">
                                            "Not enough games to build a graph 🐝🐝🐝"
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    let (data, _set_data) = signal(points);

                                    view! {
                                        <Chart
                                            aspect_ratio=AspectRatio::from_outer_ratio(400.0, 300.0)
                                            top=RotatedLabel::middle(
                                                format!("{} Rating History", game_speed),
                                            )
                                            left=TickLabels::aligned_floats()
                                            bottom=TickLabels::timestamps()
                                            inner=[
                                                AxisMarker::left_edge().into_inner(),
                                                AxisMarker::bottom_edge().into_inner(),
                                                XGridLine::default().into_inner(),
                                                YGridLine::default().into_inner(),
                                            ]
                                            series=Series::new(|data: &RatingPoint| data.datetime)
                                                .line(
                                                    Line::new(|data: &RatingPoint| data.rating)
                                                        .with_interpolation(Interpolation::Linear)
                                                        .with_marker(MarkerShape::Circle)
                                                        .with_name("Rating"),
                                                )

                                            data=data
                                        />
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
